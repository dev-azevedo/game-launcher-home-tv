//! Composition root do Sofa: liga comandos, estado e janela. Sem regra de negocio (arquitetura
//! 2.3); casos de uso e adapters entram com as tarefas BE-xx seguintes.

pub mod bootstrap;
pub mod commands;
pub mod convert;
pub mod cover_protocol;
pub mod dto;
pub mod errors;
pub mod events;
pub mod exit;
pub mod ipc;
pub mod launcher_window;
pub mod library_view;
pub mod marks;
pub mod runtime;
pub mod single_instance;
pub mod state;
pub mod webview_hardening;
pub mod window;

use launcher_core::domain::startup::StartMode;
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
        // `cover://<gameId>` (SEC-08): fora da thread da janela, so GameId existente.
        .register_asynchronous_uri_scheme_protocol(
            cover_protocol::SCHEME,
            |ctx, request, responder| {
                let app = ctx.app_handle().clone();
                tauri::async_runtime::spawn_blocking(move || {
                    let state = app.state::<state::AppState>();
                    responder.respond(cover_protocol::respond(&state, &request));
                });
            },
        )
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

/// `start` vem da linha de comando deste processo (`StartMode`); modos de remocao nunca chegam
/// aqui (`main` os trata antes de criar o Tauri).
pub fn run(start: StartMode) {
    marks::start(start);
    // O plugin de instancia unica vai primeiro (SEC-19, BE-35).
    configure(tauri::Builder::default().plugin(single_instance::plugin()))
        .setup(|app| {
            bootstrap::init_user_data(app)?;
            tracing::info!(origem = marks::origin(), "partida");
            bootstrap::init_volume(app.handle());
            bootstrap::init_power(app.handle());
            bootstrap::init_display(app.handle());
            bootstrap::init_autostart(app.handle());
            if let Err(error) = bootstrap::init_library(app.handle()) {
                // Sem biblioteca o launcher ainda abre: leituras neutras, acoes `Internal`.
                tracing::error!(%error, "biblioteca nao iniciou");
            }
            let handle = app.handle().clone();
            // Saida do app sem tocar em jogo algum (CA-05); o hook evita comando generico.
            let _ = app
                .state::<state::AppState>()
                .exit_hook
                .set(Box::new(move || {
                    // CA-82, RN-35: so o launcher fecha; linha de log, flush e `exit(0)` (exit.rs).
                    exit::close_launcher(&exit::TauriExit {
                        handle: handle.clone(),
                    })
                }));
            let elevated = launcher_win::hardening::is_process_elevated();
            app.state::<state::AppState>()
                .process_elevated
                .store(elevated, std::sync::atomic::Ordering::Relaxed);
            if elevated {
                // SEC-20, RN-27: o Sofa nunca deve estar elevado. Registra e sinaliza; a UI
                // mostra o aviso discreto em Configuracoes quando o BE-34 expuser o campo.
                tracing::error!("Sofa em execucao elevada (TokenElevation); nao e esperado");
            }
            window::create_main_window(app)?;
            // Depois da janela existir: leva o launcher ao monitor da RN-20 (CA-66, CA-67).
            if let Some(display) = app.state::<state::AppState>().display.get() {
                display.start();
            }
            bootstrap::spawn_autostart_housekeeping(app.handle());
            if let Err(error) = runtime::start(app.handle()) {
                // Sem sessao/entrada o launcher ainda abre (biblioteca e menu); os comandos
                // dependentes respondem `Internal`.
                tracing::error!(%error, "runtime de sessao e entrada nao iniciou");
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("falha ao iniciar o Sofa");
}
