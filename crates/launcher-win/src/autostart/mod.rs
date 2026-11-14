//! Adapters Win32 do "Iniciar com o Windows" (BE-36, SEC-22): a tarefa de logon do proprio usuario
//! (`ITaskService`), o valor em `HKCU\...\Run` e os fatos da instalacao. A politica (fallback,
//! um mecanismo por vez, `Program Files`, `InstallDefaults`) e do `AutostartService`.
//!
//! INV-03: so `task_scheduler.rs` registra tarefa e so `run_key.rs` toca o `Run` e o
//! `StartupApproved` (guard por grep no CI).

pub mod install_env;
pub mod run_key;
pub mod task_scheduler;

pub use install_env::WinInstallEnv;
pub use run_key::WinRunEntry;
pub use task_scheduler::WinTaskScheduler;
