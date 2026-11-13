//! Nucleo do Sofa: regras puras e casos de uso.
//!
//! INV-01: este crate nao depende de `tauri`, `windows` nem de I/O. O tempo entra como
//! parametro. Adapters vivem em `launcher-win`; a apresentacao em `src-tauri`.

pub mod application;
pub mod domain;
