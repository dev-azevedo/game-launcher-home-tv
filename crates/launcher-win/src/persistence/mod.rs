//! Persistencia JSON por usuario (ADR-0004). Unico dono dos arquivos de dados (INV-03, INV-09).

pub mod json;

pub use json::{
    atomic_write, JsonLibraryRepository, JsonScanCache, JsonSettingsRepository, MAX_FILE_BYTES,
};
