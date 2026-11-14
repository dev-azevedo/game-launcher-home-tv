// Sem console no release. A 1a instrucao e a trava de busca de DLL (SEC-23); a linha de comando e
// lida aqui, uma vez, antes de criar Tauri, WebView2 e instancia unica (SEC-19, SEC-31).
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use launcher_core::domain::startup::{parse_start_mode, StartMode};

fn main() {
    launcher_win::hardening::set_default_dll_directories();
    let args: Vec<String> = std::env::args_os()
        .skip(1)
        .map(|a| a.to_string_lossy().into_owned())
        .collect();
    match parse_start_mode(&args) {
        // BE-41 liga o modo de remocao de dados aqui. Ate la, e recusado: nada e apagado.
        StartMode::Purge | StartMode::PurgeRefused => std::process::exit(2),
        mode => sofa_lib::run(mode),
    }
}
