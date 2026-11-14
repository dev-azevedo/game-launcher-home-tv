//! 2.5 Configuracoes, monitor e app (#25 a #30).

use tauri::ipc::Request;
use tauri::State;

use crate::dto::*;
use crate::ipc::{args, guard, no_args, not_wired};
use crate::state::AppState;

#[tauri::command]
pub async fn settings_get(request: Request<'_>) -> Result<SettingsDto, ErrorDto> {
    guard("settings_get", || {
        no_args(&request)?;
        Err(not_wired("settings_get"))
    })
}

#[tauri::command]
pub async fn settings_set_autostart(request: Request<'_>) -> Result<SettingsDto, ErrorDto> {
    guard("settings_set_autostart", || {
        let _a: SetAutostartArgs = args(&request)?;
        Err(not_wired("settings_set_autostart"))
    })
}

#[tauri::command]
pub async fn settings_set_monitor(request: Request<'_>) -> Result<SettingsDto, ErrorDto> {
    guard("settings_set_monitor", || {
        let _a: SetMonitorArgs = args(&request)?;
        Err(not_wired("settings_set_monitor"))
    })
}

#[tauri::command]
pub async fn display_monitors(request: Request<'_>) -> Result<Vec<MonitorDto>, ErrorDto> {
    guard("display_monitors", || {
        no_args(&request)?;
        Ok(Vec::new()) // DisplayTopology entra com BE-34
    })
}

#[tauri::command]
pub async fn input_status(request: Request<'_>) -> Result<InputStatus, ErrorDto> {
    guard("input_status", || {
        no_args(&request)?;
        Ok(InputStatus { connected: 0 }) // XInput entra com BE-14
    })
}

/// `app_exit`: fecha so o Sofa. Jogo em sessao continua aberto (CA-05): nenhum processo de jogo
/// e encerrado aqui e nao ha Job com `KILL_ON_JOB_CLOSE` (RN-16).
#[tauri::command]
pub async fn app_exit(request: Request<'_>, state: State<'_, AppState>) -> Result<(), ErrorDto> {
    guard("app_exit", || {
        no_args(&request)?;
        match state.exit_hook.get() {
            Some(exit) => {
                exit();
                Ok(())
            }
            None => Err(not_wired("app_exit")),
        }
    })
}
