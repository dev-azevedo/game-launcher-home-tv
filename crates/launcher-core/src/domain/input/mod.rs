//! Dominio de entrada (BE-13): do estado cru dos controles ate as intencoes da UI e a combinacao
//! de retorno. Puro: o tempo entra como `Millis` e o adapter XInput (BE-14) so le os botoes e
//! chama `InputEngine::poll`. Todos os controles contam como um unico usuario (RN-08).

mod combo;
mod engine;

pub use combo::ReturnComboDetector;
pub use engine::{InputConfig, InputEngine, IntentEvent, Poll};

/// Intencoes que o nucleo emite para a UI (`input://intent`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum InputIntent {
    Up,
    Down,
    Left,
    Right,
    Confirm,
    Back,
    Menu,
    View,
    PagePrev,
    PageNext,
    VolumeDown,
    VolumeUp,
}

/// Quem pode receber entrada (CA-12): na frente emite tudo; em segundo plano so detecta a
/// combinacao de retorno; desligado nao faz nada.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputMode {
    Foreground,
    Background,
    Off,
}

/// Botoes digitais de um controle (layout do dominio, nao os bits do XInput: o adapter mapeia).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Buttons(pub u16);

impl Buttons {
    pub const DPAD_UP: Buttons = Buttons(1 << 0);
    pub const DPAD_DOWN: Buttons = Buttons(1 << 1);
    pub const DPAD_LEFT: Buttons = Buttons(1 << 2);
    pub const DPAD_RIGHT: Buttons = Buttons(1 << 3);
    pub const A: Buttons = Buttons(1 << 4);
    pub const B: Buttons = Buttons(1 << 5);
    /// Botao "Menu" (Start).
    pub const MENU: Buttons = Buttons(1 << 6);
    /// Botao "View" (Back).
    pub const VIEW: Buttons = Buttons(1 << 7);
    pub const LB: Buttons = Buttons(1 << 8);
    pub const RB: Buttons = Buttons(1 << 9);

    pub const fn contains(self, other: Buttons) -> bool {
        self.0 & other.0 == other.0 && other.0 != 0
    }
}

impl std::ops::BitOr for Buttons {
    type Output = Buttons;
    fn bitor(self, rhs: Buttons) -> Buttons {
        Buttons(self.0 | rhs.0)
    }
}

/// Uma leitura de um controle. `connected = false` ignora todo o resto.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct PadSample {
    pub connected: bool,
    pub buttons: Buttons,
    /// Analogico esquerdo, -32768..=32767 (Y positivo = para cima, como no XInput).
    pub thumb_lx: i16,
    pub thumb_ly: i16,
    /// Gatilhos, 0..=255.
    pub left_trigger: u8,
    pub right_trigger: u8,
}

impl PadSample {
    pub fn connected() -> Self {
        Self {
            connected: true,
            ..Self::default()
        }
    }
}
