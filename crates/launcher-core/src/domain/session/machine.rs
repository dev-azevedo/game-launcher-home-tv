//! Maquina de estados da sessao (arquitetura 2.4, contrato secoes 2.2 e 4). Pura: o tempo entra
//! como `Millis`, os efeitos saem como valores (`SessionEffect`) e o caso de uso os executa.
//!
//! Prazos: 60 s para confirmar o inicio (CA-29), 5 s sem processo para encerrar (E-14) e 10 s
//! ate oferecer forcar o fechamento (CA-35). Forcar nunca acontece sozinho: so `force_close`,
//! so em `CloseOverdue`, depois de confirmado pelo usuario (RN-12, INV-05, CT-01).

use crate::application::AppError;
use crate::domain::library::GameId;
use crate::domain::settings::SessionSettings;
use crate::domain::time::Millis;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionState {
    Idle,
    Launching,
    NeedsElevation,
    NotConfirmed,
    Running,
    Ending,
    Closing,
    CloseOverdue,
}

/// Resultado de trazer o Sofa para a frente (ADR-0003): o mesmo enum do contrato (`FrontOutcome`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrontOutcome {
    Front,
    FrontAfterMinimize,
    Failed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LaunchFailure {
    ElevationDenied,
}

/// Prazos da sessao, em ms (vem de `advanced.session`, 9.3 da arquitetura).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SessionConfig {
    pub start_timeout_ms: u64,
    pub end_grace_ms: u64,
    pub close_timeout_ms: u64,
}

impl Default for SessionConfig {
    fn default() -> Self {
        Self {
            start_timeout_ms: 60_000,
            end_grace_ms: 5_000,
            close_timeout_ms: 10_000,
        }
    }
}

impl From<&SessionSettings> for SessionConfig {
    fn from(s: &SessionSettings) -> Self {
        Self {
            start_timeout_ms: u64::from(s.start_timeout_seconds) * 1000,
            end_grace_ms: u64::from(s.end_grace_seconds) * 1000,
            close_timeout_ms: u64::from(s.close_timeout_seconds) * 1000,
        }
    }
}

/// O que o caso de uso deve fazer em consequencia de uma transicao.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SessionEffect {
    /// Iniciar (ou reiniciar, no "Tentar de novo") o jogo pelo `GameStarter`.
    StartGame(GameId),
    /// Iniciar com o verbo padrao do Windows depois do aviso do Sofa (AF-01).
    StartGameWithUac(GameId),
    /// Primeiro processo associado: grava `lastPlayedAt` (P-06).
    MarkPlayed(GameId),
    /// `WM_CLOSE` nas janelas da sessao (pedido normal).
    RequestClose,
    /// Encerra os processos da sessao (handle guardado ou `TerminateJobObject`). So apos `force_close`.
    ForceClose,
    /// Traz o jogo da sessao para a frente ("Voltar ao jogo").
    FrontGame,
    /// Traz o Sofa (escada de primeiro plano). Fim de sessao: `focus` = jogo em foco (CA-30).
    /// Inicio nao confirmado em 60 s: `focus = None`, so mostra a mensagem (CA-29, ARQ-BUG-01).
    BringLauncherFront { focus: Option<GameId> },
    /// Fechamento do `session_replace` terminou: abrir este jogo pelo fluxo normal de lancamento.
    LaunchPending(GameId),
}

/// Fotografia para `SessionDto` / `session://state`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionSnapshot {
    pub state: SessionState,
    pub game_id: Option<GameId>,
    pub since: Option<Millis>,
    pub dismissed: bool,
    pub elevated: bool,
    pub minimized: bool,
    pub launch_failure: Option<LaunchFailure>,
}

#[derive(Debug, Clone)]
pub struct Session {
    cfg: SessionConfig,
    state: SessionState,
    game: Option<GameId>,
    since: Millis,
    dismissed: bool,
    elevated: bool,
    minimized: bool,
    launch_failure: Option<LaunchFailure>,
    /// Jogo a abrir quando o fechamento do `session_replace` terminar (CT-01).
    pending: Option<GameId>,
}

type Effects = Vec<SessionEffect>;

impl Session {
    pub fn new(cfg: SessionConfig) -> Self {
        Self {
            cfg,
            state: SessionState::Idle,
            game: None,
            since: Millis::ZERO,
            dismissed: false,
            elevated: false,
            minimized: false,
            launch_failure: None,
            pending: None,
        }
    }

    pub fn state(&self) -> SessionState {
        self.state
    }

    pub fn game(&self) -> Option<&GameId> {
        self.game.as_ref()
    }

    pub fn pending_replace(&self) -> Option<&GameId> {
        self.pending.as_ref()
    }

    pub fn snapshot(&self) -> SessionSnapshot {
        let idle = self.state == SessionState::Idle;
        SessionSnapshot {
            state: self.state,
            game_id: self.game.clone(),
            since: (!idle).then_some(self.since),
            dismissed: self.dismissed,
            elevated: self.elevated,
            minimized: self.minimized,
            launch_failure: self.launch_failure,
        }
    }

    fn enter(&mut self, state: SessionState, now: Millis) {
        self.state = state;
        self.since = now;
        self.dismissed = false;
    }

    fn reset_to_idle(&mut self, now: Millis) {
        self.enter(SessionState::Idle, now);
        self.game = None;
        self.elevated = false;
        self.minimized = false;
        self.launch_failure = None;
        self.pending = None;
    }

    fn current(&self) -> GameId {
        self.game.clone().expect("estado sem jogo")
    }

    /// Erro de "ha sessao" para comandos que so valem em estados especificos.
    fn not_in_state(&self) -> AppError {
        if self.state == SessionState::Idle {
            AppError::NoActiveSession
        } else {
            AppError::InvalidState
        }
    }

    // ------------------------------------------------------------------ comandos (UI)

    /// `session_launch` / primeiro passo de `session_replace` depois do `Idle`.
    pub fn launch(&mut self, game: GameId, now: Millis) -> Result<Effects, AppError> {
        if self.state != SessionState::Idle {
            return Err(AppError::SessionAlreadyActive {
                game_id: self.current(),
            });
        }
        self.game = Some(game.clone());
        self.elevated = false;
        self.minimized = false;
        self.launch_failure = None;
        self.enter(SessionState::Launching, now);
        Ok(vec![SessionEffect::StartGame(game)])
    }

    /// `session_launch_with_uac`: so em `NeedsElevation` e para o mesmo jogo (AF-01).
    pub fn launch_with_uac(&mut self, game: &GameId, now: Millis) -> Result<Effects, AppError> {
        if self.state != SessionState::NeedsElevation || self.game.as_ref() != Some(game) {
            return Err(AppError::InvalidState);
        }
        self.elevated = true;
        self.launch_failure = None;
        self.enter(SessionState::Launching, now);
        Ok(vec![SessionEffect::StartGameWithUac(game.clone())])
    }

    /// `session_retry` ("Tentar de novo" em `NotConfirmed`).
    pub fn retry(&mut self, now: Millis) -> Result<Effects, AppError> {
        if self.state != SessionState::NotConfirmed {
            return Err(AppError::InvalidState);
        }
        self.enter(SessionState::Launching, now);
        Ok(vec![SessionEffect::StartGame(self.current())])
    }

    /// `session_return_to_game`.
    pub fn return_to_game(&self) -> Result<Effects, AppError> {
        match self.state {
            SessionState::Running => Ok(vec![SessionEffect::FrontGame]),
            _ => Err(self.not_in_state()),
        }
    }

    /// `session_close`: pedido normal de fechamento (`WM_CLOSE`).
    pub fn request_close(&mut self, now: Millis) -> Result<Effects, AppError> {
        if self.state != SessionState::Running {
            return Err(self.not_in_state());
        }
        self.enter(SessionState::Closing, now);
        Ok(vec![SessionEffect::RequestClose])
    }

    /// `session_replace` (CT-01): **so** `Running`; faz o mesmo pedido normal de `request_close`
    /// e guarda o novo jogo. O novo jogo abre quando a sessao chega a `Idle` (`LaunchPending`);
    /// se o fechamento passar do prazo, o pendente e descartado e o usuario decide.
    pub fn replace(&mut self, new_game: GameId, now: Millis) -> Result<Effects, AppError> {
        if self.state != SessionState::Running {
            return Err(AppError::InvalidState);
        }
        self.pending = Some(new_game);
        self.enter(SessionState::Closing, now);
        Ok(vec![SessionEffect::RequestClose])
    }

    /// `session_force_close`: so em `CloseOverdue` (RN-12). Fora dele nada e encerrado.
    pub fn force_close(&mut self) -> Result<Effects, AppError> {
        match self.state {
            SessionState::CloseOverdue => Ok(vec![SessionEffect::ForceClose]),
            SessionState::Idle | SessionState::NeedsElevation | SessionState::NotConfirmed => {
                Err(AppError::NoActiveSession)
            }
            _ => Err(AppError::NotOverdue),
        }
    }

    /// `session_dismiss` (CA-77, CA-78): nunca cancela inicio nem fechamento em andamento.
    pub fn dismiss(&mut self, now: Millis) -> Effects {
        match self.state {
            SessionState::Launching | SessionState::Closing => self.dismissed = true,
            SessionState::NeedsElevation | SessionState::NotConfirmed => self.reset_to_idle(now),
            SessionState::CloseOverdue => self.enter(SessionState::Running, now), // "Cancelar" (CA-35)
            SessionState::Idle | SessionState::Running | SessionState::Ending => {}
        }
        Vec::new()
    }

    // ------------------------------------------------------------- eventos (infraestrutura)

    /// O inicio falhou antes de haver processo (arquivo ausente, loja ausente, caminho de rede):
    /// nada rodando, a sessao volta a `Idle` e o comando devolve o erro (CA-28, CA-71).
    pub fn launch_failed(&mut self, now: Millis) -> Effects {
        if self.state == SessionState::Launching {
            self.reset_to_idle(now);
        }
        Vec::new()
    }

    /// `CreateProcess` devolveu o erro 740: o jogo exige administrador (AF-01, CA-73).
    pub fn elevation_required(&mut self, now: Millis) -> Effects {
        if self.state == SessionState::Launching && !self.elevated {
            self.enter(SessionState::NeedsElevation, now);
        }
        Vec::new()
    }

    /// O usuario recusou o UAC do jogo: volta a `NeedsElevation` com o motivo (CA-73).
    pub fn elevation_denied(&mut self, now: Millis) -> Effects {
        if self.state == SessionState::Launching && self.elevated {
            self.elevated = false;
            self.enter(SessionState::NeedsElevation, now);
            self.launch_failure = Some(LaunchFailure::ElevationDenied);
        }
        Vec::new()
    }

    /// `SessionProbe` viu um processo associado ao jogo.
    pub fn process_seen(&mut self, now: Millis) -> Effects {
        match self.state {
            // Inicio confirmado. `NotConfirmed` tambem: o jogo abriu depois dos 60 s (CA-29 nao
            // cancela o inicio) e a sessao segue o fluxo normal.
            SessionState::Launching | SessionState::NotConfirmed => {
                self.enter(SessionState::Running, now);
                vec![SessionEffect::MarkPlayed(self.current())]
            }
            // E-14: o jogo reabriu sozinho em menos de 5 s.
            SessionState::Ending => {
                self.enter(SessionState::Running, now);
                Vec::new()
            }
            _ => Vec::new(),
        }
    }

    /// Nenhum processo associado segue rodando (aviso do Job ou varredura).
    pub fn processes_gone(&mut self, now: Millis) -> Effects {
        match self.state {
            SessionState::Running => {
                self.enter(SessionState::Ending, now);
                Vec::new()
            }
            // Fechamento pedido pelo usuario: terminou, sem a tolerancia de E-14.
            SessionState::Closing | SessionState::CloseOverdue => self.finish(now),
            _ => Vec::new(),
        }
    }

    /// Marca que o retorno minimizou o jogo (degrau 4): aviso do painel (E-23).
    pub fn set_minimized(&mut self, minimized: bool) {
        if matches!(
            self.state,
            SessionState::Running | SessionState::Closing | SessionState::CloseOverdue
        ) {
            self.minimized = minimized;
        }
    }

    /// Avanca os prazos. Idempotente; chamar a cada leitura da sonda ou por temporizador.
    pub fn tick(&mut self, now: Millis) -> Effects {
        let elapsed = now.since(self.since);
        match self.state {
            SessionState::Launching if elapsed >= self.cfg.start_timeout_ms => {
                self.enter(SessionState::NotConfirmed, now);
                // CA-29: a mensagem "Nao foi possivel confirmar" nao pode ficar atras da tela da
                // loja (login, atualizacao). Sem `focus`: nao e fim de sessao.
                vec![SessionEffect::BringLauncherFront { focus: None }]
            }
            SessionState::Ending if elapsed >= self.cfg.end_grace_ms => self.finish(now),
            SessionState::Closing if elapsed >= self.cfg.close_timeout_ms => {
                // CT-01: o jogo que ignora o pedido nao e forcado; o pendente do replace some e
                // o usuario escolhe entre cancelar e forcar (com 2a confirmacao).
                self.pending = None;
                self.enter(SessionState::CloseOverdue, now);
                Vec::new()
            }
            _ => Vec::new(),
        }
    }

    /// Fim da sessao: traz o Sofa com o jogo em foco e, se havia `replace`, manda abrir o novo.
    fn finish(&mut self, now: Millis) -> Effects {
        let game = self.current();
        let pending = self.pending.take();
        self.reset_to_idle(now);
        let mut effects = vec![SessionEffect::BringLauncherFront { focus: Some(game) }];
        if let Some(next) = pending {
            effects.push(SessionEffect::LaunchPending(next));
        }
        effects
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::ports::{Clock, ManualClock};
    use SessionEffect as E;
    use SessionState as S;

    fn gid(s: &str) -> GameId {
        GameId::parse(s).unwrap()
    }

    fn portal() -> GameId {
        gid("steam:620")
    }

    /// Relogio fake + sessao: `advance` anda o relogio e roda o `tick`.
    struct Rig {
        clock: ManualClock,
        s: Session,
    }

    impl Rig {
        fn new() -> Self {
            Self {
                clock: ManualClock::new(),
                s: Session::new(SessionConfig::default()),
            }
        }
        fn now(&self) -> Millis {
            self.clock.now()
        }
        fn advance(&mut self, ms: u64) -> Effects {
            self.clock.advance_ms(ms);
            self.s.tick(self.clock.now())
        }
        fn launch(&mut self, id: &str) {
            let now = self.now();
            self.s.launch(gid(id), now).unwrap();
        }
        fn running(id: &str) -> Self {
            let mut r = Self::new();
            r.launch(id);
            let now = r.now();
            r.s.process_seen(now);
            assert_eq!(r.s.state(), S::Running);
            r
        }
        fn closing(id: &str) -> Self {
            let mut r = Self::running(id);
            let now = r.now();
            r.s.request_close(now).unwrap();
            r
        }
        fn overdue(id: &str) -> Self {
            let mut r = Self::closing(id);
            r.advance(10_000);
            assert_eq!(r.s.state(), S::CloseOverdue);
            r
        }
    }

    /// Leva uma sessao nova ate cada um dos 8 estados.
    fn rig_in(state: S) -> Rig {
        let mut r = Rig::new();
        match state {
            S::Idle => {}
            S::Launching => r.launch("steam:620"),
            S::NeedsElevation => {
                r.launch("steam:620");
                let now = r.now();
                r.s.elevation_required(now);
            }
            S::NotConfirmed => {
                r.launch("steam:620");
                r.advance(60_000);
            }
            S::Running => return Rig::running("steam:620"),
            S::Ending => {
                r = Rig::running("steam:620");
                let now = r.now();
                r.s.processes_gone(now);
            }
            S::Closing => return Rig::closing("steam:620"),
            S::CloseOverdue => return Rig::overdue("steam:620"),
        }
        assert_eq!(r.s.state(), state);
        r
    }

    const ALL: [S; 8] = [
        S::Idle,
        S::Launching,
        S::NeedsElevation,
        S::NotConfirmed,
        S::Running,
        S::Ending,
        S::Closing,
        S::CloseOverdue,
    ];

    // ---- inicio ----

    #[test]
    fn launch_em_idle_vai_a_launching_e_pede_para_iniciar() {
        let mut r = Rig::new();
        let fx = r.s.launch(portal(), r.now()).unwrap();
        assert_eq!(fx, vec![E::StartGame(portal())]);
        assert_eq!(r.s.state(), S::Launching);
        let snap = r.s.snapshot();
        assert_eq!(snap.game_id, Some(portal()));
        assert!(snap.since.is_some() && !snap.dismissed && !snap.elevated);
    }

    #[test]
    fn launch_com_sessao_em_qualquer_estado_nao_idle_e_session_already_active_com_o_jogo_atual() {
        // RN-09 / CA-31 / contrato secao 4: todas as linhas da coluna `session_launch`.
        for state in ALL.into_iter().filter(|s| *s != S::Idle) {
            let mut r = rig_in(state);
            let before = r.s.snapshot();
            let err = r.s.launch(gid("epic:SampleGameA"), r.now()).unwrap_err();
            assert_eq!(
                err,
                AppError::SessionAlreadyActive { game_id: portal() },
                "{state:?}"
            );
            assert_eq!(r.s.snapshot(), before, "nada muda");
        }
    }

    #[test]
    fn primeiro_processo_confirma_o_inicio_e_marca_jogado_uma_vez() {
        let mut r = Rig::new();
        r.launch("steam:620");
        r.advance(500);
        let fx = r.s.process_seen(r.now());
        assert_eq!(fx, vec![E::MarkPlayed(portal())]);
        assert_eq!(r.s.state(), S::Running);
        // Outros processos depois: nada.
        assert!(r.s.process_seen(r.now()).is_empty());
    }

    #[test]
    fn ca29_sessenta_segundos_sem_processo_vao_a_not_confirmed() {
        let mut r = Rig::new();
        r.launch("steam:620");
        assert!(r.advance(59_999).is_empty());
        assert_eq!(r.s.state(), S::Launching);
        let fx = r.advance(1);
        assert_eq!(r.s.state(), S::NotConfirmed);
        assert_eq!(
            fx,
            vec![E::BringLauncherFront { focus: None }],
            "ARQ-BUG-01: traz o Sofa, sem jogo em foco"
        );
        assert_eq!(
            r.s.game(),
            Some(&portal()),
            "o jogo continua associado ao 'Tentar de novo'"
        );
    }

    #[test]
    fn not_confirmed_tentar_de_novo_reinicia_o_prazo() {
        let mut r = rig_in(S::NotConfirmed);
        let fx = r.s.retry(r.now()).unwrap();
        assert_eq!(fx, vec![E::StartGame(portal())]);
        assert_eq!(r.s.state(), S::Launching);
        r.advance(59_000);
        assert_eq!(r.s.state(), S::Launching, "60 s contam do novo inicio");
        r.advance(1_000);
        assert_eq!(r.s.state(), S::NotConfirmed);
    }

    #[test]
    fn jogo_que_abre_depois_dos_60_s_segue_o_fluxo_normal() {
        let mut r = rig_in(S::NotConfirmed);
        let fx = r.s.process_seen(r.now());
        assert_eq!(fx, vec![E::MarkPlayed(portal())]);
        assert_eq!(r.s.state(), S::Running);
    }

    // ---- elevacao (AF-01, CA-73) ----

    #[test]
    fn erro_740_vai_a_needs_elevation_e_uac_confirmado_volta_a_launching_elevado() {
        let mut r = rig_in(S::NeedsElevation);
        let fx = r.s.launch_with_uac(&portal(), r.now()).unwrap();
        assert_eq!(fx, vec![E::StartGameWithUac(portal())]);
        let snap = r.s.snapshot();
        assert_eq!((snap.state, snap.elevated), (S::Launching, true));
        let fx = r.s.process_seen(r.now());
        assert_eq!(fx, vec![E::MarkPlayed(portal())]);
        assert!(
            r.s.snapshot().elevated,
            "aviso 'pode falhar' do E-23 segue na sessao"
        );
    }

    #[test]
    fn launch_with_uac_so_em_needs_elevation_e_para_o_mesmo_jogo() {
        for state in ALL.into_iter().filter(|s| *s != S::NeedsElevation) {
            let mut r = rig_in(state);
            let before = r.s.snapshot();
            assert_eq!(
                r.s.launch_with_uac(&portal(), r.now()),
                Err(AppError::InvalidState),
                "{state:?}"
            );
            assert_eq!(r.s.snapshot(), before);
        }
        let mut r = rig_in(S::NeedsElevation);
        assert_eq!(
            r.s.launch_with_uac(&gid("steam:730"), r.now()),
            Err(AppError::InvalidState),
            "outro gameId"
        );
        assert_eq!(r.s.state(), S::NeedsElevation);
    }

    #[test]
    fn uac_recusado_volta_a_needs_elevation_com_o_motivo() {
        let mut r = rig_in(S::NeedsElevation);
        r.s.launch_with_uac(&portal(), r.now()).unwrap();
        r.s.elevation_denied(r.now());
        let snap = r.s.snapshot();
        assert_eq!(snap.state, S::NeedsElevation);
        assert_eq!(snap.launch_failure, Some(LaunchFailure::ElevationDenied));
        assert!(!snap.elevated);
        // Tentar de novo limpa o motivo.
        r.s.launch_with_uac(&portal(), r.now()).unwrap();
        assert_eq!(r.s.snapshot().launch_failure, None);
    }

    #[test]
    fn erro_740_fora_de_launching_ou_com_uac_pendente_nao_muda_nada() {
        let mut r = rig_in(S::Running);
        r.s.elevation_required(r.now());
        assert_eq!(r.s.state(), S::Running);
        let mut r = rig_in(S::NeedsElevation);
        r.s.launch_with_uac(&portal(), r.now()).unwrap();
        r.s.elevation_required(r.now());
        assert_eq!(
            r.s.state(),
            S::Launching,
            "UAC em andamento nao volta por um 740"
        );
    }

    // ---- fim de sessao (E-14, CA-30) ----

    #[test]
    fn ca30_cinco_segundos_sem_processo_encerram_e_trazem_o_launcher() {
        let mut r = rig_in(S::Running);
        r.s.processes_gone(r.now());
        assert_eq!(r.s.state(), S::Ending);
        assert!(r.advance(4_999).is_empty());
        let fx = r.advance(1);
        assert_eq!(
            fx,
            vec![E::BringLauncherFront {
                focus: Some(portal())
            }]
        );
        assert_eq!(r.s.state(), S::Idle);
        assert_eq!(r.s.snapshot().game_id, None);
        assert_eq!(r.s.snapshot().since, None);
    }

    #[test]
    fn e14_processo_que_reaparece_em_menos_de_5_s_mantem_a_sessao() {
        let mut r = rig_in(S::Running);
        r.s.processes_gone(r.now());
        r.advance(4_900);
        let fx = r.s.process_seen(r.now());
        assert!(fx.is_empty(), "nao marca jogado de novo");
        assert_eq!(r.s.state(), S::Running);
        // O prazo de 5 s recomeca na proxima saida.
        r.s.processes_gone(r.now());
        r.advance(4_999);
        assert_eq!(r.s.state(), S::Ending);
        r.advance(1);
        assert_eq!(r.s.state(), S::Idle);
    }

    #[test]
    fn jogo_que_fecha_e_reabre_varias_vezes_nunca_encerra_a_sessao() {
        let mut r = rig_in(S::Running);
        for _ in 0..10 {
            r.s.processes_gone(r.now());
            r.advance(3_000);
            r.s.process_seen(r.now());
            assert_eq!(r.s.state(), S::Running);
        }
    }

    // ---- fechar (RN-12, CA-35) ----

    #[test]
    fn fechar_pede_wm_close_e_vai_a_closing() {
        let mut r = rig_in(S::Running);
        let fx = r.s.request_close(r.now()).unwrap();
        assert_eq!(fx, vec![E::RequestClose]);
        assert_eq!(r.s.state(), S::Closing);
    }

    #[test]
    fn fechar_so_em_running() {
        for state in ALL.into_iter().filter(|s| *s != S::Running) {
            let mut r = rig_in(state);
            let expected = if state == S::Idle {
                AppError::NoActiveSession
            } else {
                AppError::InvalidState
            };
            assert_eq!(r.s.request_close(r.now()), Err(expected), "{state:?}");
            assert_eq!(r.s.state(), state);
        }
    }

    #[test]
    fn processos_terminam_em_closing_volta_a_idle_na_hora_sem_tolerancia() {
        let mut r = rig_in(S::Closing);
        let fx = r.s.processes_gone(r.now());
        assert_eq!(
            fx,
            vec![E::BringLauncherFront {
                focus: Some(portal())
            }]
        );
        assert_eq!(r.s.state(), S::Idle);
    }

    #[test]
    fn ca35_dez_segundos_sem_fechar_oferece_forcar() {
        let mut r = rig_in(S::Closing);
        assert!(r.advance(9_999).is_empty());
        assert_eq!(r.s.state(), S::Closing);
        r.advance(1);
        assert_eq!(r.s.state(), S::CloseOverdue);
    }

    #[test]
    fn close_overdue_termina_sozinho_se_os_processos_acabarem() {
        let mut r = rig_in(S::CloseOverdue);
        let fx = r.s.processes_gone(r.now());
        assert_eq!(
            fx,
            vec![E::BringLauncherFront {
                focus: Some(portal())
            }]
        );
        assert_eq!(r.s.state(), S::Idle);
    }

    #[test]
    fn force_close_so_em_close_overdue_com_o_codigo_certo_em_cada_estado() {
        // Contrato secao 4, coluna `session_force_close` (RN-12, INV-05).
        let expected = [
            (S::Idle, Err(AppError::NoActiveSession)),
            (S::Launching, Err(AppError::NotOverdue)),
            (S::NeedsElevation, Err(AppError::NoActiveSession)),
            (S::NotConfirmed, Err(AppError::NoActiveSession)),
            (S::Running, Err(AppError::NotOverdue)),
            (S::Ending, Err(AppError::NotOverdue)),
            (S::Closing, Err(AppError::NotOverdue)),
            (S::CloseOverdue, Ok(vec![E::ForceClose])),
        ];
        for (state, want) in expected {
            let mut r = rig_in(state);
            let before = r.s.snapshot();
            assert_eq!(r.s.force_close(), want, "{state:?}");
            assert_eq!(
                r.s.snapshot(),
                before,
                "forcar nao muda o estado; so os processos terminando"
            );
        }
    }

    #[test]
    fn nenhum_caminho_emite_force_close_sem_o_comando() {
        // Percorre o tempo todo em Closing/CloseOverdue: forcar nunca sai sozinho.
        let mut r = rig_in(S::Closing);
        for _ in 0..100 {
            let fx = r.advance(1_000);
            assert!(!fx.contains(&E::ForceClose));
        }
        assert_eq!(r.s.state(), S::CloseOverdue);
    }

    // ---- dismiss (CA-77, CA-78, contrato secao 4) ----

    #[test]
    fn dismiss_segue_a_tabela_do_contrato_em_cada_estado() {
        // (estado, estado depois, dismissed depois)
        let expected = [
            (S::Idle, S::Idle, false),
            (S::Launching, S::Launching, true),
            (S::NeedsElevation, S::Idle, false),
            (S::NotConfirmed, S::Idle, false),
            (S::Running, S::Running, false),
            (S::Ending, S::Ending, false),
            (S::Closing, S::Closing, true),
            (S::CloseOverdue, S::Running, false),
        ];
        for (from, to, dismissed) in expected {
            let mut r = rig_in(from);
            let fx = r.s.dismiss(r.now());
            assert!(fx.is_empty(), "dismiss nunca cancela nada: {from:?}");
            assert_eq!(r.s.state(), to, "{from:?}");
            assert_eq!(r.s.snapshot().dismissed, dismissed, "{from:?}");
        }
    }

    #[test]
    fn dismiss_em_launching_nao_cancela_e_o_jogo_que_abrir_depois_segue_normal() {
        // CA-77: sair da espera, o jogo abre depois, mark_played e Running.
        let mut r = rig_in(S::Launching);
        r.s.dismiss(r.now());
        r.advance(20_000);
        assert!(r.s.snapshot().dismissed);
        let fx = r.s.process_seen(r.now());
        assert_eq!(fx, vec![E::MarkPlayed(portal())]);
        let snap = r.s.snapshot();
        assert_eq!(snap.state, S::Running);
        assert!(!snap.dismissed, "transicao de estado limpa o dismissed");
    }

    #[test]
    fn dismiss_em_closing_nao_cancela_o_fechamento_e_o_forcar_continua_disponivel() {
        // CA-78: depois de dispensar, o prazo de 10 s segue e leva a CloseOverdue.
        let mut r = rig_in(S::Closing);
        r.s.dismiss(r.now());
        r.advance(10_000);
        assert_eq!(r.s.state(), S::CloseOverdue);
        assert_eq!(r.s.force_close(), Ok(vec![E::ForceClose]));
    }

    #[test]
    fn cancelar_em_close_overdue_volta_a_running_e_permite_fechar_de_novo() {
        let mut r = rig_in(S::CloseOverdue);
        r.s.dismiss(r.now());
        assert_eq!(r.s.state(), S::Running);
        assert_eq!(r.s.request_close(r.now()), Ok(vec![E::RequestClose]));
    }

    // ---- retry / return_to_game ----

    #[test]
    fn retry_so_em_not_confirmed() {
        for state in ALL.into_iter().filter(|s| *s != S::NotConfirmed) {
            let mut r = rig_in(state);
            assert_eq!(r.s.retry(r.now()), Err(AppError::InvalidState), "{state:?}");
            assert_eq!(r.s.state(), state);
        }
    }

    #[test]
    fn return_to_game_so_em_running() {
        for state in ALL {
            let r = rig_in(state);
            let got = r.s.return_to_game();
            match state {
                S::Running => assert_eq!(got, Ok(vec![E::FrontGame])),
                S::Idle => assert_eq!(got, Err(AppError::NoActiveSession)),
                _ => assert_eq!(got, Err(AppError::InvalidState), "{state:?}"),
            }
        }
    }

    // ---- replace (CT-01, CA-31) ----

    #[test]
    fn replace_pede_so_o_fechamento_normal_e_abre_o_novo_depois_de_idle() {
        let mut r = rig_in(S::Running);
        let novo = gid("epic:SampleGameA");
        let fx = r.s.replace(novo.clone(), r.now()).unwrap();
        assert_eq!(fx, vec![E::RequestClose], "so WM_CLOSE, nunca forcar");
        assert_eq!(r.s.state(), S::Closing);
        assert_eq!(r.s.pending_replace(), Some(&novo));
        // O jogo fecha dentro do prazo.
        r.advance(2_000);
        let fx = r.s.processes_gone(r.now());
        assert_eq!(
            fx,
            vec![
                E::BringLauncherFront {
                    focus: Some(portal())
                },
                E::LaunchPending(novo)
            ]
        );
        assert_eq!(r.s.state(), S::Idle);
        assert_eq!(r.s.pending_replace(), None);
    }

    #[test]
    fn replace_com_jogo_que_ignora_wm_close_nao_forca_e_descarta_o_pendente() {
        // CT-01: jogo de teste que ignora WM_CLOSE + session_replace nunca e encerrado.
        let mut r = rig_in(S::Running);
        let novo = gid("epic:SampleGameA");
        let mut all_effects = r.s.replace(novo, r.now()).unwrap();
        for _ in 0..30 {
            all_effects.extend(r.advance(1_000));
        }
        assert_eq!(r.s.state(), S::CloseOverdue);
        assert_eq!(r.s.pending_replace(), None, "pendente descartado");
        assert!(
            !all_effects.contains(&E::ForceClose),
            "nenhum encerramento forcado"
        );
        assert!(
            all_effects
                .iter()
                .all(|e| !matches!(e, E::LaunchPending(_))),
            "novo jogo nao abre"
        );
        // Quando o jogo enfim fechar, o novo jogo NAO abre sozinho.
        let fx = r.s.processes_gone(r.now());
        assert_eq!(
            fx,
            vec![E::BringLauncherFront {
                focus: Some(portal())
            }]
        );
    }

    #[test]
    fn replace_em_close_overdue_e_em_qualquer_outro_estado_e_invalid_state() {
        for state in ALL.into_iter().filter(|s| *s != S::Running) {
            let mut r = rig_in(state);
            let before = r.s.snapshot();
            assert_eq!(
                r.s.replace(gid("epic:SampleGameA"), r.now()),
                Err(AppError::InvalidState),
                "{state:?}"
            );
            assert_eq!(r.s.snapshot(), before);
            assert_eq!(r.s.pending_replace(), None);
        }
    }

    #[test]
    fn replace_depois_cancelar_em_overdue_nao_deixa_pendente_vazando_para_a_proxima_sessao() {
        let mut r = rig_in(S::Running);
        r.s.replace(gid("epic:SampleGameA"), r.now()).unwrap();
        r.advance(10_000); // CloseOverdue, pendente descartado
        r.s.dismiss(r.now()); // Cancelar -> Running
        r.s.request_close(r.now()).unwrap();
        let fx = r.s.processes_gone(r.now());
        assert_eq!(
            fx,
            vec![E::BringLauncherFront {
                focus: Some(portal())
            }]
        );
    }

    #[test]
    fn dismiss_em_closing_de_um_replace_mantem_o_pendente() {
        let mut r = rig_in(S::Running);
        let novo = gid("epic:SampleGameA");
        r.s.replace(novo.clone(), r.now()).unwrap();
        r.s.dismiss(r.now());
        assert_eq!(r.s.pending_replace(), Some(&novo));
    }

    // ---- minimizado / config ----

    #[test]
    fn minimizado_so_vale_com_sessao_ativa_e_some_ao_encerrar() {
        let mut r = Rig::new();
        r.s.set_minimized(true);
        assert!(!r.s.snapshot().minimized);
        let mut r = rig_in(S::Running);
        r.s.set_minimized(true);
        assert!(r.s.snapshot().minimized);
        r.s.processes_gone(r.now());
        r.advance(5_000);
        assert!(!r.s.snapshot().minimized);
    }

    #[test]
    fn prazos_vem_de_advanced_session_em_segundos() {
        let settings = SessionSettings {
            start_timeout_seconds: 30,
            end_grace_seconds: 2,
            close_timeout_seconds: 5,
            probe_interval_ms: 1000,
            running_probe_interval_ms: 2000,
            use_steam_running_app_id: true,
        };
        let cfg = SessionConfig::from(&settings);
        assert_eq!(
            cfg,
            SessionConfig {
                start_timeout_ms: 30_000,
                end_grace_ms: 2_000,
                close_timeout_ms: 5_000
            }
        );
        let mut s = Session::new(cfg);
        s.launch(portal(), Millis(0)).unwrap();
        s.tick(Millis(29_999));
        assert_eq!(s.state(), S::Launching);
        s.tick(Millis(30_000));
        assert_eq!(s.state(), S::NotConfirmed);
    }

    #[test]
    fn relogio_que_anda_para_tras_nao_adianta_nenhum_prazo() {
        let mut s = Session::new(SessionConfig::default());
        s.launch(portal(), Millis(100_000)).unwrap();
        s.tick(Millis(5)); // relogio "voltou"
        assert_eq!(s.state(), S::Launching);
    }

    #[test]
    fn nova_sessao_depois_de_idle_comeca_limpa() {
        let mut r = rig_in(S::NeedsElevation);
        r.s.launch_with_uac(&portal(), r.now()).unwrap();
        r.s.elevation_denied(r.now());
        r.s.dismiss(r.now()); // Voltar -> Idle
        assert_eq!(r.s.snapshot().launch_failure, None);
        r.s.launch(gid("gog:1000000001"), r.now()).unwrap();
        let snap = r.s.snapshot();
        assert_eq!(snap.game_id, Some(gid("gog:1000000001")));
        assert!(!snap.elevated && snap.launch_failure.is_none() && !snap.minimized);
    }
}
