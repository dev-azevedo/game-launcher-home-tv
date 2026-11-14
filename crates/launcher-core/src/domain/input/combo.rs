//! `ReturnComboDetector` (RN-11, CA-32, CA-33): View + Menu segurados juntos por 1,5 s.
//! Dispara uma vez por pressao e cancela se qualquer um dos dois for solto antes do tempo.

use crate::domain::time::Millis;

/// Tempo de seguranca da combinacao (RN-11).
pub const COMBO_HOLD_MS: u64 = 1_500;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReturnComboDetector {
    hold_ms: u64,
    held_since: Option<Millis>,
    fired: bool,
}

impl Default for ReturnComboDetector {
    fn default() -> Self {
        Self::new(COMBO_HOLD_MS)
    }
}

impl ReturnComboDetector {
    pub fn new(hold_ms: u64) -> Self {
        Self {
            hold_ms,
            held_since: None,
            fired: false,
        }
    }

    /// Alimenta uma amostra. Devolve `true` **uma unica vez** por pressao, no primeiro instante em
    /// que as duas teclas completam `hold_ms` seguidos. Soltar qualquer uma reinicia tudo.
    pub fn update(&mut self, now: Millis, view: bool, menu: bool) -> bool {
        if !(view && menu) {
            self.held_since = None;
            self.fired = false;
            return false;
        }
        let since = *self.held_since.get_or_insert(now);
        if !self.fired && now.since(since) >= self.hold_ms {
            self.fired = true;
            return true;
        }
        false
    }

    /// Esquece a pressao em andamento (troca de modo, sessao encerrada).
    pub fn reset(&mut self) {
        self.held_since = None;
        self.fired = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn t(ms: u64) -> Millis {
        Millis(ms)
    }

    #[test]
    fn dispara_so_aos_1500_ms_cravados_e_uma_vez() {
        let mut d = ReturnComboDetector::default();
        assert!(!d.update(t(1000), true, true));
        assert!(!d.update(t(2499), true, true), "1499 ms ainda nao");
        assert!(d.update(t(2500), true, true), "1500 ms dispara");
        assert!(!d.update(t(2520), true, true), "uma vez por pressao");
        assert!(!d.update(t(9000), true, true));
    }

    #[test]
    fn soltar_qualquer_um_antes_do_tempo_cancela_e_exige_recomecar() {
        // CA-33: menos tempo que o definido, nada acontece.
        for (v1, m1) in [(false, true), (true, false), (false, false)] {
            let mut d = ReturnComboDetector::default();
            assert!(!d.update(t(0), true, true));
            assert!(!d.update(t(1400), true, true));
            assert!(!d.update(t(1450), v1, m1), "soltou");
            assert!(!d.update(t(1500), true, true), "recomeca a contar daqui");
            assert!(!d.update(t(2999), true, true));
            assert!(d.update(t(3000), true, true));
        }
    }

    #[test]
    fn so_uma_tecla_segurada_nunca_dispara() {
        let mut d = ReturnComboDetector::default();
        for ms in (0..10_000).step_by(50) {
            assert!(!d.update(t(ms), true, false));
            assert!(!d.update(t(ms), false, true));
        }
    }

    #[test]
    fn nova_pressao_depois_de_disparar_dispara_de_novo() {
        let mut d = ReturnComboDetector::default();
        d.update(t(0), true, true);
        assert!(d.update(t(1500), true, true));
        assert!(!d.update(t(1600), false, false));
        d.update(t(2000), true, true);
        assert!(!d.update(t(3499), true, true));
        assert!(d.update(t(3500), true, true));
    }

    #[test]
    fn amostra_unica_apos_longa_pausa_ainda_conta_o_tempo_real() {
        // A thread de 20 Hz pode atrasar; o tempo decorrido e o que vale.
        let mut d = ReturnComboDetector::default();
        d.update(t(100), true, true);
        assert!(d.update(t(4000), true, true));
    }

    #[test]
    fn reset_esquece_a_pressao() {
        let mut d = ReturnComboDetector::default();
        d.update(t(0), true, true);
        d.reset();
        assert!(
            !d.update(t(1500), true, true),
            "contagem recomecou no reset"
        );
        assert!(d.update(t(3000), true, true));
    }
}
