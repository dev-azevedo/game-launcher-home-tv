//! Adapter `GamepadSource` sobre XInput (BE-14, ADR-0002). **Unico** arquivo que chama
//! `XInputGetState` (INV-03). Nunca `XInputGetStateEx`: o Guide nao entra em fluxo nenhum.
//!
//! Slot vazio e consultado so a cada 1 s (custo medido no SP-01: 20 Hz x 4 slots vazios =
//! 0,52% de um nucleo); slot conectado e lido a cada leitura.

use launcher_core::application::ports::GamepadSource;
use launcher_core::domain::input::{Buttons, PadSample};
use launcher_core::domain::time::Millis;
use windows::Win32::UI::Input::XboxController::{
    XInputGetState, XINPUT_GAMEPAD_A, XINPUT_GAMEPAD_B, XINPUT_GAMEPAD_BACK,
    XINPUT_GAMEPAD_DPAD_DOWN, XINPUT_GAMEPAD_DPAD_LEFT, XINPUT_GAMEPAD_DPAD_RIGHT,
    XINPUT_GAMEPAD_DPAD_UP, XINPUT_GAMEPAD_LEFT_SHOULDER, XINPUT_GAMEPAD_RIGHT_SHOULDER,
    XINPUT_GAMEPAD_START, XINPUT_STATE,
};

/// Intervalo de consulta de um slot sem controle (ADR-0002).
pub const EMPTY_SLOT_PROBE_MS: u64 = 1_000;
pub const SLOTS: usize = 4;

/// Leitura crua de um slot, ainda nos bits do XInput.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct RawPad {
    pub buttons: u16,
    pub thumb_lx: i16,
    pub thumb_ly: i16,
    pub left_trigger: u8,
    pub right_trigger: u8,
}

/// A chamada de sistema atras de um trait: o teste troca por um fake.
pub trait RawPads: Send {
    /// `None` = slot sem controle.
    fn get(&mut self, slot: u32) -> Option<RawPad>;
}

/// `XInputGetState` de verdade.
#[derive(Debug, Default)]
pub struct SystemPads;

impl RawPads for SystemPads {
    fn get(&mut self, slot: u32) -> Option<RawPad> {
        let mut state = XINPUT_STATE::default();
        // SAFETY: `state` e um buffer valido e vive durante a chamada.
        let rc = unsafe { XInputGetState(slot, &mut state) };
        if rc != 0 {
            return None; // ERROR_DEVICE_NOT_CONNECTED ou qualquer outra falha: sem controle
        }
        let g = state.Gamepad;
        Some(RawPad {
            buttons: g.wButtons.0,
            thumb_lx: g.sThumbLX,
            thumb_ly: g.sThumbLY,
            left_trigger: g.bLeftTrigger,
            right_trigger: g.bRightTrigger,
        })
    }
}

/// Bits do XInput -> botoes do dominio.
pub fn map_buttons(raw: u16) -> Buttons {
    let table = [
        (XINPUT_GAMEPAD_DPAD_UP.0, Buttons::DPAD_UP),
        (XINPUT_GAMEPAD_DPAD_DOWN.0, Buttons::DPAD_DOWN),
        (XINPUT_GAMEPAD_DPAD_LEFT.0, Buttons::DPAD_LEFT),
        (XINPUT_GAMEPAD_DPAD_RIGHT.0, Buttons::DPAD_RIGHT),
        (XINPUT_GAMEPAD_A.0, Buttons::A),
        (XINPUT_GAMEPAD_B.0, Buttons::B),
        (XINPUT_GAMEPAD_START.0, Buttons::MENU),
        (XINPUT_GAMEPAD_BACK.0, Buttons::VIEW),
        (XINPUT_GAMEPAD_LEFT_SHOULDER.0, Buttons::LB),
        (XINPUT_GAMEPAD_RIGHT_SHOULDER.0, Buttons::RB),
    ];
    let mut out = Buttons::default();
    for (bit, button) in table {
        if raw & bit != 0 {
            out = out | button;
        }
    }
    out
}

pub struct XInputSource<R: RawPads> {
    raw: R,
    connected: [bool; SLOTS],
    next_probe: [Millis; SLOTS],
}

impl XInputSource<SystemPads> {
    pub fn system() -> Self {
        Self::new(SystemPads)
    }
}

impl<R: RawPads> XInputSource<R> {
    pub fn new(raw: R) -> Self {
        Self {
            raw,
            connected: [false; SLOTS],
            next_probe: [Millis::ZERO; SLOTS],
        }
    }
}

impl<R: RawPads> GamepadSource for XInputSource<R> {
    fn read(&mut self, now: Millis) -> [PadSample; SLOTS] {
        let mut out = [PadSample::default(); SLOTS];
        for slot in 0..SLOTS {
            if !self.connected[slot] && now < self.next_probe[slot] {
                continue; // slot vazio: so a cada 1 s
            }
            match self.raw.get(slot as u32) {
                Some(p) => {
                    self.connected[slot] = true;
                    out[slot] = PadSample {
                        connected: true,
                        buttons: map_buttons(p.buttons),
                        thumb_lx: p.thumb_lx,
                        thumb_ly: p.thumb_ly,
                        left_trigger: p.left_trigger,
                        right_trigger: p.right_trigger,
                    };
                }
                None => {
                    self.connected[slot] = false;
                    self.next_probe[slot] = now + EMPTY_SLOT_PROBE_MS;
                }
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::sync::Arc;

    /// Controle fake: o slot 1 pode ser ligado e desligado; conta consultas por slot.
    #[derive(Clone, Default)]
    struct Fake {
        present: Arc<[AtomicU32; SLOTS]>,
        calls: Arc<[AtomicU32; SLOTS]>,
        buttons: Arc<AtomicU32>,
    }

    impl RawPads for Fake {
        fn get(&mut self, slot: u32) -> Option<RawPad> {
            self.calls[slot as usize].fetch_add(1, Ordering::SeqCst);
            (self.present[slot as usize].load(Ordering::SeqCst) != 0).then(|| RawPad {
                buttons: self.buttons.load(Ordering::SeqCst) as u16,
                ..RawPad::default()
            })
        }
    }

    #[test]
    fn mapeia_os_botoes_publicos_do_xinput() {
        let raw = XINPUT_GAMEPAD_A.0
            | XINPUT_GAMEPAD_START.0
            | XINPUT_GAMEPAD_BACK.0
            | XINPUT_GAMEPAD_DPAD_UP.0
            | XINPUT_GAMEPAD_LEFT_SHOULDER.0;
        let b = map_buttons(raw);
        for esperado in [
            Buttons::A,
            Buttons::MENU,
            Buttons::VIEW,
            Buttons::DPAD_UP,
            Buttons::LB,
        ] {
            assert!(b.contains(esperado), "{esperado:?}");
        }
        assert!(!b.contains(Buttons::B));
        assert!(!b.contains(Buttons::RB));
        assert_eq!(map_buttons(0), Buttons::default());
    }

    #[test]
    fn slot_vazio_so_e_consultado_a_cada_segundo() {
        let fake = Fake::default();
        let mut src = XInputSource::new(fake.clone());
        // 20 Hz durante 3 s: 60 leituras.
        for i in 0..60u64 {
            let _ = src.read(Millis(i * 50));
        }
        for slot in 0..SLOTS {
            let n = fake.calls[slot].load(Ordering::SeqCst);
            assert!((3..=4).contains(&n), "slot {slot}: {n} consultas em 3 s");
        }
    }

    #[test]
    fn conexao_e_desconexao_aparecem_dentro_de_um_segundo_e_slot_conectado_le_toda_vez() {
        let fake = Fake::default();
        let mut src = XInputSource::new(fake.clone());
        assert!(!src.read(Millis(0))[1].connected);

        fake.present[1].store(1, Ordering::SeqCst);
        fake.buttons
            .store(u32::from(XINPUT_GAMEPAD_A.0), Ordering::SeqCst);
        // Detecta na proxima consulta do slot vazio (<= 1 s depois).
        assert!(!src.read(Millis(900))[1].connected);
        let s = src.read(Millis(1_000))[1];
        assert!(s.connected && s.buttons.contains(Buttons::A));

        let before = fake.calls[1].load(Ordering::SeqCst);
        for i in 1..=10u64 {
            let _ = src.read(Millis(1_000 + i * 50));
        }
        assert_eq!(fake.calls[1].load(Ordering::SeqCst) - before, 10);

        fake.present[1].store(0, Ordering::SeqCst);
        assert!(
            !src.read(Millis(1_600))[1].connected,
            "desconexao vista na leitura seguinte"
        );
    }

    #[test]
    #[ignore = "toca o XInput real; sem controle, so confere que nao quebra"]
    fn xinput_real_sem_controle_devolve_slots_vazios_sem_erro() {
        let mut src = XInputSource::system();
        let pads = src.read(Millis(0));
        assert_eq!(pads.len(), 4);
    }
}
