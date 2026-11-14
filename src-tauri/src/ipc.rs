//! Borda da IPC: leitura tipada dos argumentos e execucao de cada comando sob `catch_unwind`
//! (SEC-03, SEC-06, INV-02).
//!
//! Todo comando recebe o `Request` cru e o desserializa num `*Args` fechado (`dto.rs`). Assim
//! campo extra, campo ausente, tipo errado ou valor mal formado viram `InvalidRequest` com
//! `ErrorDto`, em vez da mensagem em texto que o Tauri devolve por conta propria.

use std::panic::{catch_unwind, AssertUnwindSafe};

use launcher_core::application::AppError;
use serde::de::DeserializeOwned;
use serde_json::Value;
use tauri::ipc::{InvokeBody, Request};

use crate::dto::ErrorDto;

/// Desserializa o corpo JSON de um comando com argumentos.
pub fn parse_json<T: DeserializeOwned>(body: &InvokeBody) -> Result<T, AppError> {
    match body {
        InvokeBody::Json(value) => serde_json::from_value(value.clone()).map_err(|e| {
            // Detalhe so no log; o chamador recebe `InvalidRequest` e nada mais.
            tracing::debug!(error = %e, "argumentos de comando rejeitados");
            AppError::InvalidRequest
        }),
        InvokeBody::Raw(_) => Err(AppError::InvalidRequest),
    }
}

/// Comando sem argumentos: aceita `{}` ou `null`; qualquer campo extra e `InvalidRequest`.
pub fn require_no_args(body: &InvokeBody) -> Result<(), AppError> {
    match body {
        InvokeBody::Json(Value::Null) => Ok(()),
        InvokeBody::Json(Value::Object(map)) if map.is_empty() => Ok(()),
        _ => Err(AppError::InvalidRequest),
    }
}

pub fn args<T: DeserializeOwned>(request: &Request<'_>) -> Result<T, AppError> {
    parse_json(request.body())
}

pub fn no_args(request: &Request<'_>) -> Result<(), AppError> {
    require_no_args(request.body())
}

/// Executa o corpo de um comando: pânico vira `Internal` (so log), erro vira `ErrorDto`.
pub fn guard<T>(
    command: &'static str,
    body: impl FnOnce() -> Result<T, AppError>,
) -> Result<T, ErrorDto> {
    match catch_unwind(AssertUnwindSafe(body)) {
        Ok(Ok(value)) => Ok(value),
        Ok(Err(err)) => {
            tracing::debug!(command, code = err.code(), "comando falhou");
            Err(ErrorDto::from(&err))
        }
        Err(_) => {
            tracing::error!(command, "panico capturado no comando");
            Err(ErrorDto::from(AppError::Internal))
        }
    }
}

/// Comando ainda sem caso de uso ligado: registra e devolve `Internal`. Some quando a tarefa BE
/// dona do comando entra (BE-20, BE-29, BE-30, BE-32 a BE-34, BE-36).
pub fn not_wired(command: &'static str) -> AppError {
    tracing::warn!(command, "comando ainda nao ligado a um caso de uso");
    AppError::Internal
}

/// Roda o corpo numa thread de bloqueio do runtime (inicio de jogo, UAC e fechar podem demorar)
/// sob `guard` (pânico vira `Internal`). Os argumentos ja foram lidos pelo chamador.
pub async fn blocking<T: Send + 'static>(
    command: &'static str,
    body: impl FnOnce() -> Result<T, AppError> + Send + 'static,
) -> Result<T, ErrorDto> {
    match tauri::async_runtime::spawn_blocking(move || guard(command, body)).await {
        Ok(result) => result,
        Err(error) => {
            tracing::error!(command, %error, "tarefa de comando nao concluiu");
            Err(ErrorDto::from(AppError::Internal))
        }
    }
}
