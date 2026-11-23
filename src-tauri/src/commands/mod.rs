//! Comandos Tauri: cada um valida a entrada, chama o caso de uso e mapeia o erro (arquitetura
//! 2.3). Corpo curto e sem regra de negocio. A lista e fechada (`command_list.rs`).

pub mod library;
pub mod power;
pub mod session;
pub mod settings;
pub mod update;
pub mod volume;

/// Os 32 nomes de comando, na ordem do contrato.
pub mod list {
    include!("../../command_list.rs");
}
