//! Marcas de tempo da partida (BE-40, CA-01, M-02): quanto falta, desde o inicio do processo, para
//! o primeiro paint da UI e para a biblioteca ficar navegavel. So log (nivel INFO), sem dado do
//! usuario; o roteiro de medicao do QA le estas linhas.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::OnceLock;
use std::time::Instant;

static START: OnceLock<Instant> = OnceLock::new();
static FIRST_PAINT: AtomicBool = AtomicBool::new(false);
static NAVIGABLE: AtomicBool = AtomicBool::new(false);

/// Chamada no inicio do `run`: fixa o instante zero.
pub fn start() {
    let _ = START.set(Instant::now());
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
            ms_desde_o_inicio = elapsed_ms(),
            "partida"
        );
    }
}
