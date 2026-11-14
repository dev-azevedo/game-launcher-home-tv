//! 2.3 Volume (#19 a #21). O Core Audio responde por uma thread propria (BE-32); os comandos
//! esperam em thread de bloqueio, nunca na do runtime. Sem dispositivo ou sem o servico ligado:
//! `volume_get` devolve `available = false` e as acoes `NoAudioDevice`.

use std::sync::Arc;

use launcher_core::application::{AppError, VolumeService, VolumeView};
use tauri::ipc::Request;
use tauri::State;

use crate::dto::*;
use crate::ipc::{args, blocking, guard, no_args};
use crate::state::AppState;

fn service(state: &AppState) -> Option<Arc<VolumeService>> {
    state.volume.get().cloned()
}

#[tauri::command]
pub async fn volume_get(
    request: Request<'_>,
    state: State<'_, AppState>,
) -> Result<VolumeState, ErrorDto> {
    guard("volume_get", || no_args(&request))?;
    let svc = service(&state);
    blocking("volume_get", move || {
        Ok(VolumeState::from(
            svc.map_or(VolumeView::UNAVAILABLE, |s| s.get()),
        ))
    })
    .await
}

#[tauri::command]
pub async fn volume_step(
    request: Request<'_>,
    state: State<'_, AppState>,
) -> Result<VolumeState, ErrorDto> {
    let a: VolumeStepArgs = guard("volume_step", || args(&request))?;
    let svc = service(&state);
    blocking("volume_step", move || {
        let svc = svc.ok_or(AppError::NoAudioDevice)?;
        svc.step_volume(a.direction == VolumeDirection::Up)
            .map(VolumeState::from)
    })
    .await
}

#[tauri::command]
pub async fn volume_toggle_mute(
    request: Request<'_>,
    state: State<'_, AppState>,
) -> Result<VolumeState, ErrorDto> {
    guard("volume_toggle_mute", || no_args(&request))?;
    let svc = service(&state);
    blocking("volume_toggle_mute", move || {
        svc.ok_or(AppError::NoAudioDevice)?
            .toggle_mute()
            .map(VolumeState::from)
    })
    .await
}
