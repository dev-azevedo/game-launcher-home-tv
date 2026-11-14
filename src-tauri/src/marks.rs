//! Marcas de tempo da partida (BE-40, CA-01, M-02): quanto falta, desde o inicio do processo, para
//! o primeiro paint da UI e para a biblioteca ficar navegavel. So log (nivel INFO), sem dado do
//! usuario; o roteiro de medicao do QA le estas linhas.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::OnceLock;
use std::time::Instant;

use launcher_core::domain::startup::StartMode;

static START: OnceLock<Instant> = OnceLock::new();
static ORIGIN: OnceLock<&'static str> = OnceLock::new();
static FIRST_PAINT: AtomicBool = AtomicBool::new(false);
static NAVIGABLE: AtomicBool = AtomicBool::new(false);

/// Chamada no inicio do `run`: fixa o instante zero e a origem da partida (`--autostart` =
/// inicio com o Windows, BE-40; nesse caso o M-02 conta a partir do disparo do autostart).
pub fn start(mode: StartMode) {
    let _ = START.set(Instant::now());
    let _ = ORIGIN.set(match mode {
        StartMode::Autostart => "autostart",
        _ => "manual",
    });
}

/// `autostart` ou `manual`, para o log da partida.
pub fn origin() -> &'static str {
    ORIGIN.get().copied().unwrap_or("manual")
}

fn elapsed_ms() -> u64 {
    START.get().map_or(0, |t| {
        u64::try_from(t.elapsed().as_millis()).unwrap_or(u64::MAX)
    })
}

/// Primeira leitura da biblioteca pela UI: ela ja carregou e pintou com a lista do cache.
pub fn first_paint() {
    if !FIRST_PAINT.swap(true, Ordering::AcqRel) {
        tracing::info!(
            marca = "primeiro_paint",
            origem = origin(),
            ms_desde_o_inicio = elapsed_ms(),
            "partida"
        );
    }
}

/// Primeira varredura concluida: a lista completa esta na tela (`library://changed`).
pub fn navigable() {
    if !NAVIGABLE.swap(true, Ordering::AcqRel) {
        tracing::info!(
            marca = "navegavel",
            origem = origin(),
            ms_desde_o_inicio = elapsed_ms(),
            "partida"
        );
    }
}
