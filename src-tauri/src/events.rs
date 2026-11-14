//! Eventos nucleo -> UI (contrato secao 3): nomes, payloads e os adapters que emitem. Carregam
//! estado completo, nunca deltas. Nenhuma regra de negocio.

use std::sync::Arc;

use launcher_core::application::ports::{InputSink, LibraryEvents, SessionEvents, VolumeEvents};
use launcher_core::application::{SessionService, SessionView, VolumeView};
use launcher_core::domain::input::IntentEvent;
use launcher_core::domain::library::GameId;
use launcher_core::domain::session::FrontOutcome;
use launcher_win::system_events::{SystemEvent, SystemEventSink};
use tauri::{AppHandle, Emitter, Manager, Runtime};

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

/// `system://resumed`; o resto (`DisplayChanged`) e do BE-34.
pub struct TauriSystemEvents<R: Runtime> {
    app: AppHandle<R>,
}

impl<R: Runtime> TauriSystemEvents<R> {
    pub fn new(app: AppHandle<R>) -> Self {
        Self { app }
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
            }
            SystemEvent::Suspending | SystemEvent::ForegroundChanged { .. } => {}
        }
    }
}
