//! 2.2 Sessao (#10 a #18). Cada comando valida a entrada, chama o caso de uso de
//! `SessionService` e mapeia o erro (arquitetura 2.3). As acoes serializadas do contrato passam
//! pela `SerialGate` (`Busy`, SEC-06) antes de qualquer coisa. Sem o servico ligado (runtime que
//! nao subiu), as acoes respondem `Internal` e a leitura devolve o estado neutro.

use std::sync::Arc;

use launcher_core::application::{AppError, SessionService};
use launcher_core::domain::library::GameId;
use tauri::ipc::Request;
use tauri::State;

use crate::dto::*;
use crate::ipc::{args, blocking, guard, no_args, not_wired};
use crate::state::AppState;

fn service(state: &AppState, command: &'static str) -> Result<Arc<SessionService>, AppError> {
    state
        .session
        .get()
        .cloned()
        .ok_or_else(|| not_wired(command))
}

/// O `GameId` ja vem validado do DTO; o `parse` e a unica porta para o tipo do dominio.
fn game_id(a: &GameIdArgs) -> Result<GameId, AppError> {
    GameId::parse(&a.game_id.0).map_err(|_| AppError::InvalidRequest)
}

#[tauri::command]
pub async fn session_get(
    request: Request<'_>,
    state: State<'_, AppState>,
) -> Result<SessionDto, ErrorDto> {
    guard("session_get", || {
        no_args(&request)?;
        Ok(state
            .session
            .get()
            .map_or_else(SessionDto::idle, |s| SessionDto::from(&s.get())))
    })
}

#[tauri::command]
pub async fn session_launch(
    request: Request<'_>,
    state: State<'_, AppState>,
) -> Result<(), ErrorDto> {
    let id = guard("session_launch", || game_id(&args::<GameIdArgs>(&request)?))?;
    let gate = state.session_gate.clone();
    let svc = service(&state, "session_launch");
    blocking("session_launch", move || {
        let _permit = gate.try_enter()?;
        svc?.launch(id)
    })
    .await
}

#[tauri::command]
pub async fn session_launch_with_uac(
    request: Request<'_>,
    state: State<'_, AppState>,
) -> Result<(), ErrorDto> {
    let id = guard("session_launch_with_uac", || {
        game_id(&args::<GameIdArgs>(&request)?)
    })?;
    let gate = state.session_gate.clone();
    let svc = service(&state, "session_launch_with_uac");
    blocking("session_launch_with_uac", move || {
        let _permit = gate.try_enter()?;
        svc?.launch_with_uac(id)
    })
    .await
}

#[tauri::command]
pub async fn session_replace(
    request: Request<'_>,
    state: State<'_, AppState>,
) -> Result<(), ErrorDto> {
    let id = guard("session_replace", || {
        game_id(&args::<GameIdArgs>(&request)?)
    })?;
    let gate = state.session_gate.clone();
    let svc = service(&state, "session_replace");
    blocking("session_replace", move || {
        let _permit = gate.try_enter()?;
        svc?.replace(id)
    })
    .await
}

#[tauri::command]
pub async fn session_return_to_game(
    request: Request<'_>,
    state: State<'_, AppState>,
) -> Result<(), ErrorDto> {
    guard("session_return_to_game", || no_args(&request))?;
    let svc = service(&state, "session_return_to_game");
    blocking("session_return_to_game", move || svc?.return_to_game()).await
}

#[tauri::command]
pub async fn session_close(
    request: Request<'_>,
    state: State<'_, AppState>,
) -> Result<(), ErrorDto> {
    guard("session_close", || {
        no_args(&request)?;
        service(&state, "session_close")?.close()
    })
}

#[tauri::command]
pub async fn session_force_close(
    request: Request<'_>,
    state: State<'_, AppState>,
) -> Result<(), ErrorDto> {
    guard("session_force_close", || {
        no_args(&request)?;
        service(&state, "session_force_close")?.force_close()
    })
}

#[tauri::command]
pub async fn session_retry(
    request: Request<'_>,
    state: State<'_, AppState>,
) -> Result<(), ErrorDto> {
    guard("session_retry", || no_args(&request))?;
    let gate = state.session_gate.clone();
    let svc = service(&state, "session_retry");
    blocking("session_retry", move || {
        let _permit = gate.try_enter()?;
        svc?.retry()
    })
    .await
}

#[tauri::command]
pub async fn session_dismiss(
    request: Request<'_>,
    state: State<'_, AppState>,
) -> Result<(), ErrorDto> {
    guard("session_dismiss", || {
        no_args(&request)?;
        // Sem servico: Idle, sem efeito e sem erro (idempotente).
        if let Some(svc) = state.session.get() {
            svc.dismiss();
        }
        Ok(())
    })
}
