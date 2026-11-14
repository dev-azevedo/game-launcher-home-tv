//! Thread de entrada (BE-14): le os controles a 120 Hz com o Sofa na frente e a 20 Hz em segundo
//! plano (RN-21), passa pelo `InputEngine` do dominio e entrega intencoes, conexao e a combinacao
//! de retorno ao `InputSink`. Sem hook global: so `XInputGetState` por polling (SEC-28).

use std::sync::atomic::{AtomicBool, AtomicU64, AtomicU8, Ordering};
use std::sync::Arc;
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use launcher_core::application::ports::{Clock, GamepadSource, InputGate, InputSink};
use launcher_core::domain::input::{InputConfig, InputEngine, InputMode};
use windows::Win32::Media::{timeBeginPeriod, timeEndPeriod};

/// Periodo de leitura por modo. Em `Off` mantem 20 Hz so para contar controles.
pub const FOREGROUND_PERIOD: Duration = Duration::from_micros(8_333); // 120 Hz
pub const BACKGROUND_PERIOD: Duration = Duration::from_millis(50); // 20 Hz

fn mode_to_u8(m: InputMode) -> u8 {
    match m {
        InputMode::Foreground => 0,
        InputMode::Background => 1,
        InputMode::Off => 2,
    }
}

fn mode_from_u8(v: u8) -> InputMode {
    match v {
        1 => InputMode::Background,
        2 => InputMode::Off,
        _ => InputMode::Foreground,
    }
}

/// Estado compartilhado com o resto do app: modo pedido e controles conectados.
#[derive(Debug)]
pub struct InputControl {
    mode: AtomicU8,
    connected: AtomicU8,
    stats: LatencyStats,
}

impl Default for InputControl {
    fn default() -> Self {
        Self {
            mode: AtomicU8::new(mode_to_u8(InputMode::Foreground)),
            connected: AtomicU8::new(0),
            stats: LatencyStats::default(),
        }
    }
}

impl InputControl {
    pub fn mode(&self) -> InputMode {
        mode_from_u8(self.mode.load(Ordering::Acquire))
    }

    /// `input_status`: controles conectados na ultima leitura.
    pub fn connected(&self) -> u8 {
        self.connected.load(Ordering::Acquire)
    }

    pub fn stats(&self) -> LatencySnapshot {
        self.stats.snapshot()
    }
}

impl InputGate for InputControl {
    fn set_mode(&self, mode: InputMode) {
        self.mode.store(mode_to_u8(mode), Ordering::Release);
    }
}

/// Latencia do processamento de uma leitura que gerou intencao: da leitura do controle ate o
/// `emit` ter sido chamado (a meta de 50 ms botao -> foco inclui polling, IPC e UI).
#[derive(Debug, Default)]
pub struct LatencyStats {
    count: AtomicU64,
    total_us: AtomicU64,
    max_us: AtomicU64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LatencySnapshot {
    pub count: u64,
    pub mean_us: u64,
    pub max_us: u64,
}

impl LatencyStats {
    fn record(&self, d: Duration) -> bool {
        let us = u64::try_from(d.as_micros()).unwrap_or(u64::MAX);
        self.count.fetch_add(1, Ordering::Relaxed);
        self.total_us.fetch_add(us, Ordering::Relaxed);
        self.max_us.fetch_max(us, Ordering::Relaxed) < us
    }

    fn snapshot(&self) -> LatencySnapshot {
        let count = self.count.load(Ordering::Relaxed);
        LatencySnapshot {
            count,
            mean_us: self
                .total_us
                .load(Ordering::Relaxed)
                .checked_div(count)
                .unwrap_or(0),
            max_us: self.max_us.load(Ordering::Relaxed),
        }
    }
}

/// Um passo de leitura. Separado da thread para o teste dirigir o tempo e a fonte.
pub struct InputRunner<S: GamepadSource> {
    source: S,
    engine: InputEngine,
    sink: Arc<dyn InputSink>,
    control: Arc<InputControl>,
    clock: Arc<dyn Clock>,
    last_connected: Option<u8>,
}

impl<S: GamepadSource> InputRunner<S> {
    pub fn new(
        source: S,
        config: InputConfig,
        sink: Arc<dyn InputSink>,
        control: Arc<InputControl>,
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self {
            source,
            engine: InputEngine::new(config),
            sink,
            control,
            clock,
            last_connected: None,
        }
    }

    /// Le, decide e entrega. Devolve o modo vigente (a thread escolhe o periodo por ele).
    pub fn step(&mut self) -> InputMode {
        let wanted = self.control.mode();
        if self.engine.mode() != wanted {
            self.engine.set_mode(wanted);
        }
        let started = Instant::now();
        let now = self.clock.now();
        let pads = self.source.read(now);
        let poll = self.engine.poll(now, &pads);

        if self.last_connected != Some(poll.connected) {
            self.last_connected = Some(poll.connected);
            self.control
                .connected
                .store(poll.connected, Ordering::Release);
            self.sink.controllers(poll.connected);
        }
        for intent in &poll.intents {
            self.sink.intent(*intent);
        }
        if !poll.intents.is_empty() {
            let took = started.elapsed();
            if self.control.stats.record(took) && took > Duration::from_millis(5) {
                tracing::info!(
                    us = took.as_micros() as u64,
                    "entrada: nova maxima de latencia leitura -> emit"
                );
            }
        }
        if poll.combo_fired {
            tracing::info!("combinacao de retorno detectada");
            self.sink.combo_fired();
        }
        self.engine.mode()
    }
}

/// Resolucao do timer do Windows em 1 ms enquanto durar (o padrao de ~15,6 ms nao da 120 Hz).
struct HighResTimer;

impl HighResTimer {
    fn begin() -> Self {
        // SAFETY: chamada Win32 sem ponteiros; pareada com `timeEndPeriod` no `Drop`.
        unsafe {
            timeBeginPeriod(1);
        }
        Self
    }
}

impl Drop for HighResTimer {
    fn drop(&mut self) {
        // SAFETY: mesmo periodo passado em `timeBeginPeriod`.
        unsafe {
            timeEndPeriod(1);
        }
    }
}

/// Thread de entrada. Larga-la para o laco e espera a thread.
pub struct InputThread {
    stop: Arc<AtomicBool>,
    handle: Option<JoinHandle<()>>,
}

impl InputThread {
    pub fn spawn<S: GamepadSource + 'static>(
        runner: InputRunner<S>,
    ) -> std::io::Result<InputThread> {
        let stop = Arc::new(AtomicBool::new(false));
        let flag = Arc::clone(&stop);
        let handle = std::thread::Builder::new()
            .name("input".into())
            .spawn(move || run_loop(runner, &flag))?;
        Ok(InputThread {
            stop,
            handle: Some(handle),
        })
    }
}

impl Drop for InputThread {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
        if let Some(h) = self.handle.take() {
            let _ = h.join();
        }
    }
}

fn run_loop<S: GamepadSource>(mut runner: InputRunner<S>, stop: &AtomicBool) {
    let mut timer: Option<HighResTimer> = None;
    let mut next = Instant::now();
    while !stop.load(Ordering::Acquire) {
        let mode = runner.step();
        // Timer de 1 ms so com o Sofa na frente (em segundo plano 20 Hz nao precisa).
        match (mode, timer.is_some()) {
            (InputMode::Foreground, false) => timer = Some(HighResTimer::begin()),
            (InputMode::Foreground, true) => {}
            (_, true) => timer = None,
            (_, false) => {}
        }
        let period = if mode == InputMode::Foreground {
            FOREGROUND_PERIOD
        } else {
            BACKGROUND_PERIOD
        };
        next += period;
        let now = Instant::now();
        if next > now {
            std::thread::sleep(next - now);
        } else {
            next = now; // atrasou: nao tenta recuperar em rajada
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use launcher_core::application::ports::ManualClock;
    use launcher_core::domain::input::{Buttons, InputIntent, IntentEvent, PadSample};
    use launcher_core::domain::time::Millis;
    use std::sync::Mutex;

    #[derive(Default)]
    struct Sink {
        intents: Mutex<Vec<IntentEvent>>,
        controllers: Mutex<Vec<u8>>,
        combos: Mutex<u32>,
    }
    impl InputSink for Sink {
        fn intent(&self, e: IntentEvent) {
            self.intents.lock().unwrap().push(e);
        }
        fn controllers(&self, n: u8) {
            self.controllers.lock().unwrap().push(n);
        }
        fn combo_fired(&self) {
            *self.combos.lock().unwrap() += 1;
        }
    }

    /// Fonte roteirizada: devolve `pad` em todos os passos.
    struct Script(Arc<Mutex<PadSample>>);
    impl GamepadSource for Script {
        fn read(&mut self, _now: Millis) -> [PadSample; 4] {
            [
                *self.0.lock().unwrap(),
                PadSample::default(),
                PadSample::default(),
                PadSample::default(),
            ]
        }
    }

    struct Rig {
        runner: InputRunner<Script>,
        pad: Arc<Mutex<PadSample>>,
        sink: Arc<Sink>,
        control: Arc<InputControl>,
        clock: Arc<ManualClock>,
    }

    fn rig() -> Rig {
        let pad = Arc::new(Mutex::new(PadSample::default()));
        let sink = Arc::new(Sink::default());
        let control = Arc::new(InputControl::default());
        let clock = Arc::new(ManualClock::new());
        let runner = InputRunner::new(
            Script(pad.clone()),
            InputConfig::default(),
            sink.clone(),
            control.clone(),
            clock.clone(),
        );
        Rig {
            runner,
            pad,
            sink,
            control,
            clock,
        }
    }

    #[test]
    fn conexao_e_desconexao_sao_emitidas_uma_vez_por_mudanca() {
        let mut r = rig();
        r.runner.step();
        assert_eq!(
            *r.sink.controllers.lock().unwrap(),
            vec![0],
            "estado inicial"
        );
        *r.pad.lock().unwrap() = PadSample::connected();
        r.runner.step();
        r.runner.step();
        assert_eq!(*r.sink.controllers.lock().unwrap(), vec![0, 1]);
        assert_eq!(r.control.connected(), 1);
        *r.pad.lock().unwrap() = PadSample::default();
        r.runner.step();
        assert_eq!(*r.sink.controllers.lock().unwrap(), vec![0, 1, 0]);
        assert_eq!(r.control.connected(), 0);
    }

    #[test]
    fn botao_vira_intencao_e_a_latencia_do_passo_e_registrada() {
        let mut r = rig();
        *r.pad.lock().unwrap() = PadSample {
            connected: true,
            buttons: Buttons::A,
            ..PadSample::default()
        };
        r.runner.step();
        assert_eq!(
            *r.sink.intents.lock().unwrap(),
            vec![IntentEvent {
                intent: InputIntent::Confirm,
                repeat: false
            }]
        );
        let s = r.control.stats();
        assert_eq!(s.count, 1);
        assert!(
            s.max_us < 50_000,
            "leitura -> emit levou {} us (meta total 50 ms)",
            s.max_us
        );
    }

    #[test]
    fn segundo_plano_so_detecta_a_combinacao_de_retorno() {
        let mut r = rig();
        r.control.set_mode(InputMode::Background);
        *r.pad.lock().unwrap() = PadSample {
            connected: true,
            buttons: Buttons::A | Buttons::VIEW | Buttons::MENU,
            ..PadSample::default()
        };
        for _ in 0..40 {
            r.clock.advance_ms(50);
            r.runner.step();
        }
        assert!(
            r.sink.intents.lock().unwrap().is_empty(),
            "CA-12: nada para a UI em segundo plano"
        );
        assert_eq!(
            *r.sink.combos.lock().unwrap(),
            1,
            "uma vez por pressao (1,5 s)"
        );
    }

    #[test]
    fn modo_desligado_nao_emite_nada() {
        let mut r = rig();
        r.control.set_mode(InputMode::Off);
        *r.pad.lock().unwrap() = PadSample {
            connected: true,
            buttons: Buttons::A,
            ..PadSample::default()
        };
        assert_eq!(r.runner.step(), InputMode::Off);
        assert!(r.sink.intents.lock().unwrap().is_empty());
    }

    /// CPU da thread em segundo plano (RN-21): 20 Hz por 5 s com fonte fake. Medida e registrada
    /// na saida; o criterio e < 0,3% do total (meta interna). Rode com `--ignored --nocapture`.
    #[test]
    #[ignore = "mede CPU por ~5 s"]
    fn cpu_da_thread_em_segundo_plano() {
        use windows::Win32::Foundation::FILETIME;
        use windows::Win32::System::Threading::{GetCurrentProcess, GetProcessTimes};
        fn cpu_s() -> f64 {
            let (mut c, mut e, mut k, mut u) = (
                FILETIME::default(),
                FILETIME::default(),
                FILETIME::default(),
                FILETIME::default(),
            );
            // SAFETY: ponteiros para locais validos.
            unsafe {
                let _ = GetProcessTimes(GetCurrentProcess(), &mut c, &mut e, &mut k, &mut u);
            }
            let f = |t: FILETIME| {
                ((u64::from(t.dwHighDateTime) << 32) | u64::from(t.dwLowDateTime)) as f64 / 1e7
            };
            f(k) + f(u)
        }
        let r = rig();
        r.control.set_mode(InputMode::Background);
        let clock: Arc<dyn Clock> = Arc::new(launcher_win_clock());
        let runner = InputRunner::new(
            Script(r.pad.clone()),
            InputConfig::default(),
            r.sink.clone(),
            r.control.clone(),
            clock,
        );
        let before = cpu_s();
        let t0 = Instant::now();
        let thread = InputThread::spawn(runner).unwrap();
        std::thread::sleep(Duration::from_secs(5));
        drop(thread);
        let cpu_pct = (cpu_s() - before) / t0.elapsed().as_secs_f64() * 100.0;
        let logical = std::thread::available_parallelism().map_or(1, usize::from) as f64;
        println!("CPU da thread de entrada a 20 Hz: {cpu_pct:.3}% de 1 nucleo ({:.4}% do total, {logical} logicos)", cpu_pct / logical);
        assert!(cpu_pct / logical < 0.3);
    }

    fn launcher_win_clock() -> crate::system_clock::SystemClock {
        crate::system_clock::SystemClock::new()
    }
}
