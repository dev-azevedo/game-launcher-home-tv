//! Janela `main`: tela cheia sem borda (CA-03). Criada em Rust (`windows: []` no
//! `tauri.conf.json`), para a UI nao ter nenhum `core:window:*` (SEC-04, SEC-05).

use tauri::{Manager, Runtime, WebviewUrl, WebviewWindowBuilder};

/// Rotulo unico da janela; e o que `capabilities/main.json` referencia em `windows`.
pub const MAIN_LABEL: &str = "main";

pub fn create_main_window<R: Runtime>(app: &impl Manager<R>) -> tauri::Result<()> {
    WebviewWindowBuilder::new(app, MAIN_LABEL, WebviewUrl::App("index.html".into()))
        .title("Sofa")
        .decorations(false)
        .fullscreen(true)
        .resizable(false)
        .build()?;
    Ok(())
}
