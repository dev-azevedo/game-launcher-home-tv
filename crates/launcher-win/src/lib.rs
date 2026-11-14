//! Adapters Win32 do Sofa (xinput, audio, power, foreground, process_probe, job, display,
//! system_events, cover_cache, folder_browser, path_canon, autostart, stores, persistence).
//! Cada um entra com a sua tarefa BE-xx. Nenhuma RN-xx aqui (arquitetura 2.3).

pub use launcher_core as core;

pub mod hardening;
pub mod logging;
pub mod persistence;
pub mod system_clock;
