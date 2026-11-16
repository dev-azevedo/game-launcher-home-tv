//! Janela `main`: tela cheia sem borda (CA-03). Criada em Rust (`windows: []` no
//! `tauri.conf.json`), para a UI nao ter nenhum `core:window:*` (SEC-04, SEC-05).

use tauri::{Manager, Runtime, WebviewUrl, WebviewWindowBuilder};

use crate::webview_hardening;

/// Rotulo unico da janela; e o que `capabilities/main.json` referencia em `windows`.
pub const MAIN_LABEL: &str = "main";

/// Titulo da janela (Alt+Tab, Gerenciador de Tarefas): o nome visivel do produto (D-49, ADR-0007).
pub const MAIN_TITLE: &str = "SofaLauncher";

pub fn create_main_window<R: Runtime>(app: &impl Manager<R>) -> tauri::Result<()> {
    let builder = WebviewWindowBuilder::new(app, MAIN_LABEL, WebviewUrl::App("index.html".into()))
        .title(MAIN_TITLE)
        .decorations(false)
        .fullscreen(true)
        .resizable(false);
    let window = webview_hardening::lock_navigation(builder).build()?;
    webview_hardening::disable_browser_features(&window);
    Ok(())
}
