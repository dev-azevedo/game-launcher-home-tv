//! Integracao do inicio, da sonda e do fim de sessao com "jogos" sinteticos reais (BE-16, BE-17,
//! BE-21): `CreateProcessW` + Job Object, varredura por pasta, `NeedsElevation` pelo erro 740,
//! forcar fechamento e PID reaproveitado. Os que precisam de desktop interativo (janelas,
//! primeiro plano) ficam `#[ignore]`: rode com `cargo test -p launcher-win --test session_runtime
//! -- --ignored --nocapture --test-threads=1`.

use std::path::{Path, PathBuf};
use std::process::{Child, Command};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::time::{Duration, Instant};

use launcher_core::application::ports::{
    GameStarter, ProbeConfig, ProbeSink, SessionProbe, SessionWatch, StartError, StartVerb,
    WatchPlan,
};
use launcher_core::application::AppError;
use launcher_core::domain::library::{GameId, LaunchSpec, LaunchTarget};
use launcher_win::game_starter::WinGameStarter;
use launcher_win::process_probe::{creation_time, terminate_if_same, WinSessionProbe};
use windows::core::PCWSTR;
use windows::Win32::Foundation::CloseHandle;
use windows::Win32::System::LibraryLoader::{
    BeginUpdateResourceW, EndUpdateResourceW, UpdateResourceW,
};
use windows::Win32::System::Threading::{OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION};

fn game_exe() -> &'static str {
    env!("CARGO_BIN_EXE_sofa-test-game")
}

static COUNTER: AtomicU32 = AtomicU32::new(0);

/// Pasta unica de teste, apagada ao fim.
struct Sandbox(PathBuf);

impl Sandbox {
    fn new() -> Self {
        let n = COUNTER.fetch_add(1, Ordering::SeqCst);
        let p = std::env::temp_dir().join(format!("sofa-it-{}-{n}", std::process::id()));
        std::fs::create_dir_all(&p).unwrap();
        Sandbox(p)
    }

    /// Copia o jogo sintetico para `rel` (ex.: `Jogos\A & B\game.exe`).
    fn game(&self, rel: &str) -> PathBuf {
        let p = self.0.join(rel);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::copy(game_exe(), &p).unwrap();
        p
    }
}

impl Drop for Sandbox {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn target(exe: &Path, args: &[&str]) -> LaunchTarget {
    let dir = exe.parent().unwrap().to_string_lossy().into_owned();
    LaunchTarget {
        game: GameId::parse("manual:9f3a1c2b7d4e5a60").unwrap(),
        spec: LaunchSpec::Exe {
            path: exe.to_string_lossy().into_owned(),
            args: args.iter().map(|s| (*s).to_owned()).collect(),
            working_dir: Some(dir.clone()),
        },
        install_dir: Some(dir),
    }
}

#[derive(Default)]
struct Sink {
    events: Mutex<Vec<(&'static str, Instant)>>,
    cv: Condvar,
}

impl ProbeSink for Sink {
    fn process_seen(&self, _g: u64) {
        self.events.lock().unwrap().push(("seen", Instant::now()));
        self.cv.notify_all();
    }
    fn processes_gone(&self, _g: u64) {
        self.events.lock().unwrap().push(("gone", Instant::now()));
        self.cv.notify_all();
    }
}

impl Sink {
    /// Espera o n-esimo evento `name` e devolve o instante dele.
    fn wait(&self, name: &str, nth: usize, timeout: Duration) -> Option<Instant> {
        let deadline = Instant::now() + timeout;
        let mut g = self.events.lock().unwrap();
        loop {
            let hits: Vec<_> = g.iter().filter(|(n, _)| *n == name).collect();
            if hits.len() >= nth {
                return Some(hits[nth - 1].1);
            }
            let left = deadline.checked_duration_since(Instant::now())?;
            g = self.cv.wait_timeout(g, left).unwrap().0;
        }
    }

    fn count(&self, name: &str) -> usize {
        self.events
            .lock()
            .unwrap()
            .iter()
            .filter(|(n, _)| *n == name)
            .count()
    }
}

fn cfg() -> ProbeConfig {
    ProbeConfig {
        launching_interval_ms: 1000,
        running_interval_ms: 2000,
        use_steam_running_app_id: false,
    }
}

fn watch_job(t: &LaunchTarget, sink: &Arc<Sink>) -> (Box<dyn SessionWatch>, Instant) {
    let started = WinGameStarter.start(t, StartVerb::Normal).expect("iniciar");
    let t0 = Instant::now();
    let plan = WatchPlan {
        game: t.game.clone(),
        job: started.job,
        // Pasta do jogo: o `classify` dos PIDs do Job precisa dela (SEC-BUG-01).
        scan_dir: t.install_dir.clone(),
        exact_exe: None,
        steam_app_id: None,
    };
    assert!(plan.job.is_some(), "exe iniciado pelo Sofa entra no Job");
    let s: Arc<dyn ProbeSink> = sink.clone();
    (
        WinSessionProbe::with_store_dirs(vec![])
            .watch(plan, 1, s, cfg())
            .unwrap(),
        t0,
    )
}

fn parent_of(pid: u32) -> Option<u32> {
    use windows::Win32::System::Diagnostics::ToolHelp::{
        CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W,
        TH32CS_SNAPPROCESS,
    };
    // SAFETY: snapshot fechado ao fim; `entry` com `dwSize` correto.
    unsafe {
        let snap = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0).ok()?;
        let mut entry = PROCESSENTRY32W {
            dwSize: std::mem::size_of::<PROCESSENTRY32W>() as u32,
            ..Default::default()
        };
        let mut ok = Process32FirstW(snap, &mut entry).is_ok();
        let mut found = None;
        while ok {
            if entry.th32ProcessID == pid {
                found = Some(entry.th32ParentProcessID);
                break;
            }
            ok = Process32NextW(snap, &mut entry).is_ok();
        }
        let _ = CloseHandle(snap);
        found
    }
}

// --------------------------------------------------------------------- BE-16: iniciar

#[test]
fn exe_em_pasta_com_e_comercial_e_aspas_inicia_o_arquivo_certo_e_o_pai_e_o_sofa() {
    let sb = Sandbox::new();
    let exe = sb.game(r"Jogos\A & B (x86)\game.exe");
    let dump = sb.0.join("args.txt");

    // 1) Argumentos chegam exatos (lista, escape do Windows), sem passar por shell.
    let t = target(
        &exe,
        &[
            "dump",
            &dump.to_string_lossy(),
            "um valor",
            "diz \"oi\"",
            r"C:\pasta com espaco\",
        ],
    );
    let sink = Arc::new(Sink::default());
    let (watch, _) = watch_job(&t, &sink);
    sink.wait("gone", 1, Duration::from_secs(5))
        .expect("o jogo de teste saiu");
    drop(watch);
    let written = std::fs::read_to_string(&dump).unwrap();
    assert_eq!(
        written.lines().collect::<Vec<_>>(),
        ["um valor", "diz \"oi\"", r"C:\pasta com espaco\"]
    );

    // 2) O processo e o do arquivo certo e o pai e o proprio Sofa (nunca cmd.exe/powershell.exe).
    let t = target(&exe, &["sleep", "1500"]);
    let sink = Arc::new(Sink::default());
    let (watch, _) = watch_job(&t, &sink);
    // `pids()` devolve os membros ja conhecidos (ARQ-BUG-14): vale depois do aviso de inicio, que
    // e quando o caso de uso o chama (`Running`).
    sink.wait("seen", 1, Duration::from_secs(3))
        .expect("inicio");
    let pids = watch.pids();
    assert_eq!(pids.len(), 1, "um processo no Job: {pids:?}");
    assert_eq!(
        parent_of(pids[0]),
        Some(std::process::id()),
        "pai = quem iniciou"
    );
    sink.wait("gone", 1, Duration::from_secs(5)).expect("fim");
}

#[test]
fn exe_com_require_administrator_devolve_needs_elevation_com_fingerprint() {
    let sb = Sandbox::new();
    let exe = sb.game(r"Admin Game\admin.exe");
    embed_admin_manifest(&exe);
    let t = target(&exe, &["sleep", "100"]);
    match WinGameStarter.start(&t, StartVerb::Normal) {
        Err(StartError::NeedsElevation { fingerprint }) => {
            assert!(
                fingerprint.is_some(),
                "o 740 guarda a identidade do arquivo (AF-01)"
            );
            assert_eq!(fingerprint, WinGameStarter.fingerprint(&t));
        }
        Ok(_) => panic!("iniciou sem UAC: o teste nao pode rodar elevado"),
        Err(e) => panic!("esperava NeedsElevation, veio {e:?}"),
    }
}

#[test]
fn fingerprint_muda_quando_o_exe_e_trocado() {
    let sb = Sandbox::new();
    let exe = sb.game(r"Jogo\game.exe");
    let t = target(&exe, &[]);
    let before = WinGameStarter.fingerprint(&t).unwrap();
    std::fs::write(&exe, b"MZ outro conteudo").unwrap();
    assert_ne!(WinGameStarter.fingerprint(&t).unwrap(), before);
}

#[test]
fn arquivo_ausente_e_caminho_de_rede_nao_iniciam_nada() {
    let sb = Sandbox::new();
    let missing = sb.0.join("nao-existe.exe");
    let t = target(&missing, &[]);
    assert!(matches!(
        WinGameStarter.start(&t, StartVerb::Normal),
        Err(StartError::FileMissing)
    ));
    let net = target(Path::new(r"\\127.0.0.1\c$\Windows\notepad.exe"), &[]);
    assert!(matches!(
        WinGameStarter.start(&net, StartVerb::Normal),
        Err(StartError::NetworkPath)
    ));
    // `.bat` editado a mao em library.json nunca inicia.
    let bat = sb.0.join("script.bat");
    std::fs::write(&bat, "@echo off").unwrap();
    assert!(matches!(
        WinGameStarter.start(&target(&bat, &[]), StartVerb::Normal),
        Err(StartError::FileMissing)
    ));
}

fn embed_admin_manifest(exe: &Path) {
    let xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><assembly xmlns="urn:schemas-microsoft-com:asm.v1" manifestVersion="1.0"><trustInfo xmlns="urn:schemas-microsoft-com:asm.v3"><security><requestedPrivileges><requestedExecutionLevel level="requireAdministrator" uiAccess="false"/></requestedPrivileges></security></trustInfo></assembly>"#;
    let path: Vec<u16> = exe
        .to_string_lossy()
        .encode_utf16()
        .chain(std::iter::once(0))
        .collect();
    // SAFETY: caminho NUL-terminado; recurso RT_MANIFEST (24) id 1 gravado numa copia do exe.
    unsafe {
        let h = BeginUpdateResourceW(PCWSTR(path.as_ptr()), false).expect("BeginUpdateResource");
        UpdateResourceW(
            h,
            PCWSTR(24 as *const u16),
            PCWSTR(1 as *const u16),
            0,
            Some(xml.as_ptr().cast()),
            xml.len() as u32,
        )
        .expect("UpdateResource");
        EndUpdateResourceW(h, false).expect("EndUpdateResource");
    }
}

// ----------------------------------------------------------------------- BE-17: sonda

#[test]
fn job_avisa_inicio_e_fim_do_ultimo_processo_sem_polling() {
    let sb = Sandbox::new();
    let exe = sb.game(r"Jogos\Jogo Um\game.exe");
    let t = target(&exe, &["sleep", "1500"]);
    let sink = Arc::new(Sink::default());
    let (_watch, t0) = watch_job(&t, &sink);
    let seen = sink
        .wait("seen", 1, Duration::from_secs(3))
        .expect("inicio");
    assert!(
        seen.duration_since(t0) < Duration::from_secs(1),
        "inicio < 1 s"
    );
    let gone = sink.wait("gone", 1, Duration::from_secs(6)).expect("fim");
    let alive = gone.duration_since(t0);
    assert!(
        alive >= Duration::from_millis(1400) && alive < Duration::from_millis(2300),
        "fim ~ 1,5 s + ~ms, foi {alive:?}"
    );
}

#[test]
fn job_inclui_o_filho_e_so_avisa_o_fim_quando_o_ultimo_sai() {
    let sb = Sandbox::new();
    let exe = sb.game(r"Jogos\Jogo Dois\game.exe");
    let child = sb.game(r"Jogos\Jogo Dois\bin\helper.exe");
    // Pai sai em 800 ms, filho vive 2500 ms.
    let t = target(&exe, &["spawn", &child.to_string_lossy(), "2500", "800"]);
    let sink = Arc::new(Sink::default());
    let (_watch, t0) = watch_job(&t, &sink);
    let gone = sink.wait("gone", 1, Duration::from_secs(8)).expect("fim");
    assert!(
        gone.duration_since(t0) >= Duration::from_millis(2400),
        "o pai saiu antes, mas o filho segue no Job: fim em {:?}",
        gone.duration_since(t0)
    );
}

/// `true` se existe processo vivo com esse nome de imagem.
fn alive_by_name(name: &str) -> bool {
    use windows::Win32::System::Diagnostics::ToolHelp::{
        CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W,
        TH32CS_SNAPPROCESS,
    };
    // SAFETY: snapshot fechado ao fim; `entry` com `dwSize` correto.
    unsafe {
        let snap = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0).unwrap();
        let mut entry = PROCESSENTRY32W {
            dwSize: std::mem::size_of::<PROCESSENTRY32W>() as u32,
            ..Default::default()
        };
        let mut found = false;
        let mut ok = Process32FirstW(snap, &mut entry).is_ok();
        while ok {
            let len = entry.szExeFile.iter().position(|c| *c == 0).unwrap();
            if String::from_utf16_lossy(&entry.szExeFile[..len]).eq_ignore_ascii_case(name) {
                found = true;
                break;
            }
            ok = Process32NextW(snap, &mut entry).is_ok();
        }
        let _ = CloseHandle(snap);
        found
    }
}

/// SEC-BUG-01: um `steam.exe` que nasce filho do jogo, dentro do Job, nao e membro: nao segura a
/// sessao aberta e nao e encerrado por "Forcar".
#[test]
fn job_com_filho_steam_exe_termina_sem_ele_e_forcar_nao_o_encerra() {
    let sb = Sandbox::new();
    let exe = sb.game(r"Jogos\Jogo Steam\game.exe");
    let steam = sb.game(r"Jogos\Jogo Steam\steam.exe");
    // Pai sai em 600 ms; o "steam.exe" filho vive 8 s (folga para antivirus e carga do CI).
    let t = target(&exe, &["spawn", &steam.to_string_lossy(), "8000", "600"]);
    let sink = Arc::new(Sink::default());
    let (watch, t0) = watch_job(&t, &sink);
    sink.wait("seen", 1, Duration::from_secs(3))
        .expect("inicio");
    let gone = sink
        .wait("gone", 1, Duration::from_secs(6))
        .expect("fim sem esperar o steam.exe");
    assert!(
        gone.duration_since(t0) < Duration::from_millis(6000),
        "o steam.exe nao e membro, mas a sessao so acabou em {:?}",
        gone.duration_since(t0)
    );
    watch.force_close().expect("nenhum membro: nada a encerrar");
    assert!(
        alive_by_name("steam.exe"),
        "Forcar nunca encerra o steam.exe"
    );
    // Deixa o filho acabar sozinho (8 s) para a pasta de teste poder ser apagada.
    let deadline = Instant::now() + Duration::from_secs(12);
    while alive_by_name("steam.exe") && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(100));
    }
}

/// SEC-BUG-01: processo do Job fora da pasta do jogo e acompanhante: nao e encerrado.
#[test]
fn forcar_no_job_so_encerra_membros_e_poupa_o_que_esta_fora_da_pasta() {
    let sb = Sandbox::new();
    let exe = sb.game(r"Jogos\Jogo Tres\game.exe");
    let outside = sb.game("Fora/acompanhante-sofa-qa.exe");
    let t = target(
        &exe,
        &["spawn", &outside.to_string_lossy(), "6000", "30000"],
    );
    let sink = Arc::new(Sink::default());
    let (watch, _) = watch_job(&t, &sink);
    sink.wait("seen", 1, Duration::from_secs(3))
        .expect("inicio");
    // O filho nasce (o antivirus pode demorar na primeira execucao do arquivo novo).
    let deadline = Instant::now() + Duration::from_secs(4);
    while !alive_by_name("acompanhante-sofa-qa.exe") {
        assert!(Instant::now() < deadline, "o acompanhante nao abriu");
        std::thread::sleep(Duration::from_millis(50));
    }
    watch.force_close().expect("encerra o membro");
    sink.wait("gone", 1, Duration::from_secs(3))
        .expect("fim apos forcar");
    assert!(
        alive_by_name("acompanhante-sofa-qa.exe"),
        "fora da pasta do jogo: nao e membro, nao e encerrado"
    );
}

/// ARQ-BUG-14: no modo Job nada tira snapshot ToolHelp. Nem a thread da sonda (rajada de processos
/// novos e saidas) nem `pids`, `request_close` e `force_close`, que o caso de uso chama com o
/// `Mutex` da sessao preso. O controle, a varredura por pasta, tira snapshots.
#[test]
fn modo_job_nao_tira_snapshot_do_sistema_e_pids_devolve_os_membros_conhecidos() {
    let sb = Sandbox::new();
    let exe = sb.game(r"Jogos\Jogo Job\game.exe");
    let filho = sb.game(r"Jogos\Jogo Job\filho.exe");
    // O jogo abre um filho que vive 800 ms; o pai sai em 1500 ms: ha mensagens de processo novo
    // e de saida ao longo da sessao.
    let t = target(&exe, &["spawn", &filho.to_string_lossy(), "800", "1500"]);
    let started = WinGameStarter
        .start(&t, StartVerb::Normal)
        .expect("iniciar");
    let plan = WatchPlan {
        game: t.game.clone(),
        job: started.job,
        scan_dir: t.install_dir.clone(),
        exact_exe: None,
        steam_app_id: None,
    };
    let sink = Arc::new(Sink::default());
    let s: Arc<dyn ProbeSink> = sink.clone();
    let watch = WinSessionProbe::with_store_dirs(vec![])
        .watch_win(plan, 1, s, cfg())
        .unwrap();
    sink.wait("seen", 1, Duration::from_secs(3))
        .expect("inicio");

    // Os membros conhecidos saem do cache, sem trabalho do sistema: o conjunto nao e vazio.
    let t0 = Instant::now();
    let pids = watch.pids();
    let took = t0.elapsed();
    assert!(!pids.is_empty(), "o jogo e membro conhecido");
    assert!(took < Duration::from_millis(15), "pids() barato: {took:?}");
    let _ = watch.request_close(); // sem janela: so confere que nao tira snapshot
    sink.wait("gone", 1, Duration::from_secs(6)).expect("fim");
    watch.force_close().expect("nenhum membro: nada a encerrar");
    assert_eq!(
        watch.snapshots_taken(),
        0,
        "modo Job nao chama CreateToolhelp32Snapshot"
    );
    // Deixa o filho acabar sozinho para a pasta de teste poder ser apagada.
    let deadline = Instant::now() + Duration::from_secs(8);
    while alive_by_name("filho.exe") && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(100));
    }

    // Controle: a varredura por pasta tira snapshots (o contador funciona).
    let plan = WatchPlan {
        game: GameId::parse("steam:620").unwrap(),
        job: None,
        scan_dir: t.install_dir.clone(),
        exact_exe: None,
        steam_app_id: None,
    };
    let s: Arc<dyn ProbeSink> = sink.clone();
    let scan = WinSessionProbe::with_store_dirs(vec![])
        .watch_win(plan, 1, s, cfg())
        .unwrap();
    std::thread::sleep(Duration::from_millis(300));
    assert!(scan.snapshots_taken() > 0, "a varredura usa snapshot");
}

/// SEC-BUG-04, modo Job: com a pasta reprovada em SEC-16 (aqui, sem `scan_dir`), so a imagem igual
/// ao exe do jogo e membro. Um processo do Job na mesma pasta do exe e acompanhante: "Forcar" nao o
/// encerra.
#[test]
fn job_com_exe_exato_poupa_o_processo_vizinho_na_mesma_pasta() {
    let sb = Sandbox::new();
    let exe = sb.game(r"Raiz\game.exe");
    let vizinho = sb.game(r"Raiz\vizinho-sofa-qa.exe");
    let t = target(
        &exe,
        &["spawn", &vizinho.to_string_lossy(), "20000", "60000"],
    );
    let started = WinGameStarter
        .start(&t, StartVerb::Normal)
        .expect("iniciar");
    let plan = WatchPlan {
        game: t.game.clone(),
        job: started.job,
        scan_dir: None,
        exact_exe: Some(exe.to_string_lossy().into_owned()),
        steam_app_id: None,
    };
    let sink = Arc::new(Sink::default());
    let s: Arc<dyn ProbeSink> = sink.clone();
    let watch = WinSessionProbe::with_store_dirs(vec![])
        .watch(plan, 1, s, cfg())
        .unwrap();
    sink.wait("seen", 1, Duration::from_secs(3))
        .expect("inicio");
    let deadline = Instant::now() + Duration::from_secs(4);
    while !alive_by_name("vizinho-sofa-qa.exe") {
        assert!(Instant::now() < deadline, "o vizinho nao abriu");
        std::thread::sleep(Duration::from_millis(50));
    }
    assert_eq!(watch.pids().len(), 1, "so o exe do jogo e membro");
    watch.force_close().expect("encerra so o membro");
    sink.wait("gone", 1, Duration::from_secs(3))
        .expect("fim apos forcar");
    assert!(
        alive_by_name("vizinho-sofa-qa.exe"),
        "outra imagem na mesma pasta nao e membro, nao e encerrada"
    );
    // Encerra o vizinho (nome unico de teste) para a pasta de teste poder ser apagada.
    let _ = Command::new("taskkill")
        .args(["/F", "/IM", "vizinho-sofa-qa.exe"])
        .output();
    let deadline = Instant::now() + Duration::from_secs(5);
    while alive_by_name("vizinho-sofa-qa.exe") && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(100));
    }
}

/// SEC-BUG-04, varredura (jogo manual elevado): outro processo do usuario na mesma pasta do exe
/// nao entra no conjunto e sobrevive ao "Forcar fechamento".
#[test]
fn varredura_com_exe_exato_nao_pega_outro_processo_da_mesma_pasta() {
    let sb = Sandbox::new();
    let game = sb.game(r"Raiz\game.exe");
    let outro = sb.game(r"Raiz\outro.exe");
    let sink = Arc::new(Sink::default());
    let plan = WatchPlan {
        game: GameId::parse("manual:9f3a1c2b7d4e5a60").unwrap(),
        job: None,
        scan_dir: None,
        exact_exe: Some(game.to_string_lossy().into_owned()),
        steam_app_id: None,
    };
    let s: Arc<dyn ProbeSink> = sink.clone();
    let watch = WinSessionProbe::with_store_dirs(vec![])
        .watch(plan, 1, s, cfg())
        .unwrap();

    let mut o = Command::new(&outro)
        .args(["sleep", "30000"])
        .spawn()
        .unwrap();
    std::thread::sleep(Duration::from_millis(2500));
    assert_eq!(sink.count("seen"), 0, "outro exe na mesma pasta nao entra");

    let mut g = Command::new(&game)
        .args(["sleep", "30000"])
        .spawn()
        .unwrap();
    sink.wait("seen", 1, Duration::from_secs(4))
        .expect("o exe exato entra");
    assert_eq!(watch.pids(), vec![g.id()]);
    watch.force_close().expect("encerra so o membro");
    assert_eq!(g.wait().unwrap().code(), Some(1));
    sink.wait("gone", 1, Duration::from_secs(4)).expect("fim");
    assert!(o.try_wait().unwrap().is_none(), "o outro processo vive");
    let _ = o.kill();
    let _ = o.wait();
}

#[test]
fn varredura_por_pasta_ignora_homonimo_fora_da_pasta_e_ve_o_fim() {
    let sb = Sandbox::new();
    let game = sb.game(r"Games\FakeGame\game.exe");
    let homonym = sb.game(r"Other\game.exe");
    let dir = game.parent().unwrap().to_string_lossy().into_owned();

    let sink = Arc::new(Sink::default());
    let plan = WatchPlan {
        game: GameId::parse("steam:620").unwrap(),
        job: None,
        scan_dir: Some(dir),
        exact_exe: None,
        steam_app_id: None,
    };
    let s: Arc<dyn ProbeSink> = sink.clone();
    let watch = WinSessionProbe::with_store_dirs(vec![])
        .watch(plan, 1, s, cfg())
        .unwrap();

    // O homonimo fora da pasta roda 3 s e nunca entra no conjunto (SEC-15, S4 do SP-02).
    let mut h = Command::new(&homonym)
        .args(["sleep", "3000"])
        .spawn()
        .unwrap();
    std::thread::sleep(Duration::from_millis(3500));
    assert_eq!(sink.count("seen"), 0, "homonimo fora da pasta nao entra");
    let _ = h.wait();

    // O jogo na pasta entra e, ao sair, o fim chega na hora (espera nos handles).
    let started = Instant::now();
    let mut g = Command::new(&game).args(["sleep", "2000"]).spawn().unwrap();
    sink.wait("seen", 1, Duration::from_secs(4))
        .expect("inicio por varredura");
    let _ = g.wait();
    let exited = Instant::now();
    let gone = sink.wait("gone", 1, Duration::from_secs(4)).expect("fim");
    let lag = gone.duration_since(exited);
    assert!(
        lag < Duration::from_millis(700),
        "fim imediato pela espera no handle: {lag:?}"
    );
    assert!(gone.duration_since(started) >= Duration::from_millis(1900));
    assert!(watch.pids().is_empty());
}

#[test]
fn varredura_exclui_processos_do_cliente_da_loja() {
    let sb = Sandbox::new();
    let game = sb.game(r"Lib\steamapps\common\Jogo\game.exe");
    let client = sb.game(r"Lib\bin\cef\helper.exe"); // dentro de uma pasta de cliente
    let dir = sb.0.join("Lib").to_string_lossy().into_owned();
    let store = sb.0.join(r"Lib\bin").to_string_lossy().into_owned();

    let sink = Arc::new(Sink::default());
    let plan = WatchPlan {
        game: GameId::parse("steam:620").unwrap(),
        job: None,
        scan_dir: Some(dir),
        exact_exe: None,
        steam_app_id: None,
    };
    let s: Arc<dyn ProbeSink> = sink.clone();
    let _watch = WinSessionProbe::with_store_dirs(vec![store])
        .watch(plan, 1, s, cfg())
        .unwrap();

    let mut c = Command::new(&client)
        .args(["sleep", "2500"])
        .spawn()
        .unwrap();
    std::thread::sleep(Duration::from_millis(2200));
    assert_eq!(
        sink.count("seen"),
        0,
        "processo do cliente da loja nunca conta (RN-10)"
    );
    let _ = c.wait();
    let mut g = Command::new(&game).args(["sleep", "1000"]).spawn().unwrap();
    sink.wait("seen", 1, Duration::from_secs(4))
        .expect("o jogo entra");
    let _ = g.wait();
}

#[test]
fn o_proprio_sofa_e_o_webview2_dele_nunca_entram_mas_a_steam_iniciada_por_ele_sim() {
    let sb = Sandbox::new();
    // A pasta vigiada contem: um "jogo" e um `msedgewebview2.exe` (filho do Sofa), os dois
    // iniciados por este processo (que faz o papel do Sofa).
    let game = sb.game(r"Games\Misto\game.exe");
    let webview = sb.game(r"Games\Misto\msedgewebview2.exe");
    let dir = game.parent().unwrap().to_string_lossy().into_owned();
    let sink = Arc::new(Sink::default());
    let plan = WatchPlan {
        game: GameId::parse("steam:620").unwrap(),
        job: None,
        scan_dir: Some(dir),
        exact_exe: None,
        steam_app_id: None,
    };
    let s: Arc<dyn ProbeSink> = sink.clone();
    let watch = WinSessionProbe::with_store_dirs(vec![])
        .watch(plan, 1, s, cfg())
        .unwrap();
    let mut wv = Command::new(&webview)
        .args(["sleep", "3500"])
        .spawn()
        .unwrap();
    std::thread::sleep(Duration::from_millis(2200));
    assert_eq!(
        sink.count("seen"),
        0,
        "o WebView2 do Sofa nunca conta (SEC-17)"
    );
    // Descendente do Sofa que nao e WebView2 (a Steam iniciada pelo Sofa e os jogos dela):
    // entra pela pasta de instalacao.
    let mut g = Command::new(&game).args(["sleep", "2500"]).spawn().unwrap();
    sink.wait("seen", 1, Duration::from_secs(4))
        .expect("o jogo entra");
    let pids = watch.pids();
    assert!(pids.contains(&g.id()), "{pids:?}");
    assert!(
        !pids.contains(&wv.id()) && !pids.contains(&std::process::id()),
        "{pids:?}"
    );
    let _ = g.wait();
    let _ = wv.wait();
}

// ----------------------------------------------------------------- BE-21: forcar fechar

#[test]
fn force_close_do_job_encerra_os_membros_e_a_sonda_avisa() {
    let sb = Sandbox::new();
    let exe = sb.game(r"Jogos\Teimoso\game.exe");
    let t = target(&exe, &["sleep", "30000"]);
    let sink = Arc::new(Sink::default());
    let (watch, _) = watch_job(&t, &sink);
    sink.wait("seen", 1, Duration::from_secs(3))
        .expect("inicio");
    assert_eq!(sink.count("gone"), 0);
    watch.force_close().expect("encerra os membros");
    sink.wait("gone", 1, Duration::from_secs(3))
        .expect("fim apos forcar");
}

#[test]
fn pid_reaproveitado_nao_e_encerrado_mas_o_mesmo_processo_e() {
    let mut child: Child = Command::new(game_exe())
        .args(["sleep", "30000"])
        .spawn()
        .unwrap();
    let pid = child.id();
    // SAFETY: handle aberto so para ler o tempo de criacao e fechado em seguida.
    let creation = unsafe {
        let h = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid).unwrap();
        let c = creation_time(h).unwrap();
        let _ = CloseHandle(h);
        c
    };
    // Tempo de criacao diferente = outro processo com o mesmo PID: nada acontece.
    assert_eq!(terminate_if_same(pid, creation + 10_000_000), Ok(false));
    assert!(
        child.try_wait().unwrap().is_none(),
        "o processo nao foi tocado"
    );
    // Mesmo processo: encerra.
    assert_eq!(terminate_if_same(pid, creation), Ok(true));
    let status = child.wait().unwrap();
    assert_eq!(status.code(), Some(1));
}

#[test]
fn force_close_por_varredura_encerra_o_processo_associado() {
    let sb = Sandbox::new();
    let game = sb.game(r"Games\Loja\game.exe");
    let dir = game.parent().unwrap().to_string_lossy().into_owned();
    let sink = Arc::new(Sink::default());
    let plan = WatchPlan {
        game: GameId::parse("epic:SampleGameA").unwrap(),
        job: None,
        scan_dir: Some(dir),
        exact_exe: None,
        steam_app_id: None,
    };
    let s: Arc<dyn ProbeSink> = sink.clone();
    let watch = WinSessionProbe::with_store_dirs(vec![])
        .watch(plan, 1, s, cfg())
        .unwrap();
    let mut g = Command::new(&game)
        .args(["sleep", "30000"])
        .spawn()
        .unwrap();
    sink.wait("seen", 1, Duration::from_secs(4))
        .expect("inicio");
    watch.force_close().expect("encerra o membro");
    let status = g.wait().unwrap();
    assert_eq!(status.code(), Some(1));
    sink.wait("gone", 1, Duration::from_secs(4)).expect("fim");
}

// ---------------------------------------------------- precisam de desktop interativo

#[test]
#[ignore = "janela real: precisa de desktop interativo"]
fn wm_close_fecha_jogo_normal_e_jogo_que_ignora_so_sai_com_force_close() {
    let sb = Sandbox::new();
    let exe = sb.game(r"Jogos\Janela\game.exe");

    // Jogo normal: WM_CLOSE fecha.
    let t = target(&exe, &["window", "30000"]);
    let sink = Arc::new(Sink::default());
    let (watch, _) = watch_job(&t, &sink);
    std::thread::sleep(Duration::from_millis(1000)); // janela aparece
    watch.request_close().expect("WM_CLOSE entregue");
    sink.wait("gone", 1, Duration::from_secs(3))
        .expect("o jogo fechou com WM_CLOSE");
    drop(watch);

    // Jogo que ignora WM_CLOSE: continua vivo ate force_close.
    let t = target(&exe, &["window", "30000", "ignore-close"]);
    let sink = Arc::new(Sink::default());
    let (watch, _) = watch_job(&t, &sink);
    std::thread::sleep(Duration::from_millis(1000));
    watch.request_close().expect("WM_CLOSE entregue");
    std::thread::sleep(Duration::from_secs(3));
    assert_eq!(
        sink.count("gone"),
        0,
        "ignorou o pedido; nada e encerrado sozinho"
    );
    watch.force_close().unwrap();
    sink.wait("gone", 1, Duration::from_secs(3))
        .expect("forcado");
}

/// CPU da varredura (meta < 0,1% da CPU total, ADR-0005): 10 s com 1 s/2 s como no produto.
#[test]
#[ignore = "mede CPU por ~10 s"]
fn cpu_da_varredura_em_sessao() {
    use windows::Win32::Foundation::FILETIME;
    use windows::Win32::System::Threading::{GetCurrentProcess, GetProcessTimes};
    fn cpu_s() -> f64 {
        let (mut c, mut e, mut k, mut u) = (
            FILETIME::default(),
            FILETIME::default(),
            FILETIME::default(),
            FILETIME::default(),
        );
        // SAFETY: ponteiros para locais validos.
        unsafe {
            let _ = GetProcessTimes(GetCurrentProcess(), &mut c, &mut e, &mut k, &mut u);
        }
        let f = |t: FILETIME| {
            ((u64::from(t.dwHighDateTime) << 32) | u64::from(t.dwLowDateTime)) as f64 / 1e7
        };
        f(k) + f(u)
    }
    let sb = Sandbox::new();
    let game = sb.game(r"Games\Cpu\game.exe");
    let dir = game.parent().unwrap().to_string_lossy().into_owned();
    let sink = Arc::new(Sink::default());
    let plan = WatchPlan {
        game: GameId::parse("steam:620").unwrap(),
        job: None,
        scan_dir: Some(dir),
        exact_exe: None,
        steam_app_id: None,
    };
    let s: Arc<dyn ProbeSink> = sink.clone();
    let _watch = WinSessionProbe::with_store_dirs(vec![])
        .watch(plan, 1, s, cfg())
        .unwrap();
    let mut g = Command::new(&game)
        .args(["sleep", "11000"])
        .spawn()
        .unwrap();
    sink.wait("seen", 1, Duration::from_secs(4))
        .expect("inicio");
    let (before, t0) = (cpu_s(), Instant::now());
    std::thread::sleep(Duration::from_secs(10));
    let pct = (cpu_s() - before) / t0.elapsed().as_secs_f64() * 100.0;
    let logical = std::thread::available_parallelism().map_or(1, usize::from) as f64;
    println!(
        "CPU da varredura em Running (2 s): {pct:.3}% de 1 nucleo = {:.4}% do total ({logical} logicos)",
        pct / logical
    );
    let _ = g.kill();
    let _ = g.wait();
    assert!(pct / logical < 0.1, "meta < 0,1% do total");
}

/// Reproduz o SP-01 com o `WindowSystem` do produto: o processo de teste (sem direito de primeiro
/// plano) leva a janela "Sofa" (um `sofa-test-game window`) a frente por cima de um "jogo".
/// Imprime degrau e ms de cada tentativa.
#[test]
#[ignore = "janela real e primeiro plano: precisa de desktop interativo"]
fn escada_real_traz_a_janela_do_sofa_para_a_frente() {
    use launcher_core::application::foreground::ForegroundBroker;
    use launcher_core::application::ports::{Clock, WindowSystem};
    use launcher_core::domain::session::FrontOutcome;
    use launcher_core::domain::settings::AdvancedSettings;
    use launcher_win::foreground::Win32WindowSystem;
    use launcher_win::system_clock::SystemClock;
    use launcher_win::windows_enum::top_level_windows;

    let _ = tracing_subscriber::fmt()
        .with_writer(std::io::stdout)
        .try_init();
    let sb = Sandbox::new();
    let sofa_exe = sb.game(r"Sofa\sofa.exe");
    let game_exe_path = sb.game(r"Jogo\game.exe");
    let mut sofa = Command::new(&sofa_exe)
        .args(["window", "60000"])
        .spawn()
        .unwrap();
    std::thread::sleep(Duration::from_millis(800));
    let mut game = Command::new(&game_exe_path)
        .args(["window", "60000"])
        .spawn()
        .unwrap(); // toma a frente
    std::thread::sleep(Duration::from_millis(1200));

    let hwnd = top_level_windows(&[sofa.id()])
        .first()
        .copied()
        .expect("janela do Sofa");
    let ws = Arc::new(Win32WindowSystem::new(hwnd));
    let clock: Arc<dyn Clock> = Arc::new(SystemClock::new());
    let broker = ForegroundBroker::new(ws.clone(), clock, AdvancedSettings::default().foreground);
    let started = Instant::now();
    let outcome = broker.bring_launcher_front(&[game.id()]);
    println!("resultado {outcome:?} em {:?}", started.elapsed());
    let front = ws.launcher_is_foreground();
    let _ = sofa.kill();
    let _ = game.kill();
    let _ = sofa.wait();
    let _ = game.wait();
    assert!(matches!(
        outcome,
        FrontOutcome::Front | FrontOutcome::FrontAfterMinimize
    ));
    assert!(front);
    assert!(started.elapsed() < Duration::from_secs(3));
    let _ = AppError::Internal; // mantem o import usado em todos os perfis de teste
}
