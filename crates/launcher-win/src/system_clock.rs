//! `Clock` real: monotonico para prazos e relogio de parede para `lastPlayedAt` e backups.

use std::time::{Instant, SystemTime, UNIX_EPOCH};

use launcher_core::application::ports::Clock;
use launcher_core::domain::time::Millis;

#[derive(Debug)]
pub struct SystemClock {
    start: Instant,
}

impl SystemClock {
    pub fn new() -> Self {
        Self {
            start: Instant::now(),
        }
    }
}

impl Default for SystemClock {
    fn default() -> Self {
        Self::new()
    }
}

impl Clock for SystemClock {
    fn now(&self) -> Millis {
        Millis(u64::try_from(self.start.elapsed().as_millis()).unwrap_or(u64::MAX))
    }

    fn unix_ms(&self) -> i64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |d| i64::try_from(d.as_millis()).unwrap_or(i64::MAX))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn monotonico_nao_volta_e_parede_esta_depois_de_2024() {
        let c = SystemClock::new();
        let a = c.now();
        let b = c.now();
        assert!(b >= a);
        assert!(c.unix_ms() > 1_700_000_000_000);
    }
}
