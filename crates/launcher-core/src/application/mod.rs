//! Aplicacao: casos de uso (um por acao) e ports implementados pela infraestrutura.

pub mod error;
pub mod foreground;
pub mod library_service;
pub mod ports;
pub mod serial;
pub mod session_service;

pub use error::AppError;
pub use foreground::ForegroundBroker;
pub use library_service::{LibraryDeps, LibraryService, LibraryState};
pub use serial::SerialGate;
pub use session_service::{SessionDeps, SessionService, SessionView};
