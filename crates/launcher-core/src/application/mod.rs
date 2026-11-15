//! Aplicacao: casos de uso (um por acao) e ports implementados pela infraestrutura.

pub mod autostart_service;
pub mod display_service;
pub mod error;
pub mod foreground;
pub mod library_service;
pub mod ports;
pub mod power_service;
pub mod serial;
pub mod session_service;
pub mod settings_service;
pub mod volume_service;

pub use autostart_service::{AutostartService, AutostartStatus};
pub use display_service::{DisplayService, MonitorView};
pub use error::AppError;
pub use foreground::ForegroundBroker;
pub use library_service::{LibraryDeps, LibraryService, LibraryState};
pub use power_service::{PowerCapabilities, PowerService};
pub use serial::SerialGate;
pub use session_service::{SessionDeps, SessionService, SessionView};
pub use settings_service::SettingsService;
pub use volume_service::{VolumeService, VolumeView};
