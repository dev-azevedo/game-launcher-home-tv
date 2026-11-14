//! 2.2 Sessao (#10 a #18).

use tauri::ipc::Request;
use tauri::State;

use crate::dto::*;
use crate::ipc::{args, guard, no_args, not_wired};
use crate::state::AppState;

#[tauri::command]
pub async fn session_get(request: Request<'_>) -> Result<SessionDto, ErrorDto> {
    guard("session_get", || {
        no_args(&request)?;
        Ok(SessionDto::idle()) // a maquina de estados entra com BE-15 e BE-20
    })
}

#[tauri::command]
pub async fn session_launch(
    request: Request<'_>,
    state: State<'_, AppState>,
) -> Result<(), ErrorDto> {
    guard("session_launch", || {
        let _a: GameIdArgs = args(&request)?;
        let _permit = state.session_gate.try_enter()?;
        Err(not_wired("session_launch"))
    })
}

#[tauri::command]
pub async fn session_launch_with_uac(
    request: Request<'_>,
    state: State<'_, AppState>,
) -> Result<(), ErrorDto> {
    guard("session_launch_with_uac", || {
        let _a: GameIdArgs = args(&request)?;
        let _permit = state.session_gate.try_enter()?;
        Err(not_wired("session_launch_with_uac"))
    })
}

#[tauri::command]
pub async fn session_replace(
    request: Request<'_>,
    state: State<'_, AppState>,
) -> Result<(), ErrorDto> {
    guard("session_replace", || {
        let _a: GameIdArgs = args(&request)?;
        let _permit = state.session_gate.try_enter()?;
        Err(not_wired("session_replace"))
    })
}

#[tauri::command]
pub async fn session_return_to_game(request: Request<'_>) -> Result<(), ErrorDto> {
    guard("session_return_to_game", || {
        no_args(&request)?;
        Err(not_wired("session_return_to_game"))
    })
}

#[tauri::command]
pub async fn session_close(request: Request<'_>) -> Result<(), ErrorDto> {
    guard("session_close", || {
        no_args(&request)?;
        Err(not_wired("session_close"))
    })
}

#[tauri::command]
pub async fn session_force_close(request: Request<'_>) -> Result<(), ErrorDto> {
    guard("session_force_close", || {
        no_args(&request)?;
        Err(not_wired("session_force_close"))
    })
}

#[tauri::command]
pub async fn session_retry(
    request: Request<'_>,
    state: State<'_, AppState>,
) -> Result<(), ErrorDto> {
    guard("session_retry", || {
        no_args(&request)?;
        let _permit = state.session_gate.try_enter()?;
        Err(not_wired("session_retry"))
    })
}

#[tauri::command]
pub async fn session_dismiss(request: Request<'_>) -> Result<(), ErrorDto> {
    guard("session_dismiss", || {
        no_args(&request)?;
        Ok(()) // Idle: sem efeito, sem erro (idempotente); as demais transicoes entram com BE-20
    })
}
