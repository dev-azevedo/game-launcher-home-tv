//! Dominio de sessao (BE-15): maquina de estados com prazos e regras de exclusao de processo.

mod machine;
pub mod process_policy;

pub use machine::{
    FrontOutcome, LaunchFailure, Session, SessionConfig, SessionEffect, SessionSnapshot,
    SessionState,
};
pub use process_policy::{classify, Exclusion, ProcessInfo, SessionContext};
