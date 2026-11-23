//! `AppError` -> `ErrorDto`: o unico lugar onde o erro de aplicacao vira o erro da IPC (SEC-06).
//! `params` leva so `gameId` e `origin`; o texto tecnico fica no log.

use launcher_core::application::AppError;

use crate::dto::{ErrorCode, ErrorDto, ErrorParams, OriginDto};

fn code_of(e: &AppError) -> ErrorCode {
    match e {
        AppError::Busy => ErrorCode::Busy,
        AppError::InvalidRequest => ErrorCode::InvalidRequest,
        AppError::Internal => ErrorCode::Internal,
        AppError::Duplicate => ErrorCode::Duplicate,
        AppError::NotAnExecutable => ErrorCode::NotAnExecutable,
        AppError::PathNotAllowed => ErrorCode::PathNotAllowed,
        AppError::NetworkPathBlocked => ErrorCode::NetworkPathBlocked,
        AppError::NotFound => ErrorCode::NotFound,
        AppError::AccessDenied => ErrorCode::AccessDenied,
        AppError::EmptyName => ErrorCode::EmptyName,
        AppError::UnknownGame => ErrorCode::UnknownGame,
        AppError::SessionAlreadyActive { .. } => ErrorCode::SessionAlreadyActive,
        AppError::GameFileMissing { .. } => ErrorCode::GameFileMissing,
        AppError::StoreNotInstalled { .. } => ErrorCode::StoreNotInstalled,
        AppError::NeedsElevation { .. } => ErrorCode::NeedsElevation,
        AppError::ElevationDenied { .. } => ErrorCode::ElevationDenied,
        AppError::InvalidState => ErrorCode::InvalidState,
        AppError::NoActiveSession => ErrorCode::NoActiveSession,
        AppError::ForegroundDenied => ErrorCode::ForegroundDenied,
        AppError::NotOverdue => ErrorCode::NotOverdue,
        AppError::NoAudioDevice => ErrorCode::NoAudioDevice,
        AppError::Unavailable => ErrorCode::Unavailable,
        AppError::TicketInvalid => ErrorCode::TicketInvalid,
        AppError::Rejected => ErrorCode::Rejected,
        AppError::AutostartFailed => ErrorCode::AutostartFailed,
        AppError::UnknownMonitor => ErrorCode::UnknownMonitor,
        AppError::UpdateFailed => ErrorCode::UpdateFailed,
        AppError::UpdateCancelled => ErrorCode::UpdateCancelled,
    }
}

impl From<&AppError> for ErrorDto {
    fn from(e: &AppError) -> Self {
        let game_id = e.game_id().map(ToString::to_string);
        let origin = e.origin().map(OriginDto::from);
        let params = if game_id.is_some() || origin.is_some() {
            Some(ErrorParams { game_id, origin })
        } else {
            None
        };
        Self {
            code: code_of(e),
            params,
        }
    }
}

impl From<AppError> for ErrorDto {
    fn from(e: AppError) -> Self {
        Self::from(&e)
    }
}
