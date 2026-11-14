//! 2.4 Energia (#22 a #24). Sem `power_execute` real ate o BE-33: nenhum ticket e emitido.

use launcher_core::application::AppError;
use tauri::ipc::Request;

use crate::dto::*;
use crate::ipc::{args, guard, no_args, not_wired};

#[tauri::command]
pub async fn power_capabilities(request: Request<'_>) -> Result<PowerCapabilities, ErrorDto> {
    guard("power_capabilities", || {
        no_args(&request)?;
        Ok(PowerCapabilities {
            shutdown: Availability::Unavailable,
            restart: Availability::Unavailable,
            suspend: Availability::Unavailable,
        })
    })
}

#[tauri::command]
pub async fn power_prepare(request: Request<'_>) -> Result<String, ErrorDto> {
    guard("power_prepare", || {
        let _a: PowerPrepareArgs = args(&request)?;
        Err(AppError::Unavailable)
    })
}

#[tauri::command]
pub async fn power_execute(request: Request<'_>) -> Result<(), ErrorDto> {
    guard("power_execute", || {
        let _a: PowerExecuteArgs = args(&request)?;
        Err(not_wired("power_execute"))
    })
}
