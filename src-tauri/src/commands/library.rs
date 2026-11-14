//! 2.1 Biblioteca (#1 a #9).

use tauri::ipc::Request;
use tauri::State;

use crate::dto::*;
use crate::ipc::{args, guard, no_args, not_wired};
use crate::state::AppState;

#[tauri::command]
pub async fn library_get(
    request: Request<'_>,
    state: State<'_, AppState>,
) -> Result<LibrarySnapshot, ErrorDto> {
    guard("library_get", || {
        no_args(&request)?;
        // O cache real entra com BE-29; ate la, o que o estado guarda (vazio).
        Ok(state
            .library_snapshot
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone())
    })
}

#[tauri::command]
pub async fn library_refresh(
    request: Request<'_>,
    state: State<'_, AppState>,
) -> Result<(), ErrorDto> {
    guard("library_refresh", || {
        no_args(&request)?;
        let _permit = state.library_gate.try_enter()?;
        Err(not_wired("library_refresh"))
    })
}

#[tauri::command]
pub async fn library_add_manual(request: Request<'_>) -> Result<GameDto, ErrorDto> {
    guard("library_add_manual", || {
        let _a: AddManualArgs = args(&request)?;
        Err(not_wired("library_add_manual"))
    })
}

#[tauri::command]
pub async fn library_hidden(request: Request<'_>) -> Result<Vec<GameDto>, ErrorDto> {
    guard("library_hidden", || {
        no_args(&request)?;
        Ok(Vec::new())
    })
}

#[tauri::command]
pub async fn game_rename(request: Request<'_>) -> Result<GameDto, ErrorDto> {
    guard("game_rename", || {
        let _a: RenameArgs = args(&request)?;
        Err(not_wired("game_rename"))
    })
}

#[tauri::command]
pub async fn game_restore_name(request: Request<'_>) -> Result<GameDto, ErrorDto> {
    guard("game_restore_name", || {
        let _a: GameIdArgs = args(&request)?;
        Err(not_wired("game_restore_name"))
    })
}

#[tauri::command]
pub async fn game_hide(request: Request<'_>) -> Result<(), ErrorDto> {
    guard("game_hide", || {
        let _a: GameIdArgs = args(&request)?;
        Err(not_wired("game_hide"))
    })
}

#[tauri::command]
pub async fn game_unhide(request: Request<'_>) -> Result<(), ErrorDto> {
    guard("game_unhide", || {
        let _a: GameIdArgs = args(&request)?;
        Err(not_wired("game_unhide"))
    })
}

#[tauri::command]
pub async fn fs_list_dir(request: Request<'_>) -> Result<DirListing, ErrorDto> {
    guard("fs_list_dir", || {
        let _a: ListDirArgs = args(&request)?;
        Err(not_wired("fs_list_dir"))
    })
}
