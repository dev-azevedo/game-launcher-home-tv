//! 2.6 Atualizacao do app (#31, #32), SEC-33. Sem argumentos: a webview nao escolhe URL,
//! versao nem arquivo; o endpoint e a chave ficam na configuracao do Rust.

use launcher_core::application::AppError;
use tauri::ipc::Request;
use tauri::State;

use crate::dto::{ErrorDto, UpdateDto};
use crate::ipc::{blocking, guard, no_args};
use crate::state::AppState;

/// `update_check` (#31): versao nova publicada ou `null`. Sem internet: `UpdateFailed`.
#[tauri::command]
pub async fn update_check(
    request: Request<'_>,
    state: State<'_, AppState>,
) -> Result<Option<UpdateDto>, ErrorDto> {
    guard("update_check", || no_args(&request))?;
    let updater = state.updater.get().cloned();
    blocking("update_check", move || {
        updater.ok_or(AppError::UpdateFailed)?.check()
    })
    .await
}

/// `update_install` (#32): baixa, confere a assinatura e abre o instalador (UAC). Em sucesso o
/// launcher fecha e o instalador o reabre; a resposta so chega em erro (`UpdateFailed`,
/// `UpdateCancelled`, `InvalidState` com jogo em sessao, `Busy`).
#[tauri::command]
pub async fn update_install(
    request: Request<'_>,
    state: State<'_, AppState>,
) -> Result<(), ErrorDto> {
    guard("update_install", || no_args(&request))?;
    let updater = state.updater.get().cloned();
    blocking("update_install", move || {
        updater.ok_or(AppError::UpdateFailed)?.install()
    })
    .await
}
