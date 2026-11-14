//! Instancia unica (BE-35, CA-02, SEC-19): so traz a janela existente para a frente.
//!
//! O plugin `tauri-plugin-single-instance` cria o mutex `{identifier}-sim` no namespace da sessao
//! (sem SID: desvio aceito no SG-03/SP-07, com a condicao abaixo). A segunda instancia manda
//! `cwd` + `argv` por `WM_COPYDATA` e **este callback os descarta por completo**: nenhum
//! argumento de linha de comando de uma segunda instancia vira acao (nem `--autostart`, que so a
//! instancia que inicia le, em `StartMode`). Sem esquema de URI proprio.
//!
//! Com jogo em tela (janela oculta e WebView2 suspenso), "trazer a janela" e a mesma volta da
//! combinacao de retorno (`handle_return_combo`: retoma o WebView2, roda a escada de primeiro
//! plano e devolve a entrada ao Sofa); so mostrar a janela deixaria a UI morta.

use launcher_core::domain::session::SessionState;
use tauri::plugin::TauriPlugin;
use tauri::{AppHandle, Manager, Runtime};

use crate::state::AppState;
use crate::window::MAIN_LABEL;

/// Plugin a registrar **primeiro** no `Builder` (o plugin pede isso para checar cedo).
pub fn plugin<R: Runtime>() -> TauriPlugin<R> {
    tauri_plugin_single_instance::init(|app, _argv, _cwd| bring_launcher_front(app))
}

/// Traz o Sofa para a frente sem ler nada da segunda instancia.
pub fn bring_launcher_front<R: Runtime>(app: &AppHandle<R>) {
    let Some(window) = app.get_webview_window(MAIN_LABEL) else {
        return;
    };
    let hidden = !window.is_visible().unwrap_or(true);
    if hidden {
        let state = app.state::<AppState>();
        if let Some(session) = state.session.get() {
            if matches!(
                session.get().snapshot.state,
                SessionState::Running
                    | SessionState::Ending
                    | SessionState::Closing
                    | SessionState::CloseOverdue
            ) {
                tracing::info!("segunda instancia: volta do jogo para o Sofa");
                session.handle_return_combo();
                return;
            }
        }
    }
    tracing::info!("segunda instancia: traz a janela do Sofa");
    let _ = window.unminimize();
    let _ = window.show();
    let _ = window.set_focus();
}
