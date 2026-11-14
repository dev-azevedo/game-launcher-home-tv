//! Casos de uso da sessao (BE-20, BE-21): iniciar, trocar, fechar, forcar, voltar ao jogo e a
//! combinacao de retorno. Orquestra a maquina de estados (`domain::session`) e os ports; nao
//! conhece Win32 nem Tauri (INV-01).
//!
//! Regras de concorrencia: as acoes serializadas do contrato (`session_launch`,
//! `session_launch_with_uac`, `session_replace`, `session_retry`) passam pela `SerialGate` na
//! apresentacao; aqui o estado fica atras de um unico `Mutex` curto. Nenhuma chamada demorada
//! (`GameStarter::start`, a escada de primeiro plano) roda com o `Mutex` preso.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError, Weak};

use crate::application::foreground::ForegroundBroker;
use crate::application::ports::{
    Clock, GameResolver, GameStarter, InputGate, LauncherWindow, PlayedRecorder, ProbeConfig,
    ProbeSink, SessionEvents, SessionProbe, SessionWatch, StartError, StartVerb, Started,
    TaskRunner, WatchPlan,
};
use crate::application::AppError;
use crate::domain::input::InputMode;
use crate::domain::library::{GameId, LaunchSpec, LaunchTarget};
use crate::domain::session::{
    FrontOutcome, Session, SessionConfig, SessionEffect, SessionSnapshot, SessionState,
};

/// Fotografia da sessao para a UI: o snapshot do dominio mais o instante de entrada no estado em
/// ms desde a epoca Unix (o dominio so conhece o relogio monotonico).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionView {
    pub snapshot: SessionSnapshot,
    pub since_unix_ms: Option<i64>,
}

/// Tudo que o servico usa, por injecao.
pub struct SessionDeps {
    pub clock: Arc<dyn Clock>,
    pub resolver: Arc<dyn GameResolver>,
    pub recorder: Arc<dyn PlayedRecorder>,
    pub starter: Arc<dyn GameStarter>,
    pub probe: Arc<dyn SessionProbe>,
    pub foreground: Arc<ForegroundBroker>,
    pub window: Arc<dyn LauncherWindow>,
    pub input: Arc<dyn InputGate>,
    pub events: Arc<dyn SessionEvents>,
    pub runner: Arc<dyn TaskRunner>,
}

/// Exigencia de AF-01: o 740 de **este** jogo e a identidade do arquivo naquele momento.
struct Elevation {
    game: GameId,
    fingerprint: Option<String>,
}

struct Inner {
    session: Session,
    /// Identifica a sessao vigiada; aviso de sonda antiga e ignorado.
    generation: u64,
    watch: Option<Box<dyn SessionWatch>>,
    elevation: Option<Elevation>,
    last_emitted: Option<SessionSnapshot>,
}

pub struct SessionService {
    deps: SessionDeps,
    probe_cfg: ProbeConfig,
    inner: Mutex<Inner>,
    me: Weak<SessionService>,
    return_in_flight: AtomicBool,
}

fn map_start_error(e: &StartError, target: &LaunchTarget) -> AppError {
    let game_id = target.game.clone();
    match e {
        StartError::NeedsElevation { .. } => AppError::NeedsElevation { game_id },
        StartError::ElevationDenied => AppError::ElevationDenied { game_id },
        StartError::FileMissing => AppError::GameFileMissing { game_id },
        StartError::NetworkPath => AppError::NetworkPathBlocked,
        StartError::StoreNotInstalled => AppError::StoreNotInstalled {
            origin: target.game.origin(),
        },
        StartError::Failed => AppError::Internal,
    }
}

impl SessionService {
    pub fn new(deps: SessionDeps, cfg: SessionConfig, probe_cfg: ProbeConfig) -> Arc<Self> {
        let session = Session::new(cfg);
        Arc::new_cyclic(|me| Self {
            deps,
            probe_cfg,
            inner: Mutex::new(Inner {
                // O `Idle` inicial ja e o que a UI le por `session_get`: nao e evento.
                last_emitted: Some(session.snapshot()),
                session,
                generation: 0,
                watch: None,
                elevation: None,
            }),
            me: me.clone(),
            return_in_flight: AtomicBool::new(false),
        })
    }

    fn lock(&self) -> MutexGuard<'_, Inner> {
        self.inner.lock().unwrap_or_else(PoisonError::into_inner)
    }

    fn view_of(&self, snap: SessionSnapshot) -> SessionView {
        let since_unix_ms = snap.since.map(|s| {
            let age = i64::try_from(self.deps.clock.now().since(s)).unwrap_or(i64::MAX);
            self.deps.clock.unix_ms().saturating_sub(age)
        });
        SessionView {
            snapshot: snap,
            since_unix_ms,
        }
    }

    /// `session_get`.
    pub fn get(&self) -> SessionView {
        let snap = self.lock().session.snapshot();
        self.view_of(snap)
    }

    /// O temporizador so precisa rodar com sessao (prazos de 60 s, 5 s e 10 s).
    pub fn needs_tick(&self) -> bool {
        self.lock().session.state() != SessionState::Idle
    }

    /// Emite `session://state` se algo mudou desde o ultimo evento.
    fn emit(&self, g: &mut Inner) {
        let snap = g.session.snapshot();
        if g.last_emitted.as_ref() == Some(&snap) {
            return;
        }
        g.last_emitted = Some(snap.clone());
        let view = self.view_of(snap);
        self.deps.events.state_changed(&view);
    }

    fn pids(&self) -> Vec<u32> {
        self.lock()
            .watch
            .as_ref()
            .map(|w| w.pids())
            .unwrap_or_default()
    }

    // ------------------------------------------------------------------ iniciar (BE-20)

    fn ensure_idle(&self) -> Result<(), AppError> {
        let g = self.lock();
        match g.session.game() {
            Some(current) if g.session.state() != SessionState::Idle => {
                Err(AppError::SessionAlreadyActive {
                    game_id: current.clone(),
                })
            }
            _ => Ok(()),
        }
    }

    fn preflight(&self, target: &LaunchTarget) -> Result<(), AppError> {
        self.deps
            .starter
            .preflight(target)
            .map_err(|e| map_start_error(&e, target))
    }

    /// `session_launch`. Revalida o jogo no nucleo (SEC-09/12/14) antes de mexer na sessao.
    pub fn launch(&self, id: GameId) -> Result<(), AppError> {
        self.ensure_idle()?;
        let target = self.deps.resolver.resolve(&id)?;
        self.preflight(&target)?;
        {
            let mut g = self.lock();
            let now = self.deps.clock.now();
            g.session.launch(id, now)?;
            g.generation += 1;
            g.elevation = None;
            self.emit(&mut g);
        }
        self.start(&target, StartVerb::Normal)
    }

    /// `session_launch_with_uac` (AF-01): so em `NeedsElevation`, para o mesmo jogo, depois do
    /// 740 dele nesta sessao, com o caminho revalidado agora e o arquivo igual ao do 740.
    pub fn launch_with_uac(&self, id: GameId) -> Result<(), AppError> {
        let expected = {
            let g = self.lock();
            if g.session.state() != SessionState::NeedsElevation || g.session.game() != Some(&id) {
                return Err(AppError::InvalidState);
            }
            match &g.elevation {
                Some(e) if e.game == id => e.fingerprint.clone(),
                _ => return Err(AppError::InvalidState),
            }
        };
        let target = self.deps.resolver.resolve(&id)?;
        self.preflight(&target)?;
        if self.deps.starter.fingerprint(&target) != expected {
            tracing::warn!(game = %id, "executavel mudou depois do erro 740; UAC recusado (AF-01)");
            return Err(AppError::InvalidState);
        }
        {
            let mut g = self.lock();
            let now = self.deps.clock.now();
            g.session.launch_with_uac(&id, now)?;
            g.generation += 1;
            self.emit(&mut g);
        }
        self.start(&target, StartVerb::Elevated)
    }

    /// `session_retry` ("Tentar de novo" em `NotConfirmed`): refaz o inicio do mesmo jogo.
    pub fn retry(&self) -> Result<(), AppError> {
        let id = {
            let g = self.lock();
            if g.session.state() != SessionState::NotConfirmed {
                return Err(AppError::InvalidState);
            }
            g.session.game().cloned().ok_or(AppError::InvalidState)?
        };
        let target = self.deps.resolver.resolve(&id)?;
        self.preflight(&target)?;
        {
            let mut g = self.lock();
            let now = self.deps.clock.now();
            g.session.retry(now)?;
            g.generation += 1;
            g.watch = None;
            self.emit(&mut g);
        }
        self.start(&target, StartVerb::Normal)
    }

    /// Inicia o jogo fora do `Mutex` (o UAC pode demorar) e liga a sonda.
    fn start(&self, target: &LaunchTarget, verb: StartVerb) -> Result<(), AppError> {
        self.deps.foreground.allow_others();
        let generation = self.lock().generation;
        match self.deps.starter.start(target, verb) {
            Ok(started) => self.attach(target, started, verb, generation),
            Err(StartError::NeedsElevation { fingerprint }) => {
                let mut g = self.lock();
                let now = self.deps.clock.now();
                g.session.elevation_required(now);
                g.elevation = Some(Elevation {
                    game: target.game.clone(),
                    fingerprint,
                });
                self.emit(&mut g);
                Err(AppError::NeedsElevation {
                    game_id: target.game.clone(),
                })
            }
            Err(StartError::ElevationDenied) => {
                let mut g = self.lock();
                let now = self.deps.clock.now();
                g.session.elevation_denied(now);
                self.emit(&mut g);
                Err(AppError::ElevationDenied {
                    game_id: target.game.clone(),
                })
            }
            Err(e) => {
                tracing::info!(game = %target.game, erro = ?e, "inicio do jogo falhou");
                let mut g = self.lock();
                let now = self.deps.clock.now();
                g.session.launch_failed(now);
                self.cleanup_if_idle(&mut g);
                self.emit(&mut g);
                Err(map_start_error(&e, target))
            }
        }
    }

    fn attach(
        &self,
        target: &LaunchTarget,
        started: Started,
        verb: StartVerb,
        generation: u64,
    ) -> Result<(), AppError> {
        let (scan_dir, steam_app_id) = match &target.spec {
            LaunchSpec::SteamUri { app_id } => (target.install_dir.clone(), Some(*app_id)),
            LaunchSpec::EpicUri { .. } => (target.install_dir.clone(), None),
            // Jogo elevado fica fora do Job (UIPI): so varredura por pasta (ADR-0005).
            LaunchSpec::Exe { .. } if verb == StartVerb::Elevated || started.job.is_none() => {
                (target.install_dir.clone(), None)
            }
            LaunchSpec::Exe { .. } => (None, None),
        };
        let plan = WatchPlan {
            game: target.game.clone(),
            job: started.job,
            scan_dir,
            steam_app_id,
        };
        let sink: Arc<dyn ProbeSink> = self.me.upgrade().ok_or(AppError::Internal)?;
        match self
            .deps
            .probe
            .watch(plan, generation, sink, self.probe_cfg)
        {
            Ok(watch) => {
                let mut g = self.lock();
                if g.generation == generation && g.session.state() != SessionState::Idle {
                    g.watch = Some(watch);
                }
                Ok(())
            }
            Err(e) => {
                tracing::error!(game = %target.game, "nao conseguiu vigiar o jogo");
                let mut g = self.lock();
                let now = self.deps.clock.now();
                g.session.launch_failed(now);
                self.cleanup_if_idle(&mut g);
                self.emit(&mut g);
                Err(e)
            }
        }
    }

    /// `session_replace` (CT-01): so `Running`; valida o novo jogo ANTES e so pede o fechamento
    /// normal (`WM_CLOSE`). O novo abre quando a sessao chegar a `Idle`.
    pub fn replace(&self, new_game: GameId) -> Result<(), AppError> {
        if self.lock().session.state() != SessionState::Running {
            return Err(AppError::InvalidState);
        }
        let target = self.deps.resolver.resolve(&new_game)?;
        self.preflight(&target)?;
        let mut g = self.lock();
        let now = self.deps.clock.now();
        g.session.replace(new_game, now)?;
        self.request_close_windows(&g);
        self.emit(&mut g);
        Ok(())
    }

    // ----------------------------------------------------------- retorno e fim (BE-21)

    fn request_close_windows(&self, g: &Inner) {
        if let Some(w) = &g.watch {
            if let Err(e) = w.request_close() {
                // Jogo elevado (UIPI) ou janela ausente: o prazo de 10 s leva a CloseOverdue.
                tracing::warn!(codigo = e.code(), "pedido de fechamento nao entregue");
            }
        }
    }

    /// `session_close`: pedido normal de fechamento (`WM_CLOSE`).
    pub fn close(&self) -> Result<(), AppError> {
        let mut g = self.lock();
        let now = self.deps.clock.now();
        g.session.request_close(now)?;
        self.request_close_windows(&g);
        self.emit(&mut g);
        Ok(())
    }

    /// `session_force_close`: so em `CloseOverdue`, depois da confirmacao da UI (RN-12, INV-05).
    pub fn force_close(&self) -> Result<(), AppError> {
        let g = self.lock();
        g.session.clone().force_close()?;
        match &g.watch {
            Some(w) => w.force_close(),
            None => Err(AppError::NoActiveSession),
        }
    }

    /// `session_return_to_game` (CA-34): o Sofa esta na frente; devolve o foco ao jogo e some.
    pub fn return_to_game(&self) -> Result<(), AppError> {
        let pids = {
            let g = self.lock();
            g.session.return_to_game()?;
            g.watch.as_ref().map(|w| w.pids()).unwrap_or_default()
        };
        if !self.deps.foreground.give_front_to(&pids) {
            return Err(AppError::ForegroundDenied);
        }
        self.deps.window.hide_and_suspend();
        self.deps.input.set_mode(InputMode::Background);
        let mut g = self.lock();
        g.session.set_minimized(false);
        self.emit(&mut g);
        Ok(())
    }

    /// `session_dismiss` (CA-77, CA-78): nunca cancela inicio nem fechamento em andamento.
    pub fn dismiss(&self) {
        let mut g = self.lock();
        let now = self.deps.clock.now();
        let effects = g.session.dismiss(now);
        self.apply(&mut g, effects, SessionState::Idle);
        self.emit(&mut g);
    }

    /// Combinacao de retorno (CA-32, M-04): mostra o Sofa, roda a escada fora da thread de quem
    /// chamou e so entao abre o painel e devolve a entrada ao Sofa.
    pub fn handle_return_combo(&self) {
        let pids = {
            let g = self.lock();
            match g.session.state() {
                SessionState::Running
                | SessionState::Ending
                | SessionState::Closing
                | SessionState::CloseOverdue => g.watch.as_ref().map(|w| w.pids()),
                _ => None,
            }
        };
        let Some(pids) = pids else { return };
        if self.return_in_flight.swap(true, Ordering::AcqRel) {
            return; // uma tentativa por vez
        }
        let Some(me) = self.me.upgrade() else {
            self.return_in_flight.store(false, Ordering::Release);
            return;
        };
        self.deps.window.show_and_resume();
        self.deps.runner.run(
            "return-combo",
            Box::new(move || {
                let outcome = me.deps.foreground.bring_launcher_front(&pids);
                {
                    let mut g = me.lock();
                    g.session
                        .set_minimized(outcome == FrontOutcome::FrontAfterMinimize);
                    me.emit(&mut g);
                }
                me.deps.input.set_mode(InputMode::Foreground);
                me.deps.events.panel_opened(outcome);
                me.return_in_flight.store(false, Ordering::Release);
            }),
        );
    }

    // ------------------------------------------------------ sinais da infraestrutura

    /// Avanca os prazos (chamado por um temporizador enquanto `needs_tick`).
    pub fn tick(&self) {
        let mut g = self.lock();
        let before = g.session.state();
        let now = self.deps.clock.now();
        let effects = g.session.tick(now);
        self.apply(&mut g, effects, before);
        self.emit(&mut g);
    }

    fn apply(&self, g: &mut Inner, effects: Vec<SessionEffect>, before: SessionState) {
        let mut focus = None;
        let mut pending = None;
        for effect in effects {
            match effect {
                SessionEffect::MarkPlayed(id) => {
                    self.deps
                        .recorder
                        .mark_played(&id, self.deps.clock.unix_ms());
                }
                SessionEffect::BringLauncherFront { focus: f } => focus = Some(f),
                SessionEffect::LaunchPending(next) => pending = Some(next),
                // Executados pelos proprios comandos, fora do `Mutex`.
                SessionEffect::StartGame(_)
                | SessionEffect::StartGameWithUac(_)
                | SessionEffect::RequestClose
                | SessionEffect::ForceClose
                | SessionEffect::FrontGame => {}
            }
        }
        if matches!(before, SessionState::Launching | SessionState::NotConfirmed)
            && g.session.state() == SessionState::Running
        {
            self.on_running();
        }
        if let Some(focus) = focus {
            self.on_ended(focus, pending);
        }
        if g.session.state() == SessionState::Idle {
            self.cleanup_if_idle(g);
        }
    }

    /// O jogo abriu: a UI sai da frente, o WebView2 dorme e a entrada vira "so a combinacao".
    fn on_running(&self) {
        self.deps.window.hide_and_suspend();
        self.deps.input.set_mode(InputMode::Background);
        if let Some(me) = self.me.upgrade() {
            self.deps.runner.run(
                "give-front-to-game",
                Box::new(move || {
                    let pids = me.pids();
                    let _ = me.deps.foreground.give_front_to(&pids);
                }),
            );
        }
    }

    /// A sessao acabou. Troca de jogo (`pending`): abre o novo pelo fluxo normal. Senao, traz o
    /// Sofa (mesma escada da combinacao, CA-30) e avisa a UI com o jogo em foco.
    fn on_ended(&self, focus: GameId, pending: Option<GameId>) {
        let Some(me) = self.me.upgrade() else { return };
        if let Some(next) = pending {
            self.deps.runner.run(
                "launch-pending",
                Box::new(move || {
                    if let Err(e) = me.launch(next.clone()) {
                        tracing::warn!(game = %next, codigo = e.code(), "troca de jogo: novo jogo nao iniciou");
                    }
                }),
            );
            return;
        }
        self.deps.window.show_and_resume();
        self.deps.input.set_mode(InputMode::Foreground);
        self.deps.runner.run(
            "bring-launcher-front",
            Box::new(move || {
                let outcome = me.deps.foreground.bring_launcher_front(&[]);
                tracing::info!(resultado = ?outcome, "fim de sessao: Sofa trazido para a frente");
                me.deps.events.game_ended(&focus);
            }),
        );
    }

    fn cleanup_if_idle(&self, g: &mut Inner) {
        if g.session.state() == SessionState::Idle {
            g.watch = None;
            g.elevation = None;
        }
    }
}

impl ProbeSink for SessionService {
    fn process_seen(&self, generation: u64) {
        let mut g = self.lock();
        if g.generation != generation {
            return;
        }
        let before = g.session.state();
        let now = self.deps.clock.now();
        let effects = g.session.process_seen(now);
        self.apply(&mut g, effects, before);
        self.emit(&mut g);
    }

    fn processes_gone(&self, generation: u64) {
        let mut g = self.lock();
        if g.generation != generation {
            return;
        }
        let before = g.session.state();
        let now = self.deps.clock.now();
        let effects = g.session.processes_gone(now);
        self.apply(&mut g, effects, before);
        self.emit(&mut g);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::ports::{ManualClock, Step3Result, WindowSystem};
    use crate::domain::library::Origin;
    use crate::domain::settings::{AdvancedSettings, Step2Key};
    use std::collections::{HashMap, VecDeque};

    // ------------------------------------------------------------------ fakes

    struct Resolver(HashMap<GameId, Result<LaunchTarget, AppError>>);
    impl GameResolver for Resolver {
        fn resolve(&self, id: &GameId) -> Result<LaunchTarget, AppError> {
            self.0
                .get(id)
                .cloned()
                .unwrap_or(Err(AppError::UnknownGame))
        }
    }

    #[derive(Default)]
    struct Recorder(Mutex<Vec<(GameId, i64)>>);
    impl PlayedRecorder for Recorder {
        fn mark_played(&self, id: &GameId, at: i64) {
            self.0.lock().unwrap().push((id.clone(), at));
        }
    }

    #[derive(Default)]
    struct Starter {
        results: Mutex<VecDeque<Result<(), StartError>>>,
        starts: Mutex<Vec<(GameId, StartVerb)>>,
        fingerprint: Mutex<Option<String>>,
        preflight_err: Mutex<Option<StartError>>,
    }
    impl GameStarter for Starter {
        fn preflight(&self, _t: &LaunchTarget) -> Result<(), StartError> {
            match self.preflight_err.lock().unwrap().clone() {
                Some(e) => Err(e),
                None => Ok(()),
            }
        }
        fn start(&self, t: &LaunchTarget, verb: StartVerb) -> Result<Started, StartError> {
            self.starts.lock().unwrap().push((t.game.clone(), verb));
            match self.results.lock().unwrap().pop_front() {
                Some(Err(e)) => Err(e),
                _ => Ok(Started::default()),
            }
        }
        fn fingerprint(&self, _t: &LaunchTarget) -> Option<String> {
            self.fingerprint.lock().unwrap().clone()
        }
    }

    #[derive(Default)]
    struct WatchState {
        close_calls: u32,
        force_calls: u32,
        force_result: Option<AppError>,
    }
    struct Watch(Arc<Mutex<WatchState>>);
    impl SessionWatch for Watch {
        fn pids(&self) -> Vec<u32> {
            vec![1234]
        }
        fn request_close(&self) -> Result<(), AppError> {
            self.0.lock().unwrap().close_calls += 1;
            Ok(())
        }
        fn force_close(&self) -> Result<(), AppError> {
            let mut s = self.0.lock().unwrap();
            s.force_calls += 1;
            s.force_result.clone().map_or(Ok(()), Err)
        }
    }

    #[derive(Default)]
    struct Probe {
        watch: Arc<Mutex<WatchState>>,
        calls: Mutex<Vec<(u64, WatchPlanInfo)>>,
        sink: Mutex<Option<Arc<dyn ProbeSink>>>,
    }
    #[derive(Debug, Clone, PartialEq, Eq)]
    struct WatchPlanInfo {
        has_job: bool,
        scan_dir: Option<String>,
        steam_app_id: Option<u32>,
    }
    impl SessionProbe for Probe {
        fn watch(
            &self,
            plan: WatchPlan,
            generation: u64,
            sink: Arc<dyn ProbeSink>,
            _c: ProbeConfig,
        ) -> Result<Box<dyn SessionWatch>, AppError> {
            self.calls.lock().unwrap().push((
                generation,
                WatchPlanInfo {
                    has_job: plan.job.is_some(),
                    scan_dir: plan.scan_dir,
                    steam_app_id: plan.steam_app_id,
                },
            ));
            *self.sink.lock().unwrap() = Some(sink);
            Ok(Box::new(Watch(self.watch.clone())))
        }
    }

    struct Ws {
        clock: Arc<ManualClock>,
        works: bool,
        step4_only: bool,
        log: Mutex<Vec<&'static str>>,
        fg: AtomicBool,
        game_fg: AtomicBool,
        front_game_ok: bool,
    }
    impl WindowSystem for Ws {
        fn sleep_ms(&self, ms: u64) {
            self.clock.advance_ms(ms);
        }
        fn desktop_is_interactive(&self) -> bool {
            true
        }
        fn launcher_is_foreground(&self) -> bool {
            self.fg.load(Ordering::SeqCst)
        }
        fn foreground_is_elevated(&self) -> bool {
            false
        }
        fn prepare_launcher_topmost(&self) {}
        fn release_topmost(&self) {}
        fn step1_set_foreground(&self) {
            self.log.lock().unwrap().push("step1");
            if self.works {
                self.fg.store(true, Ordering::SeqCst);
            }
        }
        fn step3_attach(&self, _t: u64) -> Step3Result {
            Step3Result::Failed
        }
        fn step2_key_and_foreground(&self, _k: Step2Key) {}
        fn step4_minimize_games(&self, _p: &[u32]) -> usize {
            self.log.lock().unwrap().push("step4");
            self.fg
                .store(self.works || self.step4_only, Ordering::SeqCst);
            1
        }
        fn front_game(&self, _p: &[u32]) -> bool {
            self.game_fg.store(self.front_game_ok, Ordering::SeqCst);
            self.front_game_ok
        }
        fn game_is_foreground(&self, _p: &[u32]) -> bool {
            self.game_fg.load(Ordering::SeqCst)
        }
        fn allow_set_foreground_any(&self) {
            self.log.lock().unwrap().push("allow_any");
        }
    }

    #[derive(Default)]
    struct Window(Mutex<Vec<&'static str>>);
    impl LauncherWindow for Window {
        fn hide_and_suspend(&self) {
            self.0.lock().unwrap().push("hide");
        }
        fn show_and_resume(&self) {
            self.0.lock().unwrap().push("show");
        }
    }

    #[derive(Default)]
    struct Gate(Mutex<Vec<InputMode>>);
    impl InputGate for Gate {
        fn set_mode(&self, mode: InputMode) {
            self.0.lock().unwrap().push(mode);
        }
    }

    #[derive(Default)]
    struct Events {
        states: Mutex<Vec<SessionView>>,
        panels: Mutex<Vec<FrontOutcome>>,
        ended: Mutex<Vec<GameId>>,
    }
    impl SessionEvents for Events {
        fn state_changed(&self, view: &SessionView) {
            self.states.lock().unwrap().push(view.clone());
        }
        fn panel_opened(&self, outcome: FrontOutcome) {
            self.panels.lock().unwrap().push(outcome);
        }
        fn game_ended(&self, focus: &GameId) {
            self.ended.lock().unwrap().push(focus.clone());
        }
    }

    /// Fila que so executa quando o teste manda (a escada nao roda dentro do `Mutex`).
    #[derive(Default)]
    struct Queue(Mutex<VecDeque<Box<dyn FnOnce() + Send>>>);
    impl TaskRunner for Queue {
        fn run(&self, _n: &'static str, t: Box<dyn FnOnce() + Send>) {
            self.0.lock().unwrap().push_back(t);
        }
    }
    impl Queue {
        fn drain(&self) {
            loop {
                let t = self.0.lock().unwrap().pop_front();
                match t {
                    Some(t) => t(),
                    None => break,
                }
            }
        }
    }

    struct Rig {
        svc: Arc<SessionService>,
        clock: Arc<ManualClock>,
        starter: Arc<Starter>,
        probe: Arc<Probe>,
        recorder: Arc<Recorder>,
        ws: Arc<Ws>,
        window: Arc<Window>,
        gate: Arc<Gate>,
        events: Arc<Events>,
        queue: Arc<Queue>,
    }

    fn gid(s: &str) -> GameId {
        GameId::parse(s).unwrap()
    }

    fn target(id: &str, spec: LaunchSpec, dir: Option<&str>) -> LaunchTarget {
        LaunchTarget {
            game: gid(id),
            spec,
            install_dir: dir.map(str::to_owned),
        }
    }

    fn rig_with(front_works: bool, front_game_ok: bool) -> Rig {
        rig_full(front_works, false, front_game_ok)
    }

    fn rig_full(front_works: bool, step4_only: bool, front_game_ok: bool) -> Rig {
        let clock = Arc::new(ManualClock::new());
        clock.set_unix_ms(1_794_610_000_000);
        let manual_a = gid("manual:9f3a1c2b7d4e5a60");
        let manual_b = gid("manual:0000000000000001");
        let mut games = HashMap::new();
        games.insert(
            gid("steam:620"),
            Ok(target(
                "steam:620",
                LaunchSpec::SteamUri { app_id: 620 },
                Some(r"C:\Steam\steamapps\common\Portal 2"),
            )),
        );
        for id in [&manual_a, &manual_b] {
            games.insert(
                id.clone(),
                Ok(target(
                    &id.to_string(),
                    LaunchSpec::Exe {
                        path: r"C:\Jogos\game.exe".into(),
                        args: vec![],
                        working_dir: Some(r"C:\Jogos".into()),
                    },
                    Some(r"C:\Jogos"),
                )),
            );
        }
        games.insert(gid("gog:1000000001"), Err(AppError::NetworkPathBlocked));
        let starter = Arc::new(Starter::default());
        let probe = Arc::new(Probe::default());
        let recorder = Arc::new(Recorder::default());
        let ws = Arc::new(Ws {
            clock: clock.clone(),
            works: front_works,
            step4_only,
            log: Mutex::new(Vec::new()),
            fg: AtomicBool::new(false),
            game_fg: AtomicBool::new(false),
            front_game_ok,
        });
        let window = Arc::new(Window::default());
        let gate = Arc::new(Gate::default());
        let events = Arc::new(Events::default());
        let queue = Arc::new(Queue::default());
        let broker = Arc::new(ForegroundBroker::new(
            ws.clone(),
            clock.clone(),
            AdvancedSettings::default().foreground,
        ));
        let svc = SessionService::new(
            SessionDeps {
                clock: clock.clone(),
                resolver: Arc::new(Resolver(games)),
                recorder: recorder.clone(),
                starter: starter.clone(),
                probe: probe.clone(),
                foreground: broker,
                window: window.clone(),
                input: gate.clone(),
                events: events.clone(),
                runner: queue.clone(),
            },
            SessionConfig::default(),
            ProbeConfig {
                launching_interval_ms: 1000,
                running_interval_ms: 2000,
                use_steam_running_app_id: true,
            },
        );
        Rig {
            svc,
            clock,
            starter,
            probe,
            recorder,
            ws,
            window,
            gate,
            events,
            queue,
        }
    }

    fn rig() -> Rig {
        rig_with(true, true)
    }

    impl Rig {
        fn sink(&self) -> Arc<dyn ProbeSink> {
            self.probe
                .sink
                .lock()
                .unwrap()
                .clone()
                .expect("sonda ligada")
        }
        fn generation(&self) -> u64 {
            self.probe.calls.lock().unwrap().last().unwrap().0
        }
        fn running(&self, id: &str) {
            self.svc.launch(gid(id)).unwrap();
            self.sink().process_seen(self.generation());
            self.queue.drain();
            assert_eq!(self.state(), SessionState::Running);
        }
        fn state(&self) -> SessionState {
            self.svc.get().snapshot.state
        }
        fn advance(&self, ms: u64) {
            self.clock.advance_ms(ms);
            self.svc.tick();
            self.queue.drain();
        }
        fn watch(&self) -> std::sync::MutexGuard<'_, WatchState> {
            // O guard fica preso a `probe`, que vive tanto quanto o `Rig`.
            self.probe.watch.lock().unwrap()
        }
    }

    // ------------------------------------------------------------------- iniciar

    #[test]
    fn iniciar_ate_running_marca_jogado_esconde_a_janela_e_poe_a_entrada_em_segundo_plano() {
        let r = rig();
        r.svc.launch(gid("steam:620")).unwrap();
        assert_eq!(r.state(), SessionState::Launching);
        assert_eq!(r.ws.log.lock().unwrap().first(), Some(&"allow_any"));
        assert!(r.recorder.0.lock().unwrap().is_empty());

        r.advance(800);
        r.sink().process_seen(r.generation());
        r.queue.drain();

        assert_eq!(r.state(), SessionState::Running);
        assert_eq!(
            *r.recorder.0.lock().unwrap(),
            vec![(gid("steam:620"), 1_794_610_000_800)],
            "lastPlayedAt = inicio da sessao (P-06)"
        );
        assert_eq!(*r.window.0.lock().unwrap(), vec!["hide"]);
        assert_eq!(*r.gate.0.lock().unwrap(), vec![InputMode::Background]);
        let states: Vec<_> = r
            .events
            .states
            .lock()
            .unwrap()
            .iter()
            .map(|v| v.snapshot.state)
            .collect();
        assert_eq!(states, [SessionState::Launching, SessionState::Running]);
    }

    #[test]
    fn plano_da_sonda_steam_varre_a_pasta_e_manual_usa_o_job() {
        let r = rig();
        r.svc.launch(gid("steam:620")).unwrap();
        let (_, plan) = r.probe.calls.lock().unwrap()[0].clone();
        assert_eq!(plan.steam_app_id, Some(620));
        assert_eq!(
            plan.scan_dir.as_deref(),
            Some(r"C:\Steam\steamapps\common\Portal 2")
        );

        let r = rig();
        // O starter fake nao devolve Job: para exe manual a sonda cai na varredura por pasta.
        r.svc.launch(gid("manual:9f3a1c2b7d4e5a60")).unwrap();
        let (_, plan) = r.probe.calls.lock().unwrap()[0].clone();
        assert!(!plan.has_job);
        assert_eq!(plan.scan_dir.as_deref(), Some(r"C:\Jogos"));
    }

    #[test]
    fn vinte_launch_seguidos_iniciam_no_maximo_um_processo() {
        let r = rig();
        let mut ok = 0;
        for _ in 0..20 {
            match r.svc.launch(gid("steam:620")) {
                Ok(()) => ok += 1,
                Err(e) => assert_eq!(
                    e,
                    AppError::SessionAlreadyActive {
                        game_id: gid("steam:620")
                    }
                ),
            }
        }
        assert_eq!(ok, 1);
        assert_eq!(r.starter.starts.lock().unwrap().len(), 1);
    }

    #[test]
    fn outro_jogo_com_sessao_ativa_devolve_o_jogo_atual() {
        let r = rig();
        r.running("steam:620");
        assert_eq!(
            r.svc.launch(gid("manual:9f3a1c2b7d4e5a60")).unwrap_err(),
            AppError::SessionAlreadyActive {
                game_id: gid("steam:620")
            }
        );
    }

    #[test]
    fn erros_de_inicio_voltam_a_idle_com_o_codigo_do_contrato() {
        let casos: Vec<(StartError, AppError)> = vec![
            (
                StartError::FileMissing,
                AppError::GameFileMissing {
                    game_id: gid("steam:620"),
                },
            ),
            (StartError::NetworkPath, AppError::NetworkPathBlocked),
            (
                StartError::StoreNotInstalled,
                AppError::StoreNotInstalled {
                    origin: Origin::Steam,
                },
            ),
            (StartError::Failed, AppError::Internal),
        ];
        for (start_err, esperado) in casos {
            let r = rig();
            r.starter.results.lock().unwrap().push_back(Err(start_err));
            assert_eq!(r.svc.launch(gid("steam:620")).unwrap_err(), esperado);
            assert_eq!(r.state(), SessionState::Idle);
            assert!(
                r.probe.calls.lock().unwrap().is_empty(),
                "nao vigia o que nao abriu"
            );
        }
    }

    #[test]
    fn revalidacao_antes_de_iniciar_nao_mexe_na_sessao() {
        let r = rig();
        // Caminho de rede: resolvido pela biblioteca (CA-71, 2o momento).
        assert_eq!(
            r.svc.launch(gid("gog:1000000001")).unwrap_err(),
            AppError::NetworkPathBlocked
        );
        assert_eq!(
            r.svc.launch(gid("epic:Desconhecido")).unwrap_err(),
            AppError::UnknownGame
        );
        *r.starter.preflight_err.lock().unwrap() = Some(StartError::FileMissing);
        assert!(matches!(
            r.svc.launch(gid("steam:620")).unwrap_err(),
            AppError::GameFileMissing { .. }
        ));
        assert_eq!(r.state(), SessionState::Idle);
        assert!(r.starter.starts.lock().unwrap().is_empty());
    }

    // ------------------------------------------------------------- 740 / UAC (AF-01)

    fn needs_elevation(r: &Rig, fingerprint: &str) {
        *r.starter.fingerprint.lock().unwrap() = Some(fingerprint.into());
        r.starter
            .results
            .lock()
            .unwrap()
            .push_back(Err(StartError::NeedsElevation {
                fingerprint: Some(fingerprint.into()),
            }));
        let id = gid("manual:9f3a1c2b7d4e5a60");
        assert_eq!(
            r.svc.launch(id.clone()).unwrap_err(),
            AppError::NeedsElevation { game_id: id }
        );
        assert_eq!(r.state(), SessionState::NeedsElevation);
    }

    #[test]
    fn erro_740_leva_a_needs_elevation_e_so_o_uac_explicito_abre_com_o_verbo_padrao() {
        let r = rig();
        needs_elevation(&r, "sz=10");
        // Nenhum UAC ate o usuario confirmar.
        assert_eq!(r.starter.starts.lock().unwrap().len(), 1);

        r.svc
            .launch_with_uac(gid("manual:9f3a1c2b7d4e5a60"))
            .unwrap();
        let starts = r.starter.starts.lock().unwrap().clone();
        assert_eq!(starts.len(), 2);
        assert_eq!(starts[1].1, StartVerb::Elevated);
        let snap = r.svc.get().snapshot;
        assert_eq!(snap.state, SessionState::Launching);
        assert!(snap.elevated);
        // Jogo elevado fica fora do Job: so varredura por pasta.
        assert_eq!(
            r.probe.calls.lock().unwrap()[0].1.scan_dir.as_deref(),
            Some(r"C:\Jogos")
        );
    }

    #[test]
    fn uac_fora_de_needs_elevation_para_outro_jogo_ou_exe_trocado_e_recusado() {
        // Fora de NeedsElevation (Idle, Launching, Running).
        let r = rig();
        let a = gid("manual:9f3a1c2b7d4e5a60");
        assert_eq!(
            r.svc.launch_with_uac(a.clone()).unwrap_err(),
            AppError::InvalidState
        );
        r.running("manual:9f3a1c2b7d4e5a60");
        assert_eq!(
            r.svc.launch_with_uac(a.clone()).unwrap_err(),
            AppError::InvalidState
        );

        // Outro gameId.
        let r = rig();
        needs_elevation(&r, "sz=10");
        assert_eq!(
            r.svc
                .launch_with_uac(gid("manual:0000000000000001"))
                .unwrap_err(),
            AppError::InvalidState
        );
        // Exe trocado depois do 740.
        *r.starter.fingerprint.lock().unwrap() = Some("sz=11".into());
        assert_eq!(
            r.svc.launch_with_uac(a).unwrap_err(),
            AppError::InvalidState
        );
        assert_eq!(
            r.starter.starts.lock().unwrap().len(),
            1,
            "nenhum UAC foi pedido"
        );
        assert_eq!(r.state(), SessionState::NeedsElevation);
    }

    #[test]
    fn uac_recusado_volta_a_needs_elevation_com_o_motivo() {
        let r = rig();
        needs_elevation(&r, "sz=10");
        let id = gid("manual:9f3a1c2b7d4e5a60");
        r.starter
            .results
            .lock()
            .unwrap()
            .push_back(Err(StartError::ElevationDenied));
        assert_eq!(
            r.svc.launch_with_uac(id.clone()).unwrap_err(),
            AppError::ElevationDenied { game_id: id }
        );
        let snap = r.svc.get().snapshot;
        assert_eq!(snap.state, SessionState::NeedsElevation);
        assert_eq!(
            snap.launch_failure,
            Some(crate::domain::session::LaunchFailure::ElevationDenied)
        );
    }

    #[test]
    fn voltar_em_needs_elevation_cancela_sem_processo() {
        let r = rig();
        needs_elevation(&r, "sz=10");
        r.svc.dismiss();
        assert_eq!(r.state(), SessionState::Idle);
        assert_eq!(
            r.svc
                .launch_with_uac(gid("manual:9f3a1c2b7d4e5a60"))
                .unwrap_err(),
            AppError::InvalidState
        );
    }

    // -------------------------------------------------------- dismiss e 60 s (CA-77)

    #[test]
    fn dismiss_em_launching_continua_e_o_jogo_que_abre_depois_marca_jogado() {
        let r = rig();
        r.svc.launch(gid("steam:620")).unwrap();
        r.svc.dismiss();
        let snap = r.svc.get().snapshot;
        assert_eq!(snap.state, SessionState::Launching);
        assert!(snap.dismissed);
        r.sink().process_seen(r.generation());
        r.queue.drain();
        assert_eq!(r.state(), SessionState::Running);
        assert_eq!(r.recorder.0.lock().unwrap().len(), 1);
    }

    #[test]
    fn sessenta_segundos_sem_processo_vira_not_confirmed_e_retry_refaz() {
        let r = rig();
        r.svc.launch(gid("steam:620")).unwrap();
        r.advance(60_000);
        assert_eq!(r.state(), SessionState::NotConfirmed);
        let first_gen = r.generation();
        r.svc.retry().unwrap();
        assert_eq!(r.state(), SessionState::Launching);
        assert_eq!(r.starter.starts.lock().unwrap().len(), 2);
        assert!(r.generation() > first_gen);
        // Aviso da sonda antiga e ignorado.
        r.sink().process_seen(first_gen);
        assert_eq!(r.state(), SessionState::Launching);
        // Retry fora de NotConfirmed.
        assert_eq!(r.svc.retry().unwrap_err(), AppError::InvalidState);
    }

    // ------------------------------------------------------------ fechar e forcar

    #[test]
    fn fechar_pede_wm_close_e_o_fim_dos_processos_traz_o_sofa_com_o_jogo_em_foco() {
        let r = rig();
        r.running("steam:620");
        r.window.0.lock().unwrap().clear();
        r.svc.close().unwrap();
        assert_eq!(r.state(), SessionState::Closing);
        assert_eq!(r.watch().close_calls, 1);

        r.sink().processes_gone(r.generation());
        r.queue.drain();
        assert_eq!(r.state(), SessionState::Idle);
        assert_eq!(*r.window.0.lock().unwrap(), vec!["show"]);
        assert_eq!(
            r.gate.0.lock().unwrap().last(),
            Some(&InputMode::Foreground)
        );
        assert_eq!(*r.events.ended.lock().unwrap(), vec![gid("steam:620")]);
        assert!(
            r.ws.log.lock().unwrap().contains(&"step1"),
            "a mesma escada do retorno (CA-30)"
        );
    }

    #[test]
    fn fechar_fora_de_running_e_invalid_state_ou_no_active_session() {
        let r = rig();
        assert_eq!(r.svc.close().unwrap_err(), AppError::NoActiveSession);
        r.svc.launch(gid("steam:620")).unwrap();
        assert_eq!(r.svc.close().unwrap_err(), AppError::InvalidState);
    }

    #[test]
    fn forcar_so_em_close_overdue_e_depois_do_prazo_de_10_s() {
        let r = rig();
        // Idle.
        assert_eq!(r.svc.force_close().unwrap_err(), AppError::NoActiveSession);
        r.running("steam:620");
        assert_eq!(r.svc.force_close().unwrap_err(), AppError::NotOverdue);
        r.svc.close().unwrap();
        assert_eq!(r.svc.force_close().unwrap_err(), AppError::NotOverdue);
        r.advance(9_999);
        assert_eq!(r.state(), SessionState::Closing);
        r.advance(1);
        assert_eq!(r.state(), SessionState::CloseOverdue);
        assert_eq!(r.watch().force_calls, 0, "nada e forcado sozinho (INV-05)");

        r.svc.force_close().unwrap();
        assert_eq!(r.watch().force_calls, 1);
    }

    #[test]
    fn forcar_jogo_elevado_devolve_access_denied() {
        let r = rig();
        r.running("steam:620");
        r.svc.close().unwrap();
        r.advance(10_000);
        r.watch().force_result = Some(AppError::AccessDenied);
        assert_eq!(r.svc.force_close().unwrap_err(), AppError::AccessDenied);
        assert_eq!(r.state(), SessionState::CloseOverdue);
    }

    #[test]
    fn dismiss_em_closing_nao_cancela_o_fechamento_e_o_forcar_continua_oferecido() {
        let r = rig();
        r.running("steam:620");
        r.svc.close().unwrap();
        r.svc.dismiss();
        let snap = r.svc.get().snapshot;
        assert_eq!(snap.state, SessionState::Closing);
        assert!(snap.dismissed);
        r.advance(10_000);
        assert_eq!(
            r.state(),
            SessionState::CloseOverdue,
            "CA-78: forcar oferecido apos dismiss"
        );
        // "Cancelar" em CloseOverdue volta a Running.
        r.svc.dismiss();
        assert_eq!(r.state(), SessionState::Running);
    }

    #[test]
    fn processo_que_reaparece_em_menos_de_5_s_nao_encerra_a_sessao() {
        let r = rig();
        r.running("steam:620");
        r.sink().processes_gone(r.generation());
        assert_eq!(r.state(), SessionState::Ending);
        r.advance(4_000);
        r.sink().process_seen(r.generation());
        assert_eq!(r.state(), SessionState::Running);
        r.advance(10_000);
        assert_eq!(r.state(), SessionState::Running);
    }

    #[test]
    fn fim_pelo_menu_do_jogo_traz_o_sofa_apos_5_s() {
        let r = rig();
        r.running("steam:620");
        r.sink().processes_gone(r.generation());
        r.advance(5_000);
        assert_eq!(r.state(), SessionState::Idle);
        assert_eq!(*r.events.ended.lock().unwrap(), vec![gid("steam:620")]);
    }

    // ------------------------------------------------------------------- replace

    #[test]
    fn replace_pede_o_fechamento_normal_e_abre_o_novo_quando_a_sessao_chega_a_idle() {
        let r = rig();
        r.running("steam:620");
        r.svc.replace(gid("manual:9f3a1c2b7d4e5a60")).unwrap();
        assert_eq!(r.state(), SessionState::Closing);
        assert_eq!(r.watch().close_calls, 1);
        assert_eq!(r.starter.starts.lock().unwrap().len(), 1);

        r.sink().processes_gone(r.generation());
        r.queue.drain();
        let starts = r.starter.starts.lock().unwrap().clone();
        assert_eq!(starts.len(), 2);
        assert_eq!(starts[1].0, gid("manual:9f3a1c2b7d4e5a60"));
        assert_eq!(r.state(), SessionState::Launching);
    }

    #[test]
    fn replace_com_jogo_que_ignora_wm_close_nao_encerra_nada_sem_force_close() {
        let r = rig();
        r.running("steam:620");
        r.svc.replace(gid("manual:9f3a1c2b7d4e5a60")).unwrap();
        r.advance(10_000); // o jogo ignorou o WM_CLOSE
        assert_eq!(r.state(), SessionState::CloseOverdue);
        assert_eq!(r.watch().force_calls, 0, "CT-01: replace nunca forca");
        assert_eq!(
            r.starter.starts.lock().unwrap().len(),
            1,
            "pendente descartado"
        );
        // Em CloseOverdue, replace e InvalidState.
        assert_eq!(
            r.svc.replace(gid("manual:0000000000000001")).unwrap_err(),
            AppError::InvalidState
        );
        assert_eq!(r.watch().force_calls, 0);
        // So o force_close confirmado encerra, e o novo jogo NAO abre sozinho.
        r.svc.force_close().unwrap();
        assert_eq!(r.watch().force_calls, 1);
        r.sink().processes_gone(r.generation());
        r.queue.drain();
        assert_eq!(r.starter.starts.lock().unwrap().len(), 1);
    }

    #[test]
    fn replace_valida_o_novo_jogo_antes_de_fechar_o_atual() {
        let r = rig();
        r.running("steam:620");
        assert_eq!(
            r.svc.replace(gid("gog:1000000001")).unwrap_err(),
            AppError::NetworkPathBlocked
        );
        assert_eq!(
            r.svc.replace(gid("epic:Nao")).unwrap_err(),
            AppError::UnknownGame
        );
        assert_eq!(r.state(), SessionState::Running);
        assert_eq!(r.watch().close_calls, 0);
    }

    #[test]
    fn replace_fora_de_running_e_invalid_state() {
        let r = rig();
        assert_eq!(
            r.svc.replace(gid("steam:620")).unwrap_err(),
            AppError::InvalidState
        );
        r.svc.launch(gid("steam:620")).unwrap();
        assert_eq!(
            r.svc.replace(gid("manual:9f3a1c2b7d4e5a60")).unwrap_err(),
            AppError::InvalidState
        );
    }

    // ---------------------------------------------------- combinacao de retorno / voltar

    #[test]
    fn combinacao_de_retorno_mostra_o_sofa_roda_a_escada_e_abre_o_painel() {
        let r = rig();
        r.running("steam:620");
        r.window.0.lock().unwrap().clear();
        r.svc.handle_return_combo();
        assert_eq!(
            *r.window.0.lock().unwrap(),
            vec!["show"],
            "WebView2 retomado primeiro"
        );
        assert!(
            r.events.panels.lock().unwrap().is_empty(),
            "painel so apos a escada"
        );
        r.queue.drain();
        assert_eq!(*r.events.panels.lock().unwrap(), vec![FrontOutcome::Front]);
        assert_eq!(
            r.gate.0.lock().unwrap().last(),
            Some(&InputMode::Foreground)
        );
        assert!(!r.svc.get().snapshot.minimized);
    }

    #[test]
    fn retorno_pelo_degrau_4_marca_o_jogo_como_minimizado() {
        let r = rig_full(false, true, true);
        r.running("steam:620");
        r.svc.handle_return_combo();
        r.queue.drain();
        assert_eq!(
            *r.events.panels.lock().unwrap(),
            vec![FrontOutcome::FrontAfterMinimize]
        );
        assert!(r.svc.get().snapshot.minimized, "aviso E-23 do painel");
    }

    #[test]
    fn retorno_sem_nenhum_degrau_funcionando_devolve_failed_sem_marcar_minimizado() {
        let r = rig_with(false, true);
        r.running("steam:620");
        r.svc.handle_return_combo();
        r.queue.drain();
        assert_eq!(*r.events.panels.lock().unwrap(), vec![FrontOutcome::Failed]);
        assert!(!r.svc.get().snapshot.minimized);
    }

    #[test]
    fn combinacao_fora_de_sessao_ou_repetida_nao_dispara_duas_escadas() {
        let r = rig();
        r.svc.handle_return_combo(); // Idle: ignorada
        assert!(r.queue.0.lock().unwrap().is_empty());
        r.running("steam:620");
        r.svc.handle_return_combo();
        r.svc.handle_return_combo();
        assert_eq!(r.queue.0.lock().unwrap().len(), 1);
        r.queue.drain();
        assert_eq!(r.events.panels.lock().unwrap().len(), 1);
    }

    #[test]
    fn voltar_ao_jogo_esconde_o_sofa_e_poe_a_entrada_em_segundo_plano() {
        let r = rig();
        r.running("steam:620");
        r.svc.handle_return_combo();
        r.queue.drain();
        r.window.0.lock().unwrap().clear();
        r.gate.0.lock().unwrap().clear();
        r.svc.return_to_game().unwrap();
        assert_eq!(*r.window.0.lock().unwrap(), vec!["hide"]);
        assert_eq!(*r.gate.0.lock().unwrap(), vec![InputMode::Background]);
    }

    #[test]
    fn voltar_ao_jogo_sem_conseguir_o_foco_devolve_foreground_denied() {
        let r = rig_with(true, false);
        r.running("steam:620");
        assert_eq!(
            r.svc.return_to_game().unwrap_err(),
            AppError::ForegroundDenied
        );
        assert_eq!(r.state(), SessionState::Running);
    }

    #[test]
    fn voltar_ao_jogo_sem_sessao_e_no_active_session() {
        let r = rig();
        assert_eq!(
            r.svc.return_to_game().unwrap_err(),
            AppError::NoActiveSession
        );
    }

    #[test]
    fn since_vem_em_ms_da_epoca_unix() {
        let r = rig();
        r.svc.launch(gid("steam:620")).unwrap();
        r.advance(1_000);
        let v = r.svc.get();
        assert_eq!(v.since_unix_ms, Some(1_794_610_000_000));
        assert_eq!(v.snapshot.game_id, Some(gid("steam:620")));
    }

    #[test]
    fn needs_tick_so_com_sessao() {
        let r = rig();
        assert!(!r.svc.needs_tick());
        r.svc.launch(gid("steam:620")).unwrap();
        assert!(r.svc.needs_tick());
    }
}
