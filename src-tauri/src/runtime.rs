//! Composition root do tempo de execucao: liga os adapters Win32 aos casos de uso de sessao e de
//! entrada (BE-14, BE-17 a BE-21) e sobe as threads (`input`, `system-events`, temporizador da
//! sessao). Sem regra de negocio. Roda no `setup`, depois de criar a janela.

use std::any::Any;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::Duration;

use launcher_core::application::ports::{
    Clock, GameResolver, InputGate, LauncherWindow, PlayedRecorder, ProbeConfig, SessionEvents,
    TaskRunner,
};
use launcher_core::application::{ForegroundBroker, SessionDeps, SessionService};
use launcher_core::domain::input::InputConfig;
use launcher_core::domain::session::SessionConfig;
use launcher_win::foreground::Win32WindowSystem;
use launcher_win::game_starter::WinGameStarter;
use launcher_win::input_thread::{InputControl, InputRunner, InputThread};
use launcher_win::process_probe::WinSessionProbe;
use launcher_win::system_clock::SystemClock;
use launcher_win::system_events::SystemEvents;
use launcher_win::xinput::XInputSource;
use tauri::{AppHandle, Manager, Runtime};

use crate::events::{TauriInputSink, TauriSessionEvents, TauriSystemEvents};
use crate::launcher_window::TauriLauncherWindow;
use crate::state::AppState;
use crate::window::MAIN_LABEL;

/// Trabalho fora da thread de quem chamou (a escada nunca roda na thread da janela).
struct ThreadRunner;

impl TaskRunner for ThreadRunner {
    fn run(&self, name: &'static str, task: Box<dyn FnOnce() + Send>) {
        if let Err(error) = std::thread::Builder::new().name(name.into()).spawn(task) {
            tracing::error!(tarefa = name, %error, "nao conseguiu criar a thread de trabalho");
        }
    }
}

/// Avanca os prazos da sessao (60 s, 5 s, 10 s). Dorme 1 s sem sessao e 250 ms com sessao.
struct TickThread {
    stop: Arc<AtomicBool>,
    handle: Option<JoinHandle<()>>,
}

impl TickThread {
    fn spawn(session: Arc<SessionService>) -> std::io::Result<Self> {
        let stop = Arc::new(AtomicBool::new(false));
        let flag = Arc::clone(&stop);
        let handle = std::thread::Builder::new()
            .name("session-tick".into())
            .spawn(move || {
                while !flag.load(Ordering::Acquire) {
                    let active = session.needs_tick();
                    if active {
                        session.tick();
                    }
                    std::thread::sleep(Duration::from_millis(if active { 250 } else { 1000 }));
                }
            })?;
        Ok(Self {
            stop,
            handle: Some(handle),
        })
    }
}

impl Drop for TickThread {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
        if let Some(h) = self.handle.take() {
            let _ = h.join();
        }
    }
}

/// Liga tudo e guarda no `AppState`. Falha numa peca nao derruba o launcher: o comando
/// correspondente responde `Internal` (estado neutro) e o motivo vai para o log.
pub fn start<R: Runtime>(app: &AppHandle<R>) -> Result<(), String> {
    let state = app.state::<AppState>();
    if state.user_data.get().is_none() {
        return Err("pasta de dados nao inicializada".into());
    }
    let advanced = state
        .settings
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .advanced()
        .settings;

    let hwnd = app
        .get_webview_window(MAIN_LABEL)
        .ok_or("janela main ausente")?
        .hwnd()
        .map_err(|e| format!("HWND da janela main: {e}"))?;
    let hwnd = hwnd.0 as isize;

    let clock: Arc<dyn Clock> = Arc::new(SystemClock::new());
    // A biblioteca e a mesma dos comandos (BE-29): resolve jogos manuais e de loja pela deteccao
    // atual e grava `lastPlayedAt`.
    let library = state
        .library
        .get()
        .cloned()
        .ok_or("biblioteca nao inicializada")?;
    let resolver: Arc<dyn GameResolver> = library.clone();
    let recorder: Arc<dyn PlayedRecorder> = library;

    let ws = Arc::new(Win32WindowSystem::new(hwnd));
    let foreground = Arc::new(ForegroundBroker::new(
        ws,
        Arc::clone(&clock),
        advanced.foreground.clone(),
    ));
    let control = Arc::new(InputControl::default());
    let window: Arc<dyn LauncherWindow> = Arc::new(TauriLauncherWindow::new(app.clone()));
    let events: Arc<dyn SessionEvents> = Arc::new(TauriSessionEvents::new(app.clone()));
    let input_gate: Arc<dyn InputGate> = control.clone();

    let session = SessionService::new(
        SessionDeps {
            clock: Arc::clone(&clock),
            resolver,
            recorder,
            starter: Arc::new(WinGameStarter),
            probe: Arc::new(WinSessionProbe::new()),
            foreground,
            window,
            input: input_gate,
            events,
            runner: Arc::new(ThreadRunner),
        },
        SessionConfig::from(&advanced.session),
        ProbeConfig {
            launching_interval_ms: advanced.session.probe_interval_ms,
            running_interval_ms: advanced.session.running_probe_interval_ms,
            use_steam_running_app_id: advanced.session.use_steam_running_app_id,
        },
    );

    let mut threads: Vec<Box<dyn Any + Send>> = Vec::new();

    let sink = Arc::new(TauriInputSink::new(app.clone(), Arc::clone(&session)));
    let runner = InputRunner::new(
        XInputSource::system(),
        InputConfig {
            deadzone_percent: u8::try_from(advanced.deadzone_percent).unwrap_or(25),
            ..InputConfig::default()
        },
        sink,
        Arc::clone(&control),
        Arc::clone(&clock),
    );
    match InputThread::spawn(runner) {
        Ok(t) => threads.push(Box::new(t)),
        Err(error) => tracing::error!(%error, "thread de entrada nao iniciou"),
    }
    // Sem o hook de primeiro plano (ARQ-BUG-05): `ForegroundChanged` nao tem consumidor no MVP e
    // o hook abria o processo da frente a cada troca de janela do sistema (superficie de SEC-28).
    // Lembrete para quem ligar de novo: `WINEVENT_SKIPOWNPROCESS` nao avisa quando o Sofa volta
    // para a frente.
    match SystemEvents::spawn(Arc::new(TauriSystemEvents::new(app.clone())), false) {
        Ok(t) => threads.push(Box::new(t)),
        Err(error) => tracing::error!(%error, "thread de eventos do sistema nao iniciou"),
    }
    match TickThread::spawn(Arc::clone(&session)) {
        Ok(t) => threads.push(Box::new(t)),
        Err(error) => tracing::error!(%error, "temporizador da sessao nao iniciou"),
    }

    let _ = state.session.set(session);
    let _ = state.input.set(control);
    let _ = state.threads.set(Mutex::new(threads));
    tracing::info!("sessao, entrada e eventos do sistema ligados");
    Ok(())
}
