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
        // Modo de remocao dos dados (SEC-31): sem janela, sem WebView2, sem instancia unica e fora
        // da IPC. So o desinstalador o chama. `PurgeRefused` = argumento extra: nada e apagado.
        StartMode::Purge => std::process::exit(launcher_win::purge::run_main(false)),
        StartMode::PurgeRefused => std::process::exit(launcher_win::purge::run_main(true)),
        mode => sofa_lib::run(mode),
    }
}
