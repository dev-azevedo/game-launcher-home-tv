//! Composition root do Sofa: liga comandos, estado e janela. Sem regra de negocio (arquitetura
//! 2.3); casos de uso e adapters entram com as tarefas BE-xx seguintes.

pub mod commands;
pub mod dto;
pub mod errors;
pub mod ipc;
pub mod state;
pub mod window;

use tauri::{Builder, Manager, Runtime};

/// Registra estado e a lista fechada de comandos (`commands::list::COMMAND_NAMES`). Separado de
/// `run` para os testes usarem o runtime de teste do Tauri.
///
/// `generate_handler!` e `capabilities/main.json` sao comparados com a lista em
/// `tests/command_list.rs` (INV-02).
pub fn configure<R: Runtime>(builder: Builder<R>) -> Builder<R> {
    use commands::{library::*, power::*, session::*, settings::*, volume::*};
    builder
        .manage(state::AppState::default())
        .invoke_handler(tauri::generate_handler![
            library_get,
            library_refresh,
            library_add_manual,
            library_hidden,
            game_rename,
            game_restore_name,
            game_hide,
            game_unhide,
            fs_list_dir,
            session_get,
            session_launch,
            session_launch_with_uac,
            session_replace,
            session_return_to_game,
            session_close,
            session_force_close,
            session_retry,
            session_dismiss,
            volume_get,
            volume_step,
            volume_toggle_mute,
            power_capabilities,
            power_prepare,
            power_execute,
            settings_get,
            settings_set_autostart,
            settings_set_monitor,
            display_monitors,
            input_status,
            app_exit,
        ])
}

pub fn run() {
    configure(tauri::Builder::default())
        .setup(|app| {
            let handle = app.handle().clone();
            // Saida do app sem tocar em jogo algum (CA-05); o hook evita comando generico.
            let _ = app
                .state::<state::AppState>()
                .exit_hook
                .set(Box::new(move || handle.exit(0)));
            window::create_main_window(app)?;
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("falha ao iniciar o Sofa");
}
