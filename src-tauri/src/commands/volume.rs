//! 2.3 Volume (#19 a #21).

use tauri::ipc::Request;

use crate::dto::*;
use crate::ipc::{args, guard, no_args, not_wired};

#[tauri::command]
pub async fn volume_get(request: Request<'_>) -> Result<VolumeState, ErrorDto> {
    guard("volume_get", || {
        no_args(&request)?;
        // Sem adapter de Core Audio ainda (BE-32): "sem dispositivo", como o contrato descreve.
        Ok(VolumeState {
            level: 0,
            muted: false,
            available: false,
        })
    })
}

#[tauri::command]
pub async fn volume_step(request: Request<'_>) -> Result<VolumeState, ErrorDto> {
    guard("volume_step", || {
        let _a: VolumeStepArgs = args(&request)?;
        Err(not_wired("volume_step"))
    })
}

#[tauri::command]
pub async fn volume_toggle_mute(request: Request<'_>) -> Result<VolumeState, ErrorDto> {
    guard("volume_toggle_mute", || {
        no_args(&request)?;
        Err(not_wired("volume_toggle_mute"))
    })
}
