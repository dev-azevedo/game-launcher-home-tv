//! 2.1 Biblioteca (#1 a #9). Cada comando valida a entrada (`dto.rs`), chama o caso de uso de
//! `LibraryService` e mapeia o erro (arquitetura 2.3). Leituras nunca dao `Busy`; so
//! `library_refresh` passa pela `SerialGate` (contrato secao 0). Sem a biblioteca ligada (falha
//! no `setup`), as leituras devolvem o estado neutro e as acoes respondem `Internal`.

use std::sync::atomic::Ordering;
use std::sync::Arc;

use launcher_core::application::{AppError, LibraryService};
use launcher_core::domain::library::GameId;
use tauri::ipc::Request;
use tauri::State;

use crate::dto::*;
use crate::ipc::{args, blocking, guard, no_args, not_wired};
use crate::library_view::{game_dto_for, hidden_dto, snapshot_dto};
use crate::state::AppState;

fn service(state: &AppState, command: &'static str) -> Result<Arc<LibraryService>, AppError> {
    state
        .library
        .get()
        .cloned()
        .ok_or_else(|| not_wired(command))
}

/// O `GameId` ja vem validado do DTO; o `parse` e a unica porta para o tipo do dominio.
fn game_id(a: &GameIdArgs) -> Result<GameId, AppError> {
    GameId::parse(&a.game_id.0).map_err(|_| AppError::InvalidRequest)
}

/// Varredura em segundo plano, uma de cada vez (`Busy`); o resultado chega em
/// `library://changed`. A porta fica tomada ate a varredura terminar.
pub fn spawn_refresh(state: &AppState, svc: Arc<LibraryService>) -> Result<(), AppError> {
    let permit = state.library_gate.try_enter()?;
    let spawned = std::thread::Builder::new()
        .name("library-refresh".into())
        .spawn(move || {
            let _permit = permit;
            // `refresh` isola o panico de cada loja; este e o ultimo anteparo.
            if std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| svc.refresh())).is_err() {
                tracing::error!("panico na atualizacao da biblioteca");
            }
        });
    spawned.map(|_| ()).map_err(|error| {
        tracing::error!(%error, "nao conseguiu criar a thread de varredura");
        AppError::Internal
    })
}

#[tauri::command]
pub async fn library_get(
    request: Request<'_>,
    state: State<'_, AppState>,
) -> Result<LibrarySnapshot, ErrorDto> {
    guard("library_get", || {
        no_args(&request)?;
        let Some(svc) = state.library.get() else {
            return Ok(LibrarySnapshot::default());
        };
        let snapshot = svc.snapshot();
        let dto = snapshot_dto(&state, &snapshot, snapshot.focus());
        // Primeiro paint da UI: a lista do cache ja foi entregue; a varredura roda agora, em
        // segundo plano, e chega em `library://changed` (BE-40).
        if !state.initial_refresh.swap(true, Ordering::AcqRel) {
            if let Err(e) = spawn_refresh(&state, Arc::clone(svc)) {
                tracing::warn!(code = e.code(), "varredura inicial nao iniciou");
            }
        }
        Ok(dto)
    })
}

#[tauri::command]
pub async fn library_refresh(
    request: Request<'_>,
    state: State<'_, AppState>,
) -> Result<(), ErrorDto> {
    guard("library_refresh", || {
        no_args(&request)?;
        // `Busy` tem prioridade sobre `Internal`: a porta vem antes do servico.
        let permit = state.library_gate.try_enter()?;
        let svc = service(&state, "library_refresh")?;
        std::thread::Builder::new()
            .name("library-refresh".into())
            .spawn(move || {
                let _permit = permit;
                if std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| svc.refresh())).is_err()
                {
                    tracing::error!("panico na atualizacao da biblioteca");
                }
            })
            .map(|_| ())
            .map_err(|error| {
                tracing::error!(%error, "nao conseguiu criar a thread de varredura");
                AppError::Internal
            })
    })
}

#[tauri::command]
pub async fn library_add_manual(
    request: Request<'_>,
    state: State<'_, AppState>,
) -> Result<GameDto, ErrorDto> {
    let path = guard("library_add_manual", || {
        Ok(args::<AddManualArgs>(&request)?.path.0)
    })?;
    let svc = service(&state, "library_add_manual");
    let entry = blocking("library_add_manual", move || svc?.add_manual(&path)).await?;
    Ok(game_dto_for(&state, &entry))
}

#[tauri::command]
pub async fn library_hidden(
    request: Request<'_>,
    state: State<'_, AppState>,
) -> Result<Vec<GameDto>, ErrorDto> {
    guard("library_hidden", || {
        no_args(&request)?;
        Ok(state
            .library
            .get()
            .map_or_else(Vec::new, |svc| hidden_dto(&state, &svc.snapshot())))
    })
}

#[tauri::command]
pub async fn game_rename(
    request: Request<'_>,
    state: State<'_, AppState>,
) -> Result<GameDto, ErrorDto> {
    let (id, name) = guard("game_rename", || {
        let a: RenameArgs = args(&request)?;
        let id = GameId::parse(&a.game_id.0).map_err(|_| AppError::InvalidRequest)?;
        Ok((id, a.name.0))
    })?;
    let svc = service(&state, "game_rename");
    let entry = blocking("game_rename", move || svc?.rename(&id, &name)).await?;
    Ok(game_dto_for(&state, &entry))
}

#[tauri::command]
pub async fn game_restore_name(
    request: Request<'_>,
    state: State<'_, AppState>,
) -> Result<GameDto, ErrorDto> {
    let id = guard("game_restore_name", || {
        game_id(&args::<GameIdArgs>(&request)?)
    })?;
    let svc = service(&state, "game_restore_name");
    let entry = blocking("game_restore_name", move || svc?.restore_name(&id)).await?;
    Ok(game_dto_for(&state, &entry))
}

#[tauri::command]
pub async fn game_hide(request: Request<'_>, state: State<'_, AppState>) -> Result<(), ErrorDto> {
    let id = guard("game_hide", || game_id(&args::<GameIdArgs>(&request)?))?;
    let svc = service(&state, "game_hide");
    blocking("game_hide", move || svc?.hide(&id)).await
}

#[tauri::command]
pub async fn game_unhide(request: Request<'_>, state: State<'_, AppState>) -> Result<(), ErrorDto> {
    let id = guard("game_unhide", || game_id(&args::<GameIdArgs>(&request)?))?;
    let svc = service(&state, "game_unhide");
    blocking("game_unhide", move || svc?.unhide(&id)).await
}

#[tauri::command]
pub async fn fs_list_dir(
    request: Request<'_>,
    state: State<'_, AppState>,
) -> Result<DirListing, ErrorDto> {
    let path = guard("fs_list_dir", || Ok(args::<ListDirArgs>(&request)?.path.0))?;
    let svc = service(&state, "fs_list_dir");
    blocking("fs_list_dir", move || {
        Ok(DirListing::from(svc?.list_dir(path.as_deref())?))
    })
    .await
}
