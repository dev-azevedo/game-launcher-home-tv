//! `ForegroundBroker` (BE-18, ADR-0003, SEC-28 rev. 5): escada de tentativas para trazer o Sofa
//! (ou o jogo) para a frente, com verificacao a cada ~5 ms ate `verifyDelayMs`.
//!
//! A ordem padrao e 1 -> 3 -> 2 -> 4 (`advanced.foreground.order`); cada degrau liga e desliga em
//! `settings.json`. Condicoes de SEC-28 que valem aqui, independente da configuracao:
//! - estacao bloqueada ou desktop seguro: a escada **nao roda**;
//! - janela em primeiro plano elevada (UIPI): vai direto ao degrau 4, com log;
//! - degrau 2: no maximo **um** par de tecla por tentativa, e so depois de o degrau 3 falhar
//!   (ou se o degrau 3 estiver desligado);
//! - degrau 3: uma unica thread pendente (decidido pelo `WindowSystem`; `Pending` segue ao 2).
//!
//! So o `ForegroundBroker` chama o `WindowSystem`; nenhum comando IPC dispara a escada.

use std::sync::Arc;

use crate::application::ports::{Clock, Step3Result, WindowSystem};
use crate::domain::session::FrontOutcome;
use crate::domain::settings::{ForegroundSettings, Step2Key};

/// Orcamento total da escada (M-04: <= 3 s).
pub const LADDER_BUDGET_MS: u64 = 3_000;
/// Tempo limite do degrau 3 (a thread do jogo pode estar travada).
pub const STEP3_TIMEOUT_MS: u64 = 400;
const VERIFY_POLL_MS: u64 = 5;

pub struct ForegroundBroker {
    ws: Arc<dyn WindowSystem>,
    clock: Arc<dyn Clock>,
    cfg: ForegroundSettings,
}

impl ForegroundBroker {
    pub fn new(ws: Arc<dyn WindowSystem>, clock: Arc<dyn Clock>, cfg: ForegroundSettings) -> Self {
        Self { ws, clock, cfg }
    }

    /// `AllowSetForegroundWindow(ASFW_ANY)` antes de iniciar um jogo (CA-27).
    pub fn allow_others(&self) {
        self.ws.allow_set_foreground_any();
    }

    /// Leva o Sofa para a frente. `game_pids` e o que o degrau 4 minimiza. A janela do Sofa ja
    /// deve estar visivel (WebView2 retomado); aqui ela ganha `TOPMOST` temporario.
    pub fn bring_launcher_front(&self, game_pids: &[u32]) -> FrontOutcome {
        let started = self.clock.now();
        if !self.ws.desktop_is_interactive() {
            tracing::warn!(
                "escada de primeiro plano nao roda: estacao bloqueada ou desktop seguro"
            );
            return FrontOutcome::Failed;
        }
        self.ws.prepare_launcher_topmost();
        if self.ws.launcher_is_foreground() {
            tracing::info!(
                tentativa = "ja_na_frente",
                ms = 0,
                "primeiro plano: Sofa ja estava na frente"
            );
            self.ws.release_topmost();
            return FrontOutcome::Front;
        }

        let elevated = self.ws.foreground_is_elevated();
        let steps = self.plan(elevated);
        if elevated {
            tracing::info!(
                "primeiro plano: janela em primeiro plano elevada (UIPI), indo direto ao degrau 4"
            );
        }
        let step3_in_play = self.cfg.step3 && !elevated;
        let mut step3_failed = false;

        for step in steps {
            let used = self.clock.now().since(started);
            if used >= LADDER_BUDGET_MS {
                tracing::warn!(ms = used, "escada de primeiro plano estourou o orcamento");
                break;
            }
            let step_started = self.clock.now();
            let mut note = "";
            match step {
                1 => self.ws.step1_set_foreground(),
                3 => {
                    let r = self.ws.step3_attach(STEP3_TIMEOUT_MS);
                    note = match r {
                        Step3Result::Done => "ok",
                        Step3Result::Failed => "falhou",
                        Step3Result::TimedOut => "tempo_limite",
                        Step3Result::Pending => "thread_pendente",
                    };
                }
                2 => {
                    // SEC-28: a tecla so entra depois de o degrau 3 falhar na mesma tentativa.
                    if step3_in_play && !step3_failed {
                        tracing::info!(degrau = 2, "degrau 2 adiado: o degrau 3 ainda nao falhou");
                        continue;
                    }
                    self.ws.step2_key_and_foreground(self.cfg.step2_key);
                    note = match self.cfg.step2_key {
                        Step2Key::F24 => "F24",
                        Step2Key::Alt => "Alt",
                    };
                }
                4 => {
                    let n = self.ws.step4_minimize_games(game_pids);
                    tracing::info!(minimizadas = n, "degrau 4: janelas do jogo minimizadas");
                }
                _ => continue,
            }
            let ok = self.verify();
            tracing::info!(
                degrau = step,
                ms = self.clock.now().since(step_started),
                total_ms = self.clock.now().since(started),
                resultado = if ok { "frente" } else { "nao" },
                detalhe = note,
                "primeiro plano: tentativa"
            );
            if ok {
                self.ws.release_topmost();
                return if step == 4 {
                    FrontOutcome::FrontAfterMinimize
                } else {
                    FrontOutcome::Front
                };
            }
            if step == 3 {
                step3_failed = true;
            }
        }
        tracing::warn!(
            ms = self.clock.now().since(started),
            "primeiro plano: nenhum degrau trouxe o Sofa para a frente"
        );
        FrontOutcome::Failed
    }

    /// "Voltar ao jogo" (CA-34): o Sofa ja esta na frente, entao chamar direto basta.
    pub fn give_front_to(&self, game_pids: &[u32]) -> bool {
        self.ws.release_topmost();
        if !self.ws.front_game(game_pids) {
            return false;
        }
        self.verify_with(|| self.ws.game_is_foreground(game_pids))
    }

    /// Degraus que rodam, na ordem configurada, so os ligados. Elevado: so o 4.
    fn plan(&self, elevated: bool) -> Vec<u8> {
        let enabled = |n: u8| match n {
            1 => self.cfg.step1,
            2 => self.cfg.step2,
            3 => self.cfg.step3,
            4 => self.cfg.step4,
            _ => false,
        };
        let mut out: Vec<u8> = self
            .cfg
            .order
            .iter()
            .copied()
            .filter(|n| enabled(*n))
            .collect();
        if elevated {
            out.retain(|n| *n == 4);
        }
        out
    }

    fn verify(&self) -> bool {
        self.verify_with(|| self.ws.launcher_is_foreground())
    }

    fn verify_with(&self, is_front: impl Fn() -> bool) -> bool {
        let delay = u64::from(self.cfg.verify_delay_ms.clamp(50, 500));
        let deadline = self.clock.now() + delay;
        loop {
            if is_front() {
                return true;
            }
            if self.clock.now() >= deadline {
                return false;
            }
            self.ws.sleep_ms(VERIFY_POLL_MS);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::ports::ManualClock;
    use crate::domain::settings::AdvancedSettings;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Mutex;

    /// `WindowSystem` fake: cada degrau tem um "vai funcionar?" e cada chamada fica no registro.
    struct Fake {
        clock: Arc<ManualClock>,
        log: Mutex<Vec<String>>,
        foreground: AtomicBool,
        game_front: AtomicBool,
        interactive: bool,
        elevated: bool,
        works: [bool; 5], // indice = degrau
        step3: Step3Result,
        already_front: bool,
    }

    impl Fake {
        fn new(clock: Arc<ManualClock>) -> Self {
            Self {
                clock,
                log: Mutex::new(Vec::new()),
                foreground: AtomicBool::new(false),
                game_front: AtomicBool::new(false),
                interactive: true,
                elevated: false,
                works: [false; 5],
                step3: Step3Result::Failed,
                already_front: false,
            }
        }
        fn note(&self, s: &str) {
            self.log.lock().unwrap().push(s.to_owned());
        }
        fn calls(&self) -> Vec<String> {
            self.log.lock().unwrap().clone()
        }
        fn count(&self, s: &str) -> usize {
            self.calls().iter().filter(|c| c.as_str() == s).count()
        }
    }

    impl WindowSystem for Fake {
        fn sleep_ms(&self, ms: u64) {
            self.clock.advance_ms(ms);
        }
        fn desktop_is_interactive(&self) -> bool {
            self.interactive
        }
        fn launcher_is_foreground(&self) -> bool {
            self.already_front || self.foreground.load(Ordering::SeqCst)
        }
        fn foreground_is_elevated(&self) -> bool {
            self.elevated
        }
        fn prepare_launcher_topmost(&self) {
            self.note("topmost");
        }
        fn release_topmost(&self) {
            self.note("release_topmost");
        }
        fn step1_set_foreground(&self) {
            self.note("step1");
            if self.works[1] {
                self.foreground.store(true, Ordering::SeqCst);
            }
        }
        fn step3_attach(&self, _timeout_ms: u64) -> Step3Result {
            self.note("step3");
            if self.works[3] {
                self.foreground.store(true, Ordering::SeqCst);
                Step3Result::Done
            } else {
                self.step3
            }
        }
        fn step2_key_and_foreground(&self, _key: Step2Key) {
            self.note("step2");
            if self.works[2] {
                self.foreground.store(true, Ordering::SeqCst);
            }
        }
        fn step4_minimize_games(&self, _pids: &[u32]) -> usize {
            self.note("step4");
            if self.works[4] {
                self.foreground.store(true, Ordering::SeqCst);
            }
            1
        }
        fn front_game(&self, _pids: &[u32]) -> bool {
            self.note("front_game");
            self.game_front.store(true, Ordering::SeqCst);
            true
        }
        fn game_is_foreground(&self, _pids: &[u32]) -> bool {
            self.game_front.load(Ordering::SeqCst)
        }
        fn allow_set_foreground_any(&self) {
            self.note("allow_any");
        }
    }

    fn broker(f: Fake, clock: &Arc<ManualClock>) -> (ForegroundBroker, Arc<Fake>) {
        broker_with(f, clock, AdvancedSettings::default().foreground)
    }

    fn broker_with(
        f: Fake,
        clock: &Arc<ManualClock>,
        cfg: ForegroundSettings,
    ) -> (ForegroundBroker, Arc<Fake>) {
        let f = Arc::new(f);
        let ws: Arc<dyn WindowSystem> = f.clone();
        let c: Arc<dyn Clock> = clock.clone();
        (ForegroundBroker::new(ws, c, cfg), f)
    }

    fn steps(f: &Fake) -> Vec<String> {
        f.calls()
            .into_iter()
            .filter(|c| c.starts_with("step"))
            .collect()
    }

    #[test]
    fn padrao_e_a_ordem_1_3_2_4_e_o_degrau_3_resolve_sem_mandar_tecla() {
        let clock = Arc::new(ManualClock::new());
        let mut f = Fake::new(clock.clone());
        f.works[3] = true;
        let (b, f) = broker(f, &clock);
        assert_eq!(b.bring_launcher_front(&[7]), FrontOutcome::Front);
        assert_eq!(steps(&f), ["step1", "step3"]);
        assert_eq!(
            f.count("step2"),
            0,
            "o jogo nao recebe tecla se o 3 resolve"
        );
    }

    #[test]
    fn recusa_do_primeiro_plano_chega_ao_degrau_4_em_menos_de_3_s() {
        let clock = Arc::new(ManualClock::new());
        let mut f = Fake::new(clock.clone());
        f.works[4] = true;
        let (b, f) = broker(f, &clock);
        let t0 = clock.now();
        assert_eq!(
            b.bring_launcher_front(&[7]),
            FrontOutcome::FrontAfterMinimize
        );
        assert_eq!(steps(&f), ["step1", "step3", "step2", "step4"]);
        assert!(
            clock.now().since(t0) < 3_000,
            "levou {} ms",
            clock.now().since(t0)
        );
    }

    #[test]
    fn tudo_falha_devolve_failed_dentro_do_orcamento() {
        let clock = Arc::new(ManualClock::new());
        let (b, f) = broker(Fake::new(clock.clone()), &clock);
        let t0 = clock.now();
        assert_eq!(b.bring_launcher_front(&[7]), FrontOutcome::Failed);
        assert_eq!(steps(&f), ["step1", "step3", "step2", "step4"]);
        assert!(clock.now().since(t0) < 3_000);
    }

    #[test]
    fn degrau_2_manda_um_unico_par_por_tentativa() {
        let clock = Arc::new(ManualClock::new());
        let (b, f) = broker(Fake::new(clock.clone()), &clock);
        b.bring_launcher_front(&[7]);
        assert_eq!(f.count("step2"), 1);
        // Outra tentativa e outra chance de um par; nunca dois na mesma.
        b.bring_launcher_front(&[7]);
        assert_eq!(f.count("step2"), 2);
    }

    #[test]
    fn degrau_2_so_depois_de_o_3_falhar_mesmo_com_ordem_trocada() {
        let clock = Arc::new(ManualClock::new());
        let mut cfg = AdvancedSettings::default().foreground;
        cfg.order = [2, 1, 3, 4];
        let mut f = Fake::new(clock.clone());
        f.works[3] = true;
        let (b, f) = broker_with(f, &clock, cfg);
        assert_eq!(b.bring_launcher_front(&[7]), FrontOutcome::Front);
        // O 2 vinha primeiro, foi adiado, e o 3 resolveu: nenhuma tecla.
        assert_eq!(f.count("step2"), 0);
    }

    #[test]
    fn degrau_2_roda_se_o_3_esta_desligado() {
        let clock = Arc::new(ManualClock::new());
        let mut cfg = AdvancedSettings::default().foreground;
        cfg.step3 = false;
        let mut f = Fake::new(clock.clone());
        f.works[2] = true;
        let (b, f) = broker_with(f, &clock, cfg);
        assert_eq!(b.bring_launcher_front(&[7]), FrontOutcome::Front);
        assert_eq!(steps(&f), ["step1", "step2"]);
    }

    #[test]
    fn thread_pendente_do_degrau_3_segue_para_o_2() {
        let clock = Arc::new(ManualClock::new());
        let mut f = Fake::new(clock.clone());
        f.step3 = Step3Result::Pending;
        f.works[2] = true;
        let (b, f) = broker(f, &clock);
        assert_eq!(b.bring_launcher_front(&[7]), FrontOutcome::Front);
        assert_eq!(steps(&f), ["step1", "step3", "step2"]);
    }

    #[test]
    fn tempo_limite_do_degrau_3_conta_como_falha_e_libera_o_2() {
        let clock = Arc::new(ManualClock::new());
        let mut f = Fake::new(clock.clone());
        f.step3 = Step3Result::TimedOut;
        f.works[2] = true;
        let (b, f) = broker(f, &clock);
        assert_eq!(b.bring_launcher_front(&[7]), FrontOutcome::Front);
        assert_eq!(steps(&f), ["step1", "step3", "step2"]);
    }

    #[test]
    fn jogo_elevado_vai_direto_ao_degrau_4() {
        let clock = Arc::new(ManualClock::new());
        let mut f = Fake::new(clock.clone());
        f.elevated = true;
        f.works[4] = true;
        let (b, f) = broker(f, &clock);
        assert_eq!(
            b.bring_launcher_front(&[7]),
            FrontOutcome::FrontAfterMinimize
        );
        assert_eq!(steps(&f), ["step4"], "nenhum degrau 1/2/3 contra UIPI");
    }

    #[test]
    fn estacao_bloqueada_ou_desktop_seguro_nao_roda_nada() {
        let clock = Arc::new(ManualClock::new());
        let mut f = Fake::new(clock.clone());
        f.interactive = false;
        let (b, f) = broker(f, &clock);
        assert_eq!(b.bring_launcher_front(&[7]), FrontOutcome::Failed);
        assert!(f.calls().is_empty(), "{:?}", f.calls());
    }

    #[test]
    fn degraus_desligados_nao_rodam_e_a_ordem_e_respeitada() {
        let clock = Arc::new(ManualClock::new());
        let mut cfg = AdvancedSettings::default().foreground;
        cfg.step2 = false;
        cfg.step3 = false; // cenario D
        let (b, f) = broker_with(Fake::new(clock.clone()), &clock, cfg);
        b.bring_launcher_front(&[7]);
        assert_eq!(steps(&f), ["step1", "step4"]);
    }

    #[test]
    fn ja_na_frente_nao_tenta_nada() {
        let clock = Arc::new(ManualClock::new());
        let mut f = Fake::new(clock.clone());
        f.already_front = true;
        let (b, f) = broker(f, &clock);
        assert_eq!(b.bring_launcher_front(&[7]), FrontOutcome::Front);
        assert!(steps(&f).is_empty());
    }

    #[test]
    fn degrau_1_basta_quando_o_sofa_herdou_o_direito() {
        let clock = Arc::new(ManualClock::new());
        let mut f = Fake::new(clock.clone());
        f.works[1] = true;
        let (b, f) = broker(f, &clock);
        assert_eq!(b.bring_launcher_front(&[7]), FrontOutcome::Front);
        assert_eq!(steps(&f), ["step1"]);
    }

    #[test]
    fn voltar_ao_jogo_restaura_e_confere() {
        let clock = Arc::new(ManualClock::new());
        let (b, f) = broker(Fake::new(clock.clone()), &clock);
        assert!(b.give_front_to(&[7]));
        assert_eq!(f.count("front_game"), 1);
    }

    #[test]
    fn allow_others_so_chama_o_asfw() {
        let clock = Arc::new(ManualClock::new());
        let (b, f) = broker(Fake::new(clock.clone()), &clock);
        b.allow_others();
        assert_eq!(f.calls(), ["allow_any"]);
    }
}
