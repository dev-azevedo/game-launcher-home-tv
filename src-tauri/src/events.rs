//! Eventos nucleo -> UI (contrato secao 3): nomes, payloads e os adapters que emitem. Carregam
//! estado completo, nunca deltas. Nenhuma regra de negocio.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

use launcher_core::application::ports::{
    DisplayEvents, InputSink, LibraryEvents, SessionEvents, VolumeEvents,
};
use launcher_core::application::{MonitorView, SessionService, SessionView, VolumeView};
use launcher_core::domain::input::IntentEvent;
use launcher_core::domain::library::GameId;
use launcher_core::domain::session::FrontOutcome;
use launcher_win::system_events::{SystemEvent, SystemEventSink};
use tauri::{AppHandle, Emitter, Manager, Runtime};

use crate::convert::monitor_dtos;
use crate::dto::{
    ControllersEvent, InputIntentEvent, LibrarySnapshot, SessionDto, SessionPanelEvent, VolumeState,
};
use crate::library_view::snapshot_dto;
use crate::state::AppState;

pub const INPUT_INTENT: &str = "input://intent";
pub const INPUT_CONTROLLERS: &str = "input://controllers";
pub const LIBRARY_CHANGED: &str = "library://changed";
pub const SESSION_STATE: &str = "session://state";
pub const SESSION_PANEL: &str = "session://panel";
pub const SYSTEM_RESUMED: &str = "system://resumed";
pub const VOLUME_CHANGED: &str = "volume://changed";
pub const DISPLAY_CHANGED: &str = "display://changed";

fn emit_logged<R: Runtime, P: serde::Serialize + Clone>(
    app: &AppHandle<R>,
    name: &str,
    payload: P,
) {
    if let Err(error) = app.emit(name, payload) {
        tracing::warn!(evento = name, %error, "evento nao emitido");
    }
}

/// `session://state`, `session://panel` e `library://changed` (fim de sessao, CA-30).
pub struct TauriSessionEvents<R: Runtime> {
    app: AppHandle<R>,
}

impl<R: Runtime> TauriSessionEvents<R> {
    pub fn new(app: AppHandle<R>) -> Self {
        Self { app }
    }
}

impl<R: Runtime> SessionEvents for TauriSessionEvents<R> {
    fn state_changed(&self, view: &SessionView) {
        let dto = SessionDto::from(view);
        tracing::info!(
            estado = ?view.snapshot.state,
            jogo = view.snapshot.game_id.as_ref().map(ToString::to_string),
            dispensada = view.snapshot.dismissed,
            "sessao: novo estado"
        );
        emit_logged(&self.app, SESSION_STATE, dto);
    }

    fn panel_opened(&self, outcome: FrontOutcome) {
        emit_logged(&self.app, SESSION_PANEL, SessionPanelEvent::opened(outcome));
    }

    fn game_ended(&self, focus: &GameId) {
        emit_logged(
            &self.app,
            LIBRARY_CHANGED,
            current_snapshot(&self.app, Some(focus)),
        );
    }
}

/// O `LibrarySnapshot` de agora (sem ocultos, ordem da RN-05). Sem biblioteca ligada (falha no
/// `setup`), a lista vai vazia.
pub fn current_snapshot<R: Runtime>(app: &AppHandle<R>, focus: Option<&GameId>) -> LibrarySnapshot {
    let state = app.state::<AppState>();
    match state.library.get() {
        Some(library) => snapshot_dto(&state, &library.snapshot(), focus),
        None => LibrarySnapshot {
            games: Vec::new(),
            focus_game_id: focus.map(ToString::to_string),
        },
    }
}

/// `library://changed` depois de varredura, renomear, ocultar e mostrar (contrato secao 3). O
/// foco fica por conta da UI (so o fim de sessao traz `focusGameId`, CA-30).
pub struct TauriLibraryEvents<R: Runtime> {
    app: AppHandle<R>,
}

impl<R: Runtime> TauriLibraryEvents<R> {
    pub fn new(app: AppHandle<R>) -> Self {
        Self { app }
    }
}

impl<R: Runtime> LibraryEvents for TauriLibraryEvents<R> {
    fn changed(&self) {
        emit_logged(
            &self.app,
            LIBRARY_CHANGED,
            current_snapshot(&self.app, None),
        );
    }
}

/// `input://intent`, `input://controllers` e a combinacao de retorno.
pub struct TauriInputSink<R: Runtime> {
    app: AppHandle<R>,
    session: Arc<SessionService>,
}

impl<R: Runtime> TauriInputSink<R> {
    pub fn new(app: AppHandle<R>, session: Arc<SessionService>) -> Self {
        Self { app, session }
    }
}

impl<R: Runtime> InputSink for TauriInputSink<R> {
    fn intent(&self, event: IntentEvent) {
        emit_logged(
            &self.app,
            INPUT_INTENT,
            InputIntentEvent {
                intent: event.intent.into(),
                repeat: event.repeat,
            },
        );
    }

    fn controllers(&self, connected: u8) {
        tracing::info!(conectados = connected, "controles conectados");
        emit_logged(&self.app, INPUT_CONTROLLERS, ControllersEvent { connected });
    }

    fn combo_fired(&self) {
        self.session.handle_return_combo();
    }
}

/// `volume://changed`: mudanca interna ou externa e troca de dispositivo (CA-41, CA-42).
pub struct TauriVolumeEvents<R: Runtime> {
    app: AppHandle<R>,
}

impl<R: Runtime> TauriVolumeEvents<R> {
    pub fn new(app: AppHandle<R>) -> Self {
        Self { app }
    }
}

impl<R: Runtime> VolumeEvents for TauriVolumeEvents<R> {
    fn changed(&self, view: &VolumeView) {
        emit_logged(&self.app, VOLUME_CHANGED, VolumeState::from(*view));
    }
}

/// `display://changed`: monitor ligado, desligado ou troca do monitor do launcher (CA-66, CA-67).
pub struct TauriDisplayEvents<R: Runtime> {
    app: AppHandle<R>,
}

impl<R: Runtime> TauriDisplayEvents<R> {
    pub fn new(app: AppHandle<R>) -> Self {
        Self { app }
    }
}

impl<R: Runtime> DisplayEvents for TauriDisplayEvents<R> {
    fn changed(&self, monitors: &[MonitorView]) {
        emit_logged(&self.app, DISPLAY_CHANGED, monitor_dtos(monitors));
    }
}

/// `system://resumed` e a reavaliacao dos monitores em `DisplayChanged` (BE-34, BE-19).
pub struct TauriSystemEvents<R: Runtime> {
    app: AppHandle<R>,
    /// Junta rajadas de `WM_DISPLAYCHANGE` numa so reavaliacao (um monitor ligado gera varias).
    display_burst: Arc<AtomicU64>,
}

/// Espera depois do ultimo `WM_DISPLAYCHANGE` antes de enumerar de novo.
const DISPLAY_SETTLE: Duration = Duration::from_millis(400);

impl<R: Runtime> TauriSystemEvents<R> {
    pub fn new(app: AppHandle<R>) -> Self {
        Self {
            app,
            display_burst: Arc::new(AtomicU64::new(0)),
        }
    }

    fn display_changed(&self) {
        let ticket = self.display_burst.fetch_add(1, Ordering::AcqRel) + 1;
        let burst = Arc::clone(&self.display_burst);
        let app = self.app.clone();
        let spawned = std::thread::Builder::new()
            .name("display-settle".into())
            .spawn(move || {
                std::thread::sleep(DISPLAY_SETTLE);
                if burst.load(Ordering::Acquire) != ticket {
                    return; // veio outro aviso depois: quem cuida e a thread dele
                }
                if let Some(display) = app.state::<AppState>().display.get() {
                    display.on_topology_changed();
                }
            });
        if let Err(error) = spawned {
            tracing::warn!(%error, "nao reavaliou os monitores");
        }
    }
}

impl<R: Runtime> SystemEventSink for TauriSystemEvents<R> {
    fn event(&self, event: SystemEvent) {
        match event {
            SystemEvent::Resumed => {
                tracing::info!("sistema voltou da suspensao");
                emit_logged(&self.app, SYSTEM_RESUMED, serde_json::json!({}));
            }
            SystemEvent::DisplayChanged => {
                tracing::info!("topologia de monitores mudou");
                self.display_changed();
            }
            SystemEvent::Suspending | SystemEvent::ForegroundChanged { .. } => {}
        }
    }
}
