//! Casos de uso de energia (BE-33, RN-14, RN-15, SEC-27, CT-07): `capabilities`, `prepare` (emite o
//! ticket) e `execute` (resgata e manda o Windows agir, sem forcar nada).

use std::sync::{Arc, Mutex, PoisonError};

use super::error::AppError;
use super::ports::{Clock, PowerControl, RandomSource};
use crate::domain::power::{
    encode_ticket, is_well_formed, PowerAction, TicketBook, TICKET_RANDOM_BYTES,
};

/// O que o PC permite agora (CA-47).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PowerCapabilities {
    pub shutdown: bool,
    pub restart: bool,
    pub suspend: bool,
}

impl PowerCapabilities {
    /// Nada disponivel (servico nao ligado).
    pub const NONE: PowerCapabilities = PowerCapabilities {
        shutdown: false,
        restart: false,
        suspend: false,
    };

    pub fn allows(self, action: PowerAction) -> bool {
        match action {
            PowerAction::Shutdown => self.shutdown,
            PowerAction::Restart => self.restart,
            PowerAction::Suspend => self.suspend,
        }
    }
}

pub struct PowerService {
    control: Arc<dyn PowerControl>,
    random: Arc<dyn RandomSource>,
    clock: Arc<dyn Clock>,
    tickets: Mutex<TicketBook>,
}

impl PowerService {
    pub fn new(
        control: Arc<dyn PowerControl>,
        random: Arc<dyn RandomSource>,
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self {
            control,
            random,
            clock,
            tickets: Mutex::new(TicketBook::new()),
        }
    }

    /// `power_capabilities` (#22).
    pub fn capabilities(&self) -> PowerCapabilities {
        self.control.capabilities()
    }

    /// `power_prepare` (#23): acao indisponivel -> `Unavailable`; senao emite um ticket novo, que
    /// invalida o anterior (CT-07).
    pub fn prepare(&self, action: PowerAction) -> Result<String, AppError> {
        if !self.control.capabilities().allows(action) {
            return Err(AppError::Unavailable);
        }
        let mut bytes = [0u8; TICKET_RANDOM_BYTES];
        if self.random.fill(&mut bytes).is_err() {
            tracing::error!("energia: sem fonte de numeros aleatorios do sistema");
            return Err(AppError::Internal);
        }
        let ticket = encode_ticket(&bytes);
        debug_assert!(is_well_formed(&ticket));
        self.tickets
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .issue(ticket.clone(), action, self.clock.now());
        Ok(ticket)
    }

    /// `power_execute` (#24): o ticket manda; a acao e a que ficou presa a ele no `prepare`.
    /// Ticket invalido -> `TicketInvalid`; Windows recusou -> `Rejected`.
    pub fn execute(&self, ticket: &str) -> Result<(), AppError> {
        let action = self
            .tickets
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .redeem(ticket, self.clock.now())
            .map_err(|_| AppError::TicketInvalid)?;
        // A capacidade pode ter mudado entre o prepare e a confirmacao (ex.: suspensao desligada).
        if !self.control.capabilities().allows(action) {
            return Err(AppError::Rejected);
        }
        tracing::info!(acao = ?action, "energia: pedido ao Windows");
        self.control.execute(action)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::ports::ManualClock;
    use std::sync::atomic::{AtomicU8, Ordering};

    #[derive(Default)]
    struct FakeControl {
        caps: Mutex<Option<PowerCapabilities>>,
        executed: Mutex<Vec<PowerAction>>,
        refuse: Mutex<bool>,
    }

    impl FakeControl {
        fn all() -> Arc<Self> {
            let c = Arc::new(Self::default());
            *c.caps.lock().unwrap() = Some(PowerCapabilities {
                shutdown: true,
                restart: true,
                suspend: true,
            });
            c
        }
    }

    impl PowerControl for FakeControl {
        fn capabilities(&self) -> PowerCapabilities {
            self.caps.lock().unwrap().unwrap_or(PowerCapabilities::NONE)
        }
        fn execute(&self, action: PowerAction) -> Result<(), AppError> {
            if *self.refuse.lock().unwrap() {
                return Err(AppError::Rejected);
            }
            self.executed.lock().unwrap().push(action);
            Ok(())
        }
    }

    /// Contador determinista: cada chamada gera bytes diferentes.
    #[derive(Default)]
    struct CountingRandom(AtomicU8);
    impl RandomSource for CountingRandom {
        fn fill(&self, buf: &mut [u8]) -> Result<(), AppError> {
            let n = self.0.fetch_add(1, Ordering::SeqCst) + 1;
            buf.iter_mut().for_each(|b| *b = n);
            Ok(())
        }
    }

    struct BrokenRandom;
    impl RandomSource for BrokenRandom {
        fn fill(&self, _buf: &mut [u8]) -> Result<(), AppError> {
            Err(AppError::Internal)
        }
    }

    fn setup() -> (Arc<FakeControl>, Arc<ManualClock>, PowerService) {
        let control = FakeControl::all();
        let clock = Arc::new(ManualClock::new());
        let svc = PowerService::new(
            control.clone(),
            Arc::new(CountingRandom::default()),
            clock.clone(),
        );
        (control, clock, svc)
    }

    #[test]
    fn prepare_e_execute_chamam_o_windows_com_a_acao_do_ticket() {
        let (control, _clock, svc) = setup();
        let t = svc.prepare(PowerAction::Restart).unwrap();
        assert!(is_well_formed(&t));
        assert_eq!(svc.execute(&t), Ok(()));
        assert_eq!(
            *control.executed.lock().unwrap(),
            vec![PowerAction::Restart]
        );
    }

    #[test]
    fn ticket_reutilizado_ou_expirado_e_ticket_invalid() {
        let (control, clock, svc) = setup();
        let t = svc.prepare(PowerAction::Shutdown).unwrap();
        svc.execute(&t).unwrap();
        assert_eq!(svc.execute(&t), Err(AppError::TicketInvalid));
        assert_eq!(control.executed.lock().unwrap().len(), 1);

        let t = svc.prepare(PowerAction::Shutdown).unwrap();
        clock.advance_ms(30_001);
        assert_eq!(svc.execute(&t), Err(AppError::TicketInvalid));
        assert_eq!(control.executed.lock().unwrap().len(), 1);
    }

    #[test]
    fn ticket_desconhecido_ou_mal_formado_nao_executa_nada() {
        let (control, _clock, svc) = setup();
        for t in [
            "",
            "curto",
            &"x".repeat(43),
            "ticket com espaco e simbolos!",
        ] {
            assert_eq!(svc.execute(t), Err(AppError::TicketInvalid), "{t}");
        }
        assert!(control.executed.lock().unwrap().is_empty());
    }

    /// CT-07: o ticket da acao A nunca executa a acao B, e o `prepare` novo invalida o velho.
    #[test]
    fn novo_prepare_invalida_o_anterior_e_a_acao_fica_presa_ao_ticket() {
        let (control, _clock, svc) = setup();
        let velho = svc.prepare(PowerAction::Shutdown).unwrap();
        let novo = svc.prepare(PowerAction::Suspend).unwrap();
        assert_ne!(velho, novo);
        assert_eq!(svc.execute(&velho), Err(AppError::TicketInvalid));
        assert_eq!(svc.execute(&novo), Ok(()));
        assert_eq!(
            *control.executed.lock().unwrap(),
            vec![PowerAction::Suspend]
        );
    }

    #[test]
    fn suspensao_indisponivel_e_unavailable_e_nao_emite_ticket() {
        let (control, _clock, svc) = setup();
        *control.caps.lock().unwrap() = Some(PowerCapabilities {
            shutdown: true,
            restart: true,
            suspend: false,
        });
        assert_eq!(
            svc.prepare(PowerAction::Suspend),
            Err(AppError::Unavailable)
        );
        assert!(svc.prepare(PowerAction::Shutdown).is_ok());
    }

    #[test]
    fn capacidade_que_some_entre_prepare_e_execute_e_rejected() {
        let (control, _clock, svc) = setup();
        let t = svc.prepare(PowerAction::Suspend).unwrap();
        control.caps.lock().unwrap().as_mut().unwrap().suspend = false;
        assert_eq!(svc.execute(&t), Err(AppError::Rejected));
        assert!(control.executed.lock().unwrap().is_empty());
    }

    #[test]
    fn windows_recusou_vira_rejected_e_o_ticket_nao_volta() {
        let (control, _clock, svc) = setup();
        *control.refuse.lock().unwrap() = true;
        let t = svc.prepare(PowerAction::Restart).unwrap();
        assert_eq!(svc.execute(&t), Err(AppError::Rejected));
        assert_eq!(svc.execute(&t), Err(AppError::TicketInvalid));
    }

    #[test]
    fn sem_fonte_aleatoria_nao_emite_ticket() {
        let svc = PowerService::new(
            FakeControl::all(),
            Arc::new(BrokenRandom),
            Arc::new(ManualClock::new()),
        );
        assert_eq!(svc.prepare(PowerAction::Shutdown), Err(AppError::Internal));
    }

    #[test]
    fn tickets_emitidos_sao_distintos() {
        let (_c, _clk, svc) = setup();
        let a = svc.prepare(PowerAction::Shutdown).unwrap();
        let b = svc.prepare(PowerAction::Shutdown).unwrap();
        assert_ne!(a, b);
        assert_eq!(a.len(), 43);
    }
}
