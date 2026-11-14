//! Volume do dispositivo de saida padrao (RN-13, CA-38 a CA-40, CA-64, SEC-27): passo de 5 pontos,
//! faixa 0 a 100, mudo preservando o nivel guardado. Funcoes puras; o Windows entra pelo port
//! `AudioEndpoint` (aplicacao).

/// Tamanho do passo, em pontos percentuais (RN-13).
pub const STEP: u8 = 5;
pub const MAX_LEVEL: u8 = 100;

/// Estado do volume mestre. `level` esta sempre em 0..=100; o mudo nao altera `level` (o nivel
/// "guardado" do mudo e o proprio `level`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VolumeState {
    pub level: u8,
    pub muted: bool,
}

impl VolumeState {
    /// Valor vindo de fora (driver, mixer) preso a faixa.
    pub fn new(level: u8, muted: bool) -> Self {
        Self {
            level: level.min(MAX_LEVEL),
            muted,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VolumeCommand {
    /// RT / direita: sobe um passo; no mudo, desmuta e sobe (CA-40).
    Up,
    /// LT / esquerda: desce um passo; no mudo, baixa o nivel guardado e **mantem** o mudo (CA-64).
    Down,
    /// "Mudo" do menu: alterna; ao desmutar, o nivel anterior volta (CA-39).
    ToggleMute,
}

/// Regra unica dos tres comandos (BE-31).
pub fn apply(state: VolumeState, command: VolumeCommand) -> VolumeState {
    let state = VolumeState::new(state.level, state.muted);
    match command {
        VolumeCommand::Up => VolumeState {
            level: state.level.saturating_add(STEP).min(MAX_LEVEL),
            muted: false,
        },
        VolumeCommand::Down => VolumeState {
            level: state.level.saturating_sub(STEP),
            muted: state.muted,
        },
        VolumeCommand::ToggleMute => VolumeState {
            level: state.level,
            muted: !state.muted,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use VolumeCommand::{Down, ToggleMute, Up};

    fn s(level: u8, muted: bool) -> VolumeState {
        VolumeState { level, muted }
    }

    /// (estado antes, comando, estado esperado): valores escritos a mao, vindos dos CA.
    #[test]
    fn regras_por_tabela() {
        let tabela = [
            // CA-38: passo de 5, limitado a 0 e 100.
            (s(50, false), Up, s(55, false)),
            (s(50, false), Down, s(45, false)),
            (s(100, false), Up, s(100, false)),
            (s(98, false), Up, s(100, false)),
            (s(0, false), Down, s(0, false)),
            (s(3, false), Down, s(0, false)),
            // nivel fora do multiplo de 5 (mixer): continua somando 5
            (s(47, false), Up, s(52, false)),
            // CA-39: alternar mudo guarda o nivel e devolve o mesmo nivel ao desmutar.
            (s(40, false), ToggleMute, s(40, true)),
            (s(40, true), ToggleMute, s(40, false)),
            (s(0, false), ToggleMute, s(0, true)),
            // CA-40: subir no mudo desmuta e sobe um passo.
            (s(40, true), Up, s(45, false)),
            (s(100, true), Up, s(100, false)),
            (s(0, true), Up, s(5, false)),
            // CA-64: descer no mudo baixa o nivel guardado e mantem o mudo.
            (s(40, true), Down, s(35, true)),
            (s(2, true), Down, s(0, true)),
            (s(0, true), Down, s(0, true)),
        ];
        for (antes, comando, esperado) in tabela {
            assert_eq!(apply(antes, comando), esperado, "{antes:?} {comando:?}");
        }
    }

    #[test]
    fn valor_fora_da_faixa_e_preso_antes_de_aplicar() {
        assert_eq!(apply(s(250, false), Down), s(95, false));
        assert_eq!(VolumeState::new(255, true), s(100, true));
    }

    #[test]
    fn nunca_sai_de_0_a_100_em_qualquer_sequencia() {
        let mut st = s(50, false);
        let cmds = [Up, Up, ToggleMute, Down, Up, ToggleMute, Down, Down];
        for round in 0..500 {
            st = apply(st, cmds[round % cmds.len()]);
            assert!(st.level <= 100);
        }
        for _ in 0..40 {
            st = apply(st, Up);
        }
        assert_eq!(st, s(100, false));
        for _ in 0..40 {
            st = apply(st, Down);
        }
        assert_eq!(st, s(0, false));
    }
}
