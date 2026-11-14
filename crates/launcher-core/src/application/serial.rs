//! Porta de serializacao das acoes do contrato (secao 0): uma execucao por vez, chamada
//! concorrente recebe `Busy` (SEC-06). O `Permit` libera a porta ao sair de escopo, inclusive
//! em `panic` com `catch_unwind`.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use super::error::AppError;

#[derive(Debug, Default, Clone)]
pub struct SerialGate {
    busy: Arc<AtomicBool>,
}

/// Posse da porta. Libera no `Drop`.
#[derive(Debug)]
pub struct Permit {
    busy: Arc<AtomicBool>,
}

impl SerialGate {
    pub fn new() -> Self {
        Self::default()
    }

    /// Entra na porta ou devolve `Busy`, sem esperar.
    pub fn try_enter(&self) -> Result<Permit, AppError> {
        if self
            .busy
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_ok()
        {
            Ok(Permit {
                busy: Arc::clone(&self.busy),
            })
        } else {
            Err(AppError::Busy)
        }
    }
}

impl Drop for Permit {
    fn drop(&mut self) {
        self.busy.store(false, Ordering::Release);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::panic::{catch_unwind, AssertUnwindSafe};

    #[test]
    fn segunda_entrada_recebe_busy_e_libera_ao_soltar() {
        let gate = SerialGate::new();
        let first = gate.try_enter().unwrap();
        assert_eq!(gate.try_enter().unwrap_err(), AppError::Busy);
        // Clones compartilham a mesma porta.
        assert_eq!(gate.clone().try_enter().unwrap_err(), AppError::Busy);
        drop(first);
        assert!(gate.try_enter().is_ok());
    }

    #[test]
    fn panico_dentro_da_porta_libera_a_porta() {
        let gate = SerialGate::new();
        let r = catch_unwind(AssertUnwindSafe(|| {
            let _p = gate.try_enter().unwrap();
            panic!("falha simulada");
        }));
        assert!(r.is_err());
        assert!(gate.try_enter().is_ok());
    }

    #[test]
    fn vinte_chamadas_concorrentes_deixam_passar_uma_por_vez() {
        use std::sync::atomic::AtomicUsize;
        let gate = SerialGate::new();
        let inside = Arc::new(AtomicUsize::new(0));
        let max_inside = Arc::new(AtomicUsize::new(0));
        let handles: Vec<_> = (0..20)
            .map(|_| {
                let gate = gate.clone();
                let inside = Arc::clone(&inside);
                let max_inside = Arc::clone(&max_inside);
                std::thread::spawn(move || {
                    if let Ok(_p) = gate.try_enter() {
                        let now = inside.fetch_add(1, Ordering::SeqCst) + 1;
                        max_inside.fetch_max(now, Ordering::SeqCst);
                        std::thread::sleep(std::time::Duration::from_millis(5));
                        inside.fetch_sub(1, Ordering::SeqCst);
                    }
                })
            })
            .collect();
        for h in handles {
            h.join().unwrap();
        }
        assert_eq!(max_inside.load(Ordering::SeqCst), 1);
    }
}
