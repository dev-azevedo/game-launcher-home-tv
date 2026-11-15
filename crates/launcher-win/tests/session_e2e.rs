//! Sessao de ponta a ponta com os adapters reais (`WinGameStarter`, `WinSessionProbe`,
//! `Win32WindowSystem`) e o "jogo" sintetico (BE-20, BE-21): a janela do Sofa, o WebView2, a
//! entrada e os eventos para a UI sao fakes. Os que precisam de janela real ficam `#[ignore]`.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use launcher_core::application::foreground::ForegroundBroker;
use launcher_core::application::ports::{
    Clock, GameResolver, GameStarter, InputGate, LauncherWindow, PlayedRecorder, ProbeConfig,
    SessionEvents, StartError, StartVerb, Started, TaskRunner,
};
use launcher_core::application::{AppError, SessionDeps, SessionService, SessionView};
use launcher_core::domain::input::InputMode;
use launcher_core::domain::library::{GameId, InstallPathPolicy, LaunchSpec, LaunchTarget};
use launcher_core::domain::session::{FrontOutcome, SessionConfig, SessionState};
use launcher_core::domain::settings::AdvancedSettings;
use launcher_win::foreground::Win32WindowSystem;
use launcher_win::game_starter::WinGameStarter;
use launcher_win::process_probe::WinSessionProbe;
use launcher_win::system_clock::SystemClock;

fn game_exe() -> &'static str {
    env!("CARGO_BIN_EXE_sofa-test-game")
}

struct Dir(PathBuf);
impl Drop for Dir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

struct Resolver(HashMap<GameId, LaunchTarget>);
impl GameResolver for Resolver {
    fn resolve(&self, id: &GameId) -> Result<LaunchTarget, AppError> {
        self.0.get(id).cloned().ok_or(AppError::UnknownGame)
    }
}

#[derive(Default)]
struct Recorder(Mutex<Vec<GameId>>);
impl PlayedRecorder for Recorder {
    fn mark_played(&self, id: &GameId, _at: i64) {
        self.0.lock().unwrap().push(id.clone());
    }
}

/// Conta quantos processos o `GameStarter` real chegou a criar.
struct Counting(AtomicU32);
impl GameStarter for Counting {
    fn preflight(&self, t: &LaunchTarget) -> Result<(), StartError> {
        WinGameStarter.preflight(t)
    }
    fn start(&self, t: &LaunchTarget, v: StartVerb) -> Result<Started, StartError> {
        self.0.fetch_add(1, Ordering::SeqCst);
        WinGameStarter.start(t, v)
    }
    fn fingerprint(&self, t: &LaunchTarget) -> Option<String> {
        WinGameStarter.fingerprint(t)
    }
}

#[derive(Default)]
struct Window(Mutex<Vec<&'static str>>);
impl LauncherWindow for Window {
    fn hide_and_suspend(&self) {
        self.0.lock().unwrap().push("hide");
    }
    fn show_and_resume(&self) {
        self.0.lock().unwrap().push("show");
    }
}

#[derive(Default)]
struct Gate(Mutex<Vec<InputMode>>);
impl InputGate for Gate {
    fn set_mode(&self, m: InputMode) {
        self.0.lock().unwrap().push(m);
    }
}

#[derive(Default)]
struct Events {
    states: Mutex<Vec<SessionState>>,
    ended: Mutex<Vec<GameId>>,
}
impl SessionEvents for Events {
    fn state_changed(&self, v: &SessionView) {
        self.states.lock().unwrap().push(v.snapshot.state);
    }
    fn panel_opened(&self, _o: FrontOutcome) {}
    fn game_ended(&self, f: &GameId) {
        self.ended.lock().unwrap().push(f.clone());
    }
}

struct Threads;
impl TaskRunner for Threads {
    fn run(&self, name: &'static str, task: Box<dyn FnOnce() + Send>) {
        std::thread::Builder::new()
            .name(name.into())
            .spawn(task)
            .unwrap();
    }
}

struct Rig {
    svc: Arc<SessionService>,
    starter: Arc<Counting>,
    recorder: Arc<Recorder>,
    window: Arc<Window>,
    gate: Arc<Gate>,
    events: Arc<Events>,
    stop: Arc<std::sync::atomic::AtomicBool>,
    _dir: Dir,
}

impl Drop for Rig {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
    }
}

fn id(n: u8) -> GameId {
    GameId::parse(&format!("manual:{n:016x}")).unwrap()
}

/// `games`: (numero do id, argumentos do jogo sintetico).
fn rig(games: &[(u8, &[&str])], cfg: SessionConfig) -> Rig {
    let dir = std::env::temp_dir().join(format!("sofa-e2e-{}-{}", std::process::id(), id_seed()));
    std::fs::create_dir_all(dir.join("Jogos")).unwrap();
    let exe = dir.join("Jogos").join("game.exe");
    std::fs::copy(game_exe(), &exe).unwrap();
    let dirs = dir.join("Jogos").to_string_lossy().into_owned();
    let mut map = HashMap::new();
    for (n, args) in games {
        map.insert(
            id(*n),
            LaunchTarget {
                game: id(*n),
                spec: LaunchSpec::Exe {
                    path: exe.to_string_lossy().into_owned(),
                    args: args.iter().map(|s| (*s).to_owned()).collect(),
                    working_dir: Some(dirs.clone()),
                },
                install_dir: Some(dirs.clone()),
            },
        );
    }
    let clock: Arc<dyn Clock> = Arc::new(SystemClock::new());
    let starter = Arc::new(Counting(AtomicU32::new(0)));
    let recorder = Arc::new(Recorder::default());
    let window = Arc::new(Window::default());
    let gate = Arc::new(Gate::default());
    let events = Arc::new(Events::default());
    let ws = Arc::new(Win32WindowSystem::new(0));
    let svc = SessionService::new(
        SessionDeps {
            clock: clock.clone(),
            resolver: Arc::new(Resolver(map)),
            recorder: recorder.clone(),
            starter: starter.clone(),
            probe: Arc::new(WinSessionProbe::with_store_dirs(vec![])),
            foreground: Arc::new(ForegroundBroker::new(
                ws,
                clock,
                AdvancedSettings::default().foreground,
            )),
            window: window.clone(),
            input: gate.clone(),
            events: events.clone(),
            runner: Arc::new(Threads),
            install_policy: InstallPathPolicy::new(r"C:\Windows", Vec::new()),
        },
        cfg,
        ProbeConfig {
            launching_interval_ms: 1000,
            running_interval_ms: 2000,
            use_steam_running_app_id: false,
        },
    );
    let stop = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let (s, flag) = (svc.clone(), stop.clone());
    std::thread::spawn(move || {
        while !flag.load(Ordering::SeqCst) {
            s.tick();
            std::thread::sleep(Duration::from_millis(100));
        }
    });
    Rig {
        svc,
        starter,
        recorder,
        window,
        gate,
        events,
        stop,
        _dir: Dir(dir),
    }
}

fn id_seed() -> u32 {
    static N: AtomicU32 = AtomicU32::new(0);
    N.fetch_add(1, Ordering::SeqCst)
}

fn wait_state(r: &Rig, want: SessionState, timeout: Duration) -> bool {
    let end = Instant::now() + timeout;
    while Instant::now() < end {
        if r.svc.get().snapshot.state == want {
            return true;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    false
}

fn cfg() -> SessionConfig {
    SessionConfig {
        start_timeout_ms: 60_000,
        end_grace_ms: 1_000,
        close_timeout_ms: 2_000,
    }
}

#[test]
fn sessao_completa_inicia_marca_jogado_esconde_e_ao_fim_traz_o_sofa_com_foco() {
    let r = rig(&[(1, &["sleep", "2500"])], cfg());
    r.svc.launch(id(1)).unwrap();
    assert!(
        wait_state(&r, SessionState::Running, Duration::from_secs(5)),
        "Running"
    );
    assert_eq!(
        *r.recorder.0.lock().unwrap(),
        vec![id(1)],
        "lastPlayedAt no inicio (P-06)"
    );
    assert_eq!(r.window.0.lock().unwrap().first(), Some(&"hide"));
    assert_eq!(
        r.gate.0.lock().unwrap().first(),
        Some(&InputMode::Background)
    );

    // O jogo sai: Ending -> (1 s de tolerancia) -> Idle, Sofa de volta com o jogo em foco.
    assert!(
        wait_state(&r, SessionState::Idle, Duration::from_secs(8)),
        "Idle"
    );
    let end = Instant::now() + Duration::from_secs(2);
    while r.events.ended.lock().unwrap().is_empty() && Instant::now() < end {
        std::thread::sleep(Duration::from_millis(20));
    }
    assert_eq!(*r.events.ended.lock().unwrap(), vec![id(1)]);
    assert_eq!(r.window.0.lock().unwrap().last(), Some(&"show"));
    assert_eq!(
        r.gate.0.lock().unwrap().last(),
        Some(&InputMode::Foreground)
    );
    let states = r.events.states.lock().unwrap().clone();
    assert_eq!(
        states,
        [
            SessionState::Launching,
            SessionState::Running,
            SessionState::Ending,
            SessionState::Idle
        ]
    );
}

#[test]
fn vinte_session_launch_seguidos_criam_no_maximo_um_processo() {
    let r = rig(&[(1, &["sleep", "2000"])], cfg());
    let ok = Arc::new(AtomicU32::new(0));
    let handles: Vec<_> = (0..20)
        .map(|_| {
            let (svc, ok) = (r.svc.clone(), ok.clone());
            std::thread::spawn(move || {
                if svc.launch(id(1)).is_ok() {
                    ok.fetch_add(1, Ordering::SeqCst);
                }
            })
        })
        .collect();
    for h in handles {
        h.join().unwrap();
    }
    // O contrato serializa por `SerialGate` na apresentacao; aqui o proprio estado impede o
    // segundo processo (`SessionAlreadyActive`).
    assert_eq!(ok.load(Ordering::SeqCst), 1);
    assert_eq!(
        r.starter.0.load(Ordering::SeqCst),
        1,
        "um unico processo criado"
    );
    assert!(wait_state(&r, SessionState::Idle, Duration::from_secs(8)));
}

#[test]
fn dismiss_em_launching_continua_e_o_jogo_que_abre_leva_a_running_com_mark_played() {
    let r = rig(&[(1, &["sleep", "2000"])], cfg());
    r.svc.launch(id(1)).unwrap();
    r.svc.dismiss(); // CA-77: so tira a UI da espera
    assert!(wait_state(
        &r,
        SessionState::Running,
        Duration::from_secs(5)
    ));
    assert_eq!(r.recorder.0.lock().unwrap().len(), 1);
}

#[test]
fn arquivo_ausente_nao_deixa_sessao_nem_processo() {
    let r = rig(&[(1, &["sleep", "100"])], cfg());
    // Id bem formado fora da biblioteca.
    assert_eq!(r.svc.launch(id(9)).unwrap_err(), AppError::UnknownGame);
    assert_eq!(r.svc.get().snapshot.state, SessionState::Idle);
    assert_eq!(r.starter.0.load(Ordering::SeqCst), 0);
}

#[test]
#[ignore = "janela real: precisa de desktop interativo"]
fn replace_com_jogo_que_ignora_wm_close_vai_a_close_overdue_e_so_o_force_close_encerra() {
    let r = rig(
        &[
            (1, &["window", "60000", "ignore-close"]),
            (2, &["sleep", "500"]),
        ],
        cfg(),
    );
    r.svc.launch(id(1)).unwrap();
    assert!(wait_state(
        &r,
        SessionState::Running,
        Duration::from_secs(5)
    ));
    std::thread::sleep(Duration::from_millis(1000)); // janela aparece

    r.svc.replace(id(2)).unwrap();
    assert!(
        wait_state(&r, SessionState::CloseOverdue, Duration::from_secs(5)),
        "o jogo ignorou WM_CLOSE: Closing -> CloseOverdue no prazo"
    );
    assert_eq!(
        r.starter.0.load(Ordering::SeqCst),
        1,
        "o novo jogo nao abriu e nada foi encerrado"
    );
    assert_eq!(
        r.svc.replace(id(2)).unwrap_err(),
        AppError::InvalidState,
        "replace em CloseOverdue"
    );
    // "Forcar" confirmado encerra; o pendente foi descartado (CT-01).
    r.svc.force_close().unwrap();
    assert!(wait_state(&r, SessionState::Idle, Duration::from_secs(5)));
    assert_eq!(r.starter.0.load(Ordering::SeqCst), 1);
}

#[test]
#[ignore = "janela real: precisa de desktop interativo"]
fn fechar_pelo_painel_pede_wm_close_e_o_jogo_normal_fecha_sem_forcar() {
    let r = rig(&[(1, &["window", "60000"])], cfg());
    r.svc.launch(id(1)).unwrap();
    assert!(wait_state(
        &r,
        SessionState::Running,
        Duration::from_secs(5)
    ));
    std::thread::sleep(Duration::from_millis(1000));
    r.svc.close().unwrap();
    assert!(
        wait_state(&r, SessionState::Idle, Duration::from_secs(5)),
        "fechou com WM_CLOSE"
    );
    assert_eq!(r.svc.force_close().unwrap_err(), AppError::NoActiveSession);
}
