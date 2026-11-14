//! 2.4 Energia (#22 a #24). `power_prepare` emite o ticket (30 s, uso unico, preso a acao) e
//! `power_execute` so age com ele (SEC-27, CT-07). Sem o servico ligado nada fica disponivel:
//! `power_prepare` responde `Unavailable` e `power_execute` responde `Internal`.

use std::sync::Arc;

use launcher_core::application::{AppError, PowerCapabilities as CoreCaps, PowerService};
use launcher_core::domain::power::PowerAction as CoreAction;
use tauri::ipc::Request;
use tauri::State;

use crate::dto::*;
use crate::ipc::{args, blocking, guard, no_args, not_wired};
use crate::state::AppState;

fn core_action(a: PowerAction) -> CoreAction {
    match a {
        PowerAction::Shutdown => CoreAction::Shutdown,
        PowerAction::Restart => CoreAction::Restart,
        PowerAction::Suspend => CoreAction::Suspend,
    }
}

fn availability(on: bool) -> Availability {
    if on {
        Availability::Available
    } else {
        Availability::Unavailable
    }
}

fn service(state: &AppState) -> Option<Arc<PowerService>> {
    state.power.get().cloned()
}

#[tauri::command]
pub async fn power_capabilities(
    request: Request<'_>,
    state: State<'_, AppState>,
) -> Result<PowerCapabilities, ErrorDto> {
    guard("power_capabilities", || {
        no_args(&request)?;
        let caps = service(&state).map_or(CoreCaps::NONE, |s| s.capabilities());
        Ok(PowerCapabilities {
            shutdown: availability(caps.shutdown),
            restart: availability(caps.restart),
            suspend: availability(caps.suspend),
        })
    })
}

#[tauri::command]
pub async fn power_prepare(
    request: Request<'_>,
    state: State<'_, AppState>,
) -> Result<String, ErrorDto> {
    guard("power_prepare", || {
        let a: PowerPrepareArgs = args(&request)?;
        service(&state)
            .ok_or(AppError::Unavailable)?
            .prepare(core_action(a.action))
    })
}

#[tauri::command]
pub async fn power_execute(
    request: Request<'_>,
    state: State<'_, AppState>,
) -> Result<(), ErrorDto> {
    let a: PowerExecuteArgs = guard("power_execute", || args(&request))?;
    let svc = service(&state);
    // Suspender so volta quando o PC acorda: espera em thread de bloqueio, nunca no runtime.
    blocking("power_execute", move || {
        svc.ok_or_else(|| not_wired("power_execute"))?
            .execute(&a.ticket.0)
    })
    .await
}
