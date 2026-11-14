//! `InputEngine`: zona morta, repeticao, borda do gatilho, varios controles como um e gating por
//! modo (RN-06, RN-08, RN-11, RN-23, CA-06 a CA-08, CA-11, CA-12, CA-33, CA-64, CA-65, E-05, E-06,
//! E-20). Sem I/O: o adapter XInput entrega as amostras e o `Millis` de cada leitura.

use super::{Buttons, InputIntent, InputMode, PadSample, ReturnComboDetector};
use crate::domain::time::Millis;

/// Atraso ate a primeira repeticao (CA-07).
pub const REPEAT_DELAY_MS: u64 = 400;
/// Intervalo entre repeticoes depois do atraso: 10 por segundo, rapido o bastante para listas
/// longas e lento o bastante para a UI nao perder o foco de vista.
pub const REPEAT_INTERVAL_MS: u64 = 100;
/// Zona morta inicial do analogico: 25% do curso (RN-06).
pub const DEFAULT_DEADZONE_PERCENT: u8 = 25;
/// Gatilho "puxado" ao passar do meio do curso (RN-23).
pub const TRIGGER_PRESS: u8 = 128;
/// Gatilho "solto" abaixo deste valor (histerese contra tremor perto do meio).
pub const TRIGGER_RELEASE: u8 = 96;

// Bits internos, um por entrada logica (todos os controles combinados).
const UP: u16 = 1 << 0;
const DOWN: u16 = 1 << 1;
const LEFT: u16 = 1 << 2;
const RIGHT: u16 = 1 << 3;
const CONFIRM: u16 = 1 << 4;
const BACK: u16 = 1 << 5;
const MENU: u16 = 1 << 6;
const VIEW: u16 = 1 << 7;
const PAGE_PREV: u16 = 1 << 8;
const PAGE_NEXT: u16 = 1 << 9;
const LT: u16 = 1 << 10;
const RT: u16 = 1 << 11;

const DIRECTIONS: u16 = UP | DOWN | LEFT | RIGHT;

/// Ordem de emissao dentro de uma mesma leitura (deterministica).
const ORDER: [(u16, InputIntent); 12] = [
    (UP, InputIntent::Up),
    (DOWN, InputIntent::Down),
    (LEFT, InputIntent::Left),
    (RIGHT, InputIntent::Right),
    (CONFIRM, InputIntent::Confirm),
    (BACK, InputIntent::Back),
    (MENU, InputIntent::Menu),
    (VIEW, InputIntent::View),
    (PAGE_PREV, InputIntent::PagePrev),
    (PAGE_NEXT, InputIntent::PageNext),
    (LT, InputIntent::VolumeDown),
    (RT, InputIntent::VolumeUp),
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InputConfig {
    /// `advanced.input.deadzonePercent` (10..=50). Fora disso o engine usa o limite mais proximo.
    pub deadzone_percent: u8,
    pub repeat_delay_ms: u64,
    pub repeat_interval_ms: u64,
}

impl Default for InputConfig {
    fn default() -> Self {
        Self {
            deadzone_percent: DEFAULT_DEADZONE_PERCENT,
            repeat_delay_ms: REPEAT_DELAY_MS,
            repeat_interval_ms: REPEAT_INTERVAL_MS,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IntentEvent {
    pub intent: InputIntent,
    pub repeat: bool,
}

/// Resultado de uma leitura.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Poll {
    pub intents: Vec<IntentEvent>,
    /// A combinacao de retorno completou 1,5 s nesta leitura (so em segundo plano).
    pub combo_fired: bool,
    /// Controles conectados nesta leitura (0..=4).
    pub connected: u8,
}

#[derive(Debug, Clone)]
pub struct InputEngine {
    cfg: InputConfig,
    mode: InputMode,
    /// Entradas ativas na leitura anterior, sem filtro (base da histerese).
    raw_prev: u16,
    /// Entradas ativas na leitura anterior depois de tirar as travadas (base das bordas).
    effective_prev: u16,
    /// Entradas seguradas na hora de uma troca de modo: ficam mudas ate serem soltas.
    suppressed: u16,
    /// Proxima repeticao de cada direcao (indice = posicao em `DIRECTIONS`).
    next_repeat: [Option<Millis>; 4],
    combo: ReturnComboDetector,
}

impl InputEngine {
    pub fn new(cfg: InputConfig) -> Self {
        Self {
            cfg,
            mode: InputMode::Foreground,
            raw_prev: 0,
            effective_prev: 0,
            suppressed: 0,
            next_repeat: [None; 4],
            combo: ReturnComboDetector::default(),
        }
    }

    pub fn mode(&self) -> InputMode {
        self.mode
    }

    /// Troca o modo (CA-12). O que estiver segurado na hora **nao** vira acao: por exemplo, a
    /// combinacao de retorno ainda esta pressionada quando o launcher volta para a frente, e o
    /// `View`/`Menu` dela nao pode abrir telas. Volta a valer depois de soltar.
    pub fn set_mode(&mut self, mode: InputMode) {
        if mode == self.mode {
            return;
        }
        self.mode = mode;
        self.suppressed = self.raw_prev;
        self.effective_prev = 0;
        self.next_repeat = [None; 4];
        self.combo.reset();
    }

    /// Processa uma leitura de todos os slots (ate 4 controles) no instante `now`.
    pub fn poll(&mut self, now: Millis, pads: &[PadSample]) -> Poll {
        let (raw, connected) = self.combine(pads);
        self.suppressed &= raw; // soltou: volta a valer
        let effective = raw & !self.suppressed;

        let mut out = Poll {
            connected,
            ..Poll::default()
        };
        match self.mode {
            InputMode::Foreground => {
                self.combo.reset();
                self.emit(now, effective, &mut out.intents);
            }
            InputMode::Background => {
                self.next_repeat = [None; 4];
                out.combo_fired =
                    self.combo
                        .update(now, effective & VIEW != 0, effective & MENU != 0);
            }
            InputMode::Off => {
                self.next_repeat = [None; 4];
                self.combo.reset();
            }
        }
        self.raw_prev = raw;
        self.effective_prev = effective;
        out
    }

    fn combine(&self, pads: &[PadSample]) -> (u16, u8) {
        let dz = u64::from(self.cfg.deadzone_percent.clamp(10, 50));
        let press = (32_767 * dz / 100) as i64;
        let release = press * 80 / 100; // histerese: solta em 80% da zona morta

        let mut bits = 0u16;
        let mut connected = 0u8;
        for pad in pads.iter().filter(|p| p.connected) {
            connected += 1;
            let b = pad.buttons;
            let map = [
                (Buttons::DPAD_UP, UP),
                (Buttons::DPAD_DOWN, DOWN),
                (Buttons::DPAD_LEFT, LEFT),
                (Buttons::DPAD_RIGHT, RIGHT),
                (Buttons::A, CONFIRM),
                (Buttons::B, BACK),
                (Buttons::MENU, MENU),
                (Buttons::VIEW, VIEW),
                (Buttons::LB, PAGE_PREV),
                (Buttons::RB, PAGE_NEXT),
            ];
            for (button, bit) in map {
                if b.contains(button) {
                    bits |= bit;
                }
            }

            // Analogico esquerdo: raio acima da zona morta, direcao pelo eixo dominante.
            let (x, y) = (i64::from(pad.thumb_lx), i64::from(pad.thumb_ly));
            let mag2 = x * x + y * y;
            let dir = if y.abs() >= x.abs() {
                if y > 0 {
                    UP
                } else {
                    DOWN
                }
            } else if x > 0 {
                RIGHT
            } else {
                LEFT
            };
            let was_active = self.raw_prev & dir != 0;
            let threshold = if was_active { release } else { press };
            if mag2 >= threshold * threshold && mag2 > 0 {
                bits |= dir;
            }

            // Gatilhos: borda no meio do curso.
            for (value, bit) in [(pad.left_trigger, LT), (pad.right_trigger, RT)] {
                let limit = if self.raw_prev & bit != 0 {
                    TRIGGER_RELEASE
                } else {
                    TRIGGER_PRESS
                };
                if value >= limit {
                    bits |= bit;
                }
            }
        }
        (bits, connected)
    }

    fn emit(&mut self, now: Millis, effective: u16, out: &mut Vec<IntentEvent>) {
        let pressed = effective & !self.effective_prev;
        for (bit, intent) in ORDER {
            if pressed & bit != 0 {
                out.push(IntentEvent {
                    intent,
                    repeat: false,
                });
            }
        }
        // Repeticao: so direcoes (CA-07). Botoes e gatilhos contam uma vez por pressao (RN-23).
        for (i, (bit, intent)) in ORDER.iter().take(4).enumerate() {
            debug_assert_eq!(*bit & DIRECTIONS, *bit);
            if effective & bit == 0 {
                self.next_repeat[i] = None;
                continue;
            }
            if pressed & bit != 0 {
                self.next_repeat[i] = Some(now + self.cfg.repeat_delay_ms);
                continue;
            }
            if let Some(next) = self.next_repeat[i] {
                if now >= next {
                    out.push(IntentEvent {
                        intent: *intent,
                        repeat: true,
                    });
                    let mut following = next + self.cfg.repeat_interval_ms;
                    if following <= now {
                        following = now + self.cfg.repeat_interval_ms; // atraso grande: sem rajada
                    }
                    self.next_repeat[i] = Some(following);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use InputIntent as I;

    const T0: u64 = 10_000;

    fn pad(buttons: Buttons) -> PadSample {
        PadSample {
            connected: true,
            buttons,
            ..PadSample::default()
        }
    }

    fn stick(x: i16, y: i16) -> PadSample {
        PadSample {
            connected: true,
            thumb_lx: x,
            thumb_ly: y,
            ..PadSample::default()
        }
    }

    fn triggers(l: u8, r: u8) -> PadSample {
        PadSample {
            connected: true,
            left_trigger: l,
            right_trigger: r,
            ..PadSample::default()
        }
    }

    fn engine() -> InputEngine {
        InputEngine::new(InputConfig::default())
    }

    fn intents(p: &Poll) -> Vec<(InputIntent, bool)> {
        p.intents.iter().map(|e| (e.intent, e.repeat)).collect()
    }

    fn at(e: &mut InputEngine, ms: u64, pads: &[PadSample]) -> Poll {
        e.poll(Millis(T0 + ms), pads)
    }

    // ---- CA-06: direcoes e botoes ----

    #[test]
    fn dpad_e_botoes_viram_intencoes_na_borda_de_descida() {
        let casos = [
            (Buttons::DPAD_UP, I::Up),
            (Buttons::DPAD_DOWN, I::Down),
            (Buttons::DPAD_LEFT, I::Left),
            (Buttons::DPAD_RIGHT, I::Right),
            (Buttons::A, I::Confirm),
            (Buttons::B, I::Back),
            (Buttons::MENU, I::Menu),
            (Buttons::VIEW, I::View),
            (Buttons::LB, I::PagePrev),
            (Buttons::RB, I::PageNext),
        ];
        for (button, intent) in casos {
            let mut e = engine();
            assert!(intents(&at(&mut e, 0, &[pad(Buttons::default())])).is_empty());
            assert_eq!(
                intents(&at(&mut e, 8, &[pad(button)])),
                vec![(intent, false)]
            );
            // segurar nao repete botao nenhum nos primeiros 300 ms
            assert!(intents(&at(&mut e, 100, &[pad(button)])).is_empty());
            assert!(intents(&at(&mut e, 300, &[pad(button)])).is_empty());
        }
    }

    #[test]
    fn analogico_acima_da_zona_morta_move_na_direcao_dominante() {
        let casos = [
            (stick(0, 32_000), I::Up),
            (stick(0, -32_000), I::Down),
            (stick(-32_000, 0), I::Left),
            (stick(32_000, 0), I::Right),
            (stick(20_000, 30_000), I::Up), // diagonal: vence o eixo maior
            (stick(-30_000, -20_000), I::Left),
            (stick(i16::MIN, 0), I::Left), // extremo negativo sem estourar
            (stick(0, i16::MIN), I::Down),
        ];
        for (p, intent) in casos {
            let mut e = engine();
            assert_eq!(
                intents(&at(&mut e, 0, &[p])),
                vec![(intent, false)],
                "{p:?}"
            );
        }
    }

    // ---- CA-08 / RN-06: zona morta ----

    #[test]
    fn drift_leve_nao_move_o_foco() {
        // 25% de 32767 = 8191. Drift tipico: 6000 em um eixo, ou 5000+5000 (raio 7071).
        let mut e = engine();
        for (i, p) in [
            stick(6_000, 0),
            stick(0, -6_000),
            stick(5_000, 5_000),
            stick(8_000, 0),
        ]
        .into_iter()
        .enumerate()
        {
            let out = at(&mut e, i as u64 * 8, &[p]);
            assert!(out.intents.is_empty(), "{p:?}");
        }
        // Repouso por 5 s: nada, nunca.
        for ms in (0..5_000).step_by(8) {
            assert!(at(&mut e, 100 + ms, &[stick(300, -250)]).intents.is_empty());
        }
    }

    #[test]
    fn limite_exato_da_zona_morta() {
        let mut e = engine();
        assert!(
            at(&mut e, 0, &[stick(8_190, 0)]).intents.is_empty(),
            "abaixo de 25%"
        );
        assert_eq!(
            intents(&at(&mut e, 8, &[stick(8_191, 0)])),
            vec![(I::Right, false)],
            "25% ativa"
        );
    }

    #[test]
    fn zona_morta_configuravel_e_limitada_a_10_50() {
        let mut maior = InputEngine::new(InputConfig {
            deadzone_percent: 50,
            ..InputConfig::default()
        });
        assert!(
            at(&mut maior, 0, &[stick(16_000, 0)]).intents.is_empty(),
            "50% = 16383"
        );
        assert!(!at(&mut maior, 8, &[stick(16_400, 0)]).intents.is_empty());
        let mut absurda = InputEngine::new(InputConfig {
            deadzone_percent: 0,
            ..InputConfig::default()
        });
        assert!(
            at(&mut absurda, 0, &[stick(2_000, 0)]).intents.is_empty(),
            "0 vira 10% (3276)"
        );
        let mut enorme = InputEngine::new(InputConfig {
            deadzone_percent: 99,
            ..InputConfig::default()
        });
        assert!(
            !at(&mut enorme, 0, &[stick(17_000, 0)]).intents.is_empty(),
            "99 vira 50%"
        );
    }

    #[test]
    fn histerese_evita_tremor_na_borda_da_zona_morta() {
        let mut e = engine();
        assert_eq!(
            intents(&at(&mut e, 0, &[stick(9_000, 0)])),
            vec![(I::Right, false)]
        );
        // Oscila entre 7.000 e 9.000 (acima de 80% de 8191 = 6552): continua pressionado, sem novas bordas.
        for (i, x) in [7_000, 9_000, 7_000, 9_000].into_iter().enumerate() {
            assert!(at(&mut e, 8 + i as u64 * 8, &[stick(x, 0)])
                .intents
                .is_empty());
        }
        // Soltou de verdade (abaixo de 6552) e voltou: nova borda.
        assert!(at(&mut e, 100, &[stick(6_000, 0)]).intents.is_empty());
        assert_eq!(
            intents(&at(&mut e, 108, &[stick(9_000, 0)])),
            vec![(I::Right, false)]
        );
    }

    // ---- CA-07: repeticao apos 400 ms ----

    #[test]
    fn segurar_direcao_repete_depois_de_400_ms_em_intervalos_regulares() {
        let mut e = engine();
        assert_eq!(
            intents(&at(&mut e, 0, &[pad(Buttons::DPAD_DOWN)])),
            vec![(I::Down, false)]
        );
        // 120 Hz (8 ms): nada antes de 400 ms.
        let mut repeats = Vec::new();
        for step in 1..=100u64 {
            let ms = step * 8;
            let out = at(&mut e, ms, &[pad(Buttons::DPAD_DOWN)]);
            for ev in out.intents {
                assert_eq!(
                    ev,
                    IntentEvent {
                        intent: I::Down,
                        repeat: true
                    }
                );
                repeats.push(ms);
            }
        }
        assert_eq!(repeats[0], 400, "primeira repeticao em 400 ms");
        // Intervalos de ~100 ms (a leitura de 8 ms alinha na proxima amostra).
        for pair in repeats.windows(2) {
            let gap = pair[1] - pair[0];
            assert!((96..=112).contains(&gap), "intervalo {gap} ms");
        }
        assert!(repeats.len() >= 3);
    }

    #[test]
    fn soltar_para_a_repeticao_e_apertar_de_novo_reinicia_o_atraso() {
        let mut e = engine();
        at(&mut e, 0, &[pad(Buttons::DPAD_UP)]);
        assert_eq!(
            intents(&at(&mut e, 400, &[pad(Buttons::DPAD_UP)])),
            vec![(I::Up, true)]
        );
        assert!(at(&mut e, 410, &[pad(Buttons::default())])
            .intents
            .is_empty());
        assert_eq!(
            intents(&at(&mut e, 420, &[pad(Buttons::DPAD_UP)])),
            vec![(I::Up, false)]
        );
        assert!(
            at(&mut e, 800, &[pad(Buttons::DPAD_UP)]).intents.is_empty(),
            "so 380 ms desde o novo toque"
        );
        assert_eq!(
            intents(&at(&mut e, 820, &[pad(Buttons::DPAD_UP)])),
            vec![(I::Up, true)]
        );
    }

    #[test]
    fn leitura_atrasada_nao_gera_rajada_de_repeticoes() {
        let mut e = engine();
        at(&mut e, 0, &[pad(Buttons::DPAD_UP)]);
        // A thread ficou parada por 2 s: uma repeticao, nao vinte.
        assert_eq!(
            intents(&at(&mut e, 2_000, &[pad(Buttons::DPAD_UP)])),
            vec![(I::Up, true)]
        );
        assert!(at(&mut e, 2_050, &[pad(Buttons::DPAD_UP)])
            .intents
            .is_empty());
        assert_eq!(
            intents(&at(&mut e, 2_100, &[pad(Buttons::DPAD_UP)])),
            vec![(I::Up, true)]
        );
    }

    #[test]
    fn repeticao_vale_tambem_para_o_analogico() {
        let mut e = engine();
        at(&mut e, 0, &[stick(0, 30_000)]);
        assert!(at(&mut e, 399, &[stick(0, 30_000)]).intents.is_empty());
        assert_eq!(
            intents(&at(&mut e, 400, &[stick(0, 30_000)])),
            vec![(I::Up, true)]
        );
    }

    // ---- RN-23 / CA-64 / E-20: gatilhos ----

    #[test]
    fn gatilho_conta_uma_vez_ao_passar_do_meio_do_curso_e_segurar_nao_repete() {
        let mut e = engine();
        assert!(
            at(&mut e, 0, &[triggers(0, 127)]).intents.is_empty(),
            "127 ainda e antes do meio"
        );
        assert_eq!(
            intents(&at(&mut e, 8, &[triggers(0, 128)])),
            vec![(I::VolumeUp, false)]
        );
        for ms in (16..2_000).step_by(8) {
            assert!(
                at(&mut e, ms, &[triggers(0, 255)]).intents.is_empty(),
                "segurar nao repete"
            );
        }
        // Solta e puxa de novo: nova puxada.
        assert!(at(&mut e, 2_100, &[triggers(0, 0)]).intents.is_empty());
        assert_eq!(
            intents(&at(&mut e, 2_108, &[triggers(0, 200)])),
            vec![(I::VolumeUp, false)]
        );
    }

    #[test]
    fn lt_baixa_e_rt_sobe() {
        let mut e = engine();
        assert_eq!(
            intents(&at(&mut e, 0, &[triggers(255, 0)])),
            vec![(I::VolumeDown, false)]
        );
        assert_eq!(
            intents(&at(&mut e, 8, &[triggers(255, 255)])),
            vec![(I::VolumeUp, false)]
        );
    }

    #[test]
    fn tremor_do_gatilho_perto_do_meio_nao_conta_duas_puxadas() {
        let mut e = engine();
        assert_eq!(intents(&at(&mut e, 0, &[triggers(0, 130)])).len(), 1);
        for (i, v) in [120, 130, 110, 135, 100].into_iter().enumerate() {
            assert!(
                at(&mut e, 8 + i as u64 * 8, &[triggers(0, v)])
                    .intents
                    .is_empty(),
                "{v}"
            );
        }
        assert!(
            at(&mut e, 100, &[triggers(0, 95)]).intents.is_empty(),
            "abaixo de 96 solta"
        );
        assert_eq!(intents(&at(&mut e, 108, &[triggers(0, 130)])).len(), 1);
    }

    #[test]
    fn e20_volume_e_direcao_ao_mesmo_tempo_nao_se_cancelam() {
        let mut e = engine();
        let dir = PadSample {
            buttons: Buttons::DPAD_RIGHT,
            ..PadSample::connected()
        };
        assert_eq!(intents(&at(&mut e, 0, &[dir])), vec![(I::Right, false)]);
        let dir_e_gatilho = PadSample {
            right_trigger: 255,
            ..dir
        };
        assert_eq!(
            intents(&at(&mut e, 200, &[dir_e_gatilho])),
            vec![(I::VolumeUp, false)]
        );
        // A direcao continua repetindo no tempo dela, gatilho ainda segurado.
        assert_eq!(
            intents(&at(&mut e, 400, &[dir_e_gatilho])),
            vec![(I::Right, true)]
        );
    }

    // ---- CA-11 / RN-08: varios controles ----

    #[test]
    fn varios_controles_valem_como_um_so() {
        let mut e = engine();
        // Controle 2 aperta A; o 1 esta parado. Responde igual.
        let out = at(
            &mut e,
            0,
            &[
                pad(Buttons::default()),
                pad(Buttons::A),
                PadSample::default(),
            ],
        );
        assert_eq!(intents(&out), vec![(I::Confirm, false)]);
        assert_eq!(out.connected, 2);
        // Dois controles apertando A juntos: uma intencao so (um usuario), nao duas.
        let mut e = engine();
        let out = at(&mut e, 0, &[pad(Buttons::A), pad(Buttons::A)]);
        assert_eq!(intents(&out), vec![(I::Confirm, false)]);
        // O segundo entra enquanto o primeiro segura: sem nova borda.
        let mut e = engine();
        at(&mut e, 0, &[pad(Buttons::A), pad(Buttons::default())]);
        assert!(at(&mut e, 8, &[pad(Buttons::A), pad(Buttons::A)])
            .intents
            .is_empty());
        // Soltar um deles com o outro ainda segurando: continua apertado.
        assert!(at(&mut e, 16, &[pad(Buttons::default()), pad(Buttons::A)])
            .intents
            .is_empty());
    }

    #[test]
    fn direcoes_de_controles_diferentes_somam() {
        let mut e = engine();
        let out = at(&mut e, 0, &[pad(Buttons::DPAD_UP), stick(30_000, 0)]);
        assert_eq!(intents(&out), vec![(I::Up, false), (I::Right, false)]);
    }

    #[test]
    fn slot_desconectado_e_ignorado_mesmo_com_valores_sujos() {
        let mut e = engine();
        let sujo = PadSample {
            connected: false,
            buttons: Buttons::A | Buttons::VIEW,
            thumb_lx: 32_000,
            right_trigger: 255,
            ..PadSample::default()
        };
        let out = at(&mut e, 0, &[sujo]);
        assert!(out.intents.is_empty());
        assert_eq!(out.connected, 0);
    }

    // ---- CA-12 / CA-65: gating por modo ----

    #[test]
    fn em_segundo_plano_nenhuma_intencao_sai() {
        let mut e = engine();
        e.set_mode(InputMode::Background);
        let tudo = PadSample {
            connected: true,
            buttons: Buttons::DPAD_UP | Buttons::A | Buttons::B | Buttons::LB | Buttons::RB,
            thumb_lx: 30_000,
            left_trigger: 255,
            right_trigger: 255,
            ..PadSample::default()
        };
        for ms in (0..2_000).step_by(50) {
            let out = at(&mut e, ms, &[tudo]);
            assert!(out.intents.is_empty(), "CA-12/CA-65 em {ms} ms");
            assert!(!out.combo_fired);
        }
    }

    #[test]
    fn desligado_nao_emite_nem_detecta_combinacao() {
        let mut e = engine();
        e.set_mode(InputMode::Off);
        let combo = pad(Buttons::VIEW | Buttons::MENU);
        for ms in (0..4_000).step_by(50) {
            let out = at(&mut e, ms, &[combo, pad(Buttons::A)]);
            assert!(out.intents.is_empty() && !out.combo_fired);
        }
    }

    #[test]
    fn so_em_primeiro_plano_o_gatilho_aciona_volume() {
        for mode in [InputMode::Background, InputMode::Off] {
            let mut e = engine();
            e.set_mode(mode);
            assert!(at(&mut e, 0, &[triggers(255, 255)]).intents.is_empty());
        }
    }

    // ---- RN-11 / CA-32 / CA-33 / E-05 / E-06: combinacao de retorno ----

    fn combo_pad() -> PadSample {
        pad(Buttons::VIEW | Buttons::MENU)
    }

    #[test]
    fn combinacao_em_segundo_plano_dispara_aos_1500_ms_uma_vez() {
        let mut e = engine();
        e.set_mode(InputMode::Background);
        at(&mut e, 0, &[pad(Buttons::default())]);
        let mut fired_at = Vec::new();
        for ms in (8..4_000).step_by(8) {
            if at(&mut e, ms, &[combo_pad()]).combo_fired {
                fired_at.push(ms);
            }
        }
        assert_eq!(fired_at.len(), 1, "uma vez por pressao");
        assert!(
            (1_500..1_510).contains(&(fired_at[0] - 8)),
            "{}",
            fired_at[0]
        );
    }

    #[test]
    fn ca33_apertar_menos_que_o_tempo_nao_faz_nada() {
        let mut e = engine();
        e.set_mode(InputMode::Background);
        for ms in (0..1_400).step_by(20) {
            assert!(!at(&mut e, ms, &[combo_pad()]).combo_fired);
        }
        assert!(
            !at(&mut e, 1_420, &[pad(Buttons::default())]).combo_fired,
            "soltou"
        );
        for ms in (1_440..2_800).step_by(20) {
            assert!(
                !at(&mut e, ms, &[combo_pad()]).combo_fired,
                "contagem recomecou em {ms}"
            );
        }
    }

    #[test]
    fn combinacao_vale_com_view_num_controle_e_menu_em_outro() {
        let mut e = engine();
        e.set_mode(InputMode::Background);
        let mut fired = false;
        for ms in (0..1_600).step_by(20) {
            fired |= at(&mut e, ms, &[pad(Buttons::VIEW), pad(Buttons::MENU)]).combo_fired;
        }
        assert!(fired, "RN-08: dois controles contam como um usuario");
    }

    #[test]
    fn e06_com_o_launcher_na_frente_a_combinacao_nao_tem_efeito_de_retorno() {
        let mut e = engine(); // Foreground
        let mut combo = false;
        let mut views = 0;
        let mut menus = 0;
        for ms in (0..3_000).step_by(20) {
            let out = at(&mut e, ms, &[combo_pad()]);
            combo |= out.combo_fired;
            views += out.intents.iter().filter(|i| i.intent == I::View).count();
            menus += out.intents.iter().filter(|i| i.intent == I::Menu).count();
        }
        assert!(!combo, "RN-11: so tem efeito em segundo plano com sessao");
        assert_eq!(
            (views, menus),
            (1, 1),
            "em primeiro plano View e Menu sao botoes normais"
        );
    }

    #[test]
    fn e05_desconectar_durante_a_pressao_cancela_e_reconectar_volta_a_funcionar() {
        let mut e = engine();
        e.set_mode(InputMode::Background);
        for ms in (0..1_000).step_by(20) {
            at(&mut e, ms, &[combo_pad()]);
        }
        // Controle some no meio da pressao.
        for ms in (1_000..2_000).step_by(20) {
            let out = at(&mut e, ms, &[PadSample::default()]);
            assert!(!out.combo_fired && out.connected == 0);
        }
        // Volta e o jogador aperta de novo: precisa dos 1,5 s inteiros a partir daqui.
        let mut fired = None;
        for ms in (2_000..4_200).step_by(20) {
            if at(&mut e, ms, &[combo_pad()]).combo_fired {
                fired = Some(ms);
            }
        }
        let fired = fired.expect("voltou a funcionar");
        assert!(fired >= 3_500, "contou do zero ao reconectar: {fired}");
    }

    #[test]
    fn combinacao_ja_segurada_ao_entrar_em_segundo_plano_exige_soltar_e_apertar() {
        let mut e = engine();
        for ms in (0..200).step_by(20) {
            at(&mut e, ms, &[combo_pad()]);
        }
        e.set_mode(InputMode::Background); // jogo abriu com os botoes ainda apertados
        for ms in (200..4_000).step_by(20) {
            assert!(!at(&mut e, ms, &[combo_pad()]).combo_fired);
        }
        at(&mut e, 4_000, &[pad(Buttons::default())]);
        let mut fired = false;
        for ms in (4_020..5_700).step_by(20) {
            fired |= at(&mut e, ms, &[combo_pad()]).combo_fired;
        }
        assert!(fired);
    }

    // ---- troca de modo ----

    #[test]
    fn voltar_para_a_frente_com_a_combinacao_ainda_segurada_nao_aciona_view_nem_menu() {
        let mut e = engine();
        e.set_mode(InputMode::Background);
        let mut fired_at = None;
        for ms in (0..2_000).step_by(20) {
            if at(&mut e, ms, &[combo_pad()]).combo_fired {
                fired_at = Some(ms);
                break;
            }
        }
        assert!(fired_at.is_some());
        // O nucleo trouxe o launcher para a frente; os botoes ainda estao apertados.
        e.set_mode(InputMode::Foreground);
        for ms in (1_520..2_500).step_by(20) {
            assert!(
                at(&mut e, ms, &[combo_pad()]).intents.is_empty(),
                "sem acao fantasma em {ms}"
            );
        }
        at(&mut e, 2_520, &[pad(Buttons::default())]);
        let out = at(&mut e, 2_540, &[pad(Buttons::VIEW)]);
        assert_eq!(
            intents(&out),
            vec![(I::View, false)],
            "depois de soltar volta ao normal"
        );
    }

    #[test]
    fn direcao_segurada_na_troca_de_modo_nao_repete_nem_dispara_ate_soltar() {
        let mut e = engine();
        e.set_mode(InputMode::Background);
        at(&mut e, 0, &[pad(Buttons::DPAD_DOWN)]);
        e.set_mode(InputMode::Foreground);
        for ms in (20..1_500).step_by(20) {
            assert!(at(&mut e, ms, &[pad(Buttons::DPAD_DOWN)])
                .intents
                .is_empty());
        }
    }

    #[test]
    fn mesmo_modo_nao_mexe_no_estado() {
        let mut e = engine();
        at(&mut e, 0, &[pad(Buttons::DPAD_DOWN)]);
        e.set_mode(InputMode::Foreground); // sem troca de fato
        assert_eq!(
            intents(&at(&mut e, 400, &[pad(Buttons::DPAD_DOWN)])),
            vec![(I::Down, true)]
        );
        assert_eq!(e.mode(), InputMode::Foreground);
    }

    #[test]
    fn contagem_de_controles_acompanha_conexao_e_desconexao() {
        let mut e = engine();
        assert_eq!(at(&mut e, 0, &[]).connected, 0);
        assert_eq!(
            at(&mut e, 8, &[PadSample::connected(), PadSample::default()]).connected,
            1
        );
        assert_eq!(at(&mut e, 16, &[PadSample::connected(); 4]).connected, 4);
        assert_eq!(at(&mut e, 24, &[PadSample::default(); 4]).connected, 0);
    }
}
