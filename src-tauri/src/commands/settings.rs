//! 2.5 Configuracoes, monitor e app (#25 a #30).

use tauri::ipc::Request;
use tauri::State;

use launcher_core::application::ports::SettingsStore;
use launcher_core::application::{AppError, AutostartStatus};

use crate::convert::monitor_dtos;
use crate::dto::*;
use crate::ipc::{args, blocking, guard, no_args, not_wired};
use crate::state::AppState;

/// `SettingsDto` de agora: monitor e pasta de logs vem das configuracoes; o autostart (estado
/// real do Windows, ADR-0006) vem de `status`, lido antes em thread de bloqueio.
fn dto_with(state: &AppState, status: AutostartStatus) -> Result<SettingsDto, AppError> {
    let log_dir = state
        .user_data
        .get()
        .map(|u| u.data_dir.join("logs").display().to_string())
        .unwrap_or_default();
    Ok(SettingsDto {
        autostart: status.enabled,
        autostart_mechanism: status.mechanism.into(),
        monitor_id: state.settings.get().and_then(|s| s.monitor_id()),
        log_dir,
    })
}

/// Estado do autostart; sem o servico (usuario nao identificado), desligado.
async fn autostart_status(state: &AppState) -> Result<AutostartStatus, ErrorDto> {
    let svc = state.autostart.get().cloned();
    blocking("settings_get", move || {
        Ok(svc.map_or(AutostartStatus::OFF, |s| s.status()))
    })
    .await
}

#[tauri::command]
pub async fn settings_get(
    request: Request<'_>,
    state: State<'_, AppState>,
) -> Result<SettingsDto, ErrorDto> {
    guard("settings_get", || no_args(&request))?;
    let status = autostart_status(&state).await?;
    guard("settings_get", || dto_with(&state, status))
}

/// `settings_set_autostart` (#26): liga (tarefa de logon, com fallback automatico para o Run) ou
/// desliga (remove os dois). Recusa ligar fora de `Program Files` (DR-04): `AutostartFailed`.
/// Nunca pede administrador (CA-74).
#[tauri::command]
pub async fn settings_set_autostart(
    request: Request<'_>,
    state: State<'_, AppState>,
) -> Result<SettingsDto, ErrorDto> {
    let a: SetAutostartArgs = guard("settings_set_autostart", || args(&request))?;
    let svc = state.autostart.get().cloned();
    let status = blocking("settings_set_autostart", move || {
        let svc = svc.ok_or(AppError::AutostartFailed)?;
        if a.enabled {
            svc.enable()
        } else {
            svc.disable()
        }
    })
    .await?;
    guard("settings_set_autostart", || dto_with(&state, status))
}

/// `settings_set_monitor` (#27): o id deve estar em `display_monitors` (`UnknownMonitor`); nulo
/// volta ao principal. Grava a preferencia e move o launcher na hora (CA-66).
#[tauri::command]
pub async fn settings_set_monitor(
    request: Request<'_>,
    state: State<'_, AppState>,
) -> Result<SettingsDto, ErrorDto> {
    let a: SetMonitorArgs = guard("settings_set_monitor", || args(&request))?;
    let display = state.display.get().cloned();
    blocking("settings_set_monitor", move || {
        display
            .ok_or_else(|| not_wired("settings_set_monitor"))?
            .set_monitor(a.monitor_id.0)
    })
    .await?;
    let status = autostart_status(&state).await?;
    guard("settings_set_monitor", || dto_with(&state, status))
}

#[tauri::command]
pub async fn display_monitors(
    request: Request<'_>,
    state: State<'_, AppState>,
) -> Result<Vec<MonitorDto>, ErrorDto> {
    guard("display_monitors", || no_args(&request))?;
    let display = state.display.get().cloned();
    blocking("display_monitors", move || {
        Ok(display.map_or_else(Vec::new, |d| monitor_dtos(&d.monitors())))
    })
    .await
}

#[tauri::command]
pub async fn input_status(
    request: Request<'_>,
    state: State<'_, AppState>,
) -> Result<InputStatus, ErrorDto> {
    guard("input_status", || {
        no_args(&request)?;
        Ok(InputStatus {
            connected: state.input.get().map_or(0, |c| c.connected()),
        })
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
