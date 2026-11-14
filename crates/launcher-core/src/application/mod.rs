//! Aplicacao: casos de uso (um por acao) e ports implementados pela infraestrutura.

pub mod error;
pub mod ports;
pub mod serial;

pub use error::AppError;
pub use serial::SerialGate;
