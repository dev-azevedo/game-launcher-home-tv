//! Adapters Win32 do Sofa (xinput, audio, power, foreground, process_probe, job, display,
//! system_events, cover_cache, folder_browser, path_canon, autostart, stores, persistence).
//! Cada um entra com a sua tarefa BE-xx. Nenhuma RN-xx aqui (arquitetura 2.3).

pub use launcher_core as core;

pub mod audio;
pub mod autostart;
pub mod cover_cache;
pub mod display;
pub mod folder_browser;
pub mod foreground;
pub mod game_starter;
pub mod hardening;
pub mod input_thread;
pub mod job;
pub mod logging;
pub mod manual_exe;
pub mod path_canon;
pub mod persistence;
pub mod power;
pub mod process_probe;
pub mod purge;
pub mod stores;
pub mod system_clock;
pub mod system_events;
pub mod windows_enum;
pub mod xinput;
