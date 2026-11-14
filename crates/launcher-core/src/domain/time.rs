//! Tempo como parametro (arquitetura 2.3): o dominio nunca le o relogio; recebe `Millis`.

use std::ops::{Add, Sub};

/// Instante monotonico em milissegundos desde um ponto arbitrario (inicio do processo).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct Millis(pub u64);

impl Millis {
    pub const ZERO: Millis = Millis(0);

    pub const fn from_secs(s: u64) -> Self {
        Millis(s * 1000)
    }

    /// Tempo decorrido desde `earlier`; 0 se o relogio andou para tras.
    pub fn since(self, earlier: Millis) -> u64 {
        self.0.saturating_sub(earlier.0)
    }
}

impl Add<u64> for Millis {
    type Output = Millis;
    fn add(self, rhs: u64) -> Millis {
        Millis(self.0.saturating_add(rhs))
    }
}

impl Sub for Millis {
    type Output = u64;
    fn sub(self, rhs: Millis) -> u64 {
        self.since(rhs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aritmetica_satura() {
        assert_eq!(Millis(100) + 50, Millis(150));
        assert_eq!(Millis(150) - Millis(100), 50);
        assert_eq!(Millis(100).since(Millis(150)), 0);
        assert_eq!(Millis(u64::MAX) + 1, Millis(u64::MAX));
        assert_eq!(Millis::from_secs(60), Millis(60_000));
    }
}
