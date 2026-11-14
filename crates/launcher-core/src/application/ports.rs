//! Ports implementados pela infraestrutura (`launcher-win`). Entram com a tarefa que os usa; a
//! lista completa esta na arquitetura 2.3.

use std::sync::atomic::{AtomicI64, AtomicU64, Ordering};

use crate::domain::library::StoredLibrary;
use crate::domain::settings::Settings;
use crate::domain::time::Millis;

/// Relogios. O dominio recebe `Millis`; quem chama o caso de uso le daqui.
pub trait Clock: Send + Sync {
    /// Monotonico, para prazos (60 s, 5 s, 10 s, 1,5 s).
    fn now(&self) -> Millis;
    /// Parede, em ms desde a epoca Unix, para `lastPlayedAt` e nomes de backup.
    fn unix_ms(&self) -> i64;
}

/// Relogio fake para testes: so anda quando mandam.
#[derive(Debug, Default)]
pub struct ManualClock {
    now: AtomicU64,
    unix: AtomicI64,
}

impl ManualClock {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn advance_ms(&self, ms: u64) {
        self.now.fetch_add(ms, Ordering::SeqCst);
        self.unix.fetch_add(ms as i64, Ordering::SeqCst);
    }

    pub fn set_unix_ms(&self, v: i64) {
        self.unix.store(v, Ordering::SeqCst);
    }
}

impl Clock for ManualClock {
    fn now(&self) -> Millis {
        Millis(self.now.load(Ordering::SeqCst))
    }
    fn unix_ms(&self) -> i64 {
        self.unix.load(Ordering::SeqCst)
    }
}

/// Por que um arquivo foi recriado com os padroes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RecoveryReason {
    /// Maior que o limite de 4 MB (nem foi lido).
    TooLarge,
    /// JSON invalido, campo desconhecido, versao desconhecida ou limite excedido.
    Invalid,
}

/// O arquivo anterior foi renomeado para `backup_name` (nome do arquivo, sem pasta).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Recovery {
    pub reason: RecoveryReason,
    pub backup_name: Option<String>,
}

/// Resultado de `load`: sempre ha um valor utilizavel; `recovery` diz se foi preciso recriar.
#[derive(Debug, Clone, PartialEq)]
pub struct Loaded<T> {
    pub value: T,
    pub recovery: Option<Recovery>,
    /// Avisos de valores corrigidos para o padrao (ex.: `advanced`), para o log.
    pub warnings: Vec<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum StoreError {
    #[error("falha de E/S na pasta de dados: {0}")]
    Io(#[from] std::io::Error),
    #[error("conteudo acima do limite de {0} bytes")]
    TooLarge(usize),
}

/// Dono unico de `library.json` (INV-03, INV-09).
pub trait LibraryRepository: Send + Sync {
    fn load(&self) -> Loaded<StoredLibrary>;
    fn save(&self, library: &StoredLibrary) -> Result<(), StoreError>;
}

/// Dono unico de `settings.json`. `save` preserva `advanced_raw` como recebido.
pub trait SettingsRepository: Send + Sync {
    fn load(&self) -> Loaded<Settings>;
    fn save(&self, settings: &Settings) -> Result<(), StoreError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn relogio_fake_so_anda_quando_mandam() {
        let c = ManualClock::new();
        assert_eq!(c.now(), Millis(0));
        c.advance_ms(1500);
        assert_eq!(c.now(), Millis(1500));
        assert_eq!(c.unix_ms(), 1500);
        c.set_unix_ms(1_794_610_000_000);
        assert_eq!(c.unix_ms(), 1_794_610_000_000);
    }
}
