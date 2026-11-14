//! Casos de uso de volume (BE-31, BE-32): `get`, `step_volume` e `toggle_mute` sobre o port
//! `AudioEndpoint`. A regra de passo e mudo e de `domain::volume`; aqui ficam a leitura, a escrita
//! serializada e a publicacao de `volume://changed` sem repeticao.

use std::sync::{Arc, Mutex, PoisonError};

use super::error::AppError;
use super::ports::{AudioEndpoint, AudioListener, VolumeEvents};
use crate::domain::volume::{apply, VolumeCommand, VolumeState};

/// O que a UI recebe (`VolumeState` do contrato). Sem dispositivo: `available = false` e nivel 0.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VolumeView {
    pub level: u8,
    pub muted: bool,
    pub available: bool,
}

impl VolumeView {
    pub const UNAVAILABLE: VolumeView = VolumeView {
        level: 0,
        muted: false,
        available: false,
    };

    fn of(state: Option<VolumeState>) -> Self {
        match state {
            Some(s) => Self {
                level: s.level,
                muted: s.muted,
                available: true,
            },
            None => Self::UNAVAILABLE,
        }
    }
}

pub struct VolumeService {
    endpoint: Arc<dyn AudioEndpoint>,
    events: Arc<dyn VolumeEvents>,
    /// Serializa ler-aplicar-gravar (dois LT seguidos nao se atropelam).
    op: Mutex<()>,
    /// Ultimo estado publicado: o mesmo estado nao vira evento de novo.
    last: Mutex<Option<VolumeView>>,
}

impl VolumeService {
    /// Cria o servico e o registra como ouvinte do endpoint.
    pub fn new(endpoint: Arc<dyn AudioEndpoint>, events: Arc<dyn VolumeEvents>) -> Arc<Self> {
        let service = Arc::new(Self {
            endpoint: Arc::clone(&endpoint),
            events,
            op: Mutex::new(()),
            last: Mutex::new(None),
        });
        endpoint.subscribe(service.clone());
        service
    }

    /// `volume_get`: nunca falha; sem dispositivo, `available = false`.
    pub fn get(&self) -> VolumeView {
        let view = VolumeView::of(self.endpoint.state());
        *self.last.lock().unwrap_or_else(PoisonError::into_inner) = Some(view);
        view
    }

    /// `volume_step`: um passo para cima ou para baixo (CA-38, CA-40, CA-64).
    pub fn step_volume(&self, up: bool) -> Result<VolumeView, AppError> {
        self.run(if up {
            VolumeCommand::Up
        } else {
            VolumeCommand::Down
        })
    }

    /// `volume_toggle_mute` (CA-39).
    pub fn toggle_mute(&self) -> Result<VolumeView, AppError> {
        self.run(VolumeCommand::ToggleMute)
    }

    fn run(&self, command: VolumeCommand) -> Result<VolumeView, AppError> {
        let _guard = self.op.lock().unwrap_or_else(PoisonError::into_inner);
        let before = self.endpoint.state().ok_or(AppError::NoAudioDevice)?;
        let after = apply(before, command);
        if after != before {
            self.endpoint.set(after)?;
        }
        let view = VolumeView::of(Some(after));
        self.publish(view);
        Ok(view)
    }

    fn publish(&self, view: VolumeView) {
        {
            let mut last = self.last.lock().unwrap_or_else(PoisonError::into_inner);
            if *last == Some(view) {
                return;
            }
            *last = Some(view);
        }
        self.events.changed(&view);
    }
}

impl AudioListener for VolumeService {
    fn changed(&self, state: Option<VolumeState>) {
        self.publish(VolumeView::of(state));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Default)]
    struct FakeEndpoint {
        state: Mutex<Option<VolumeState>>,
        writes: Mutex<Vec<VolumeState>>,
        listener: Mutex<Option<Arc<dyn AudioListener>>>,
    }

    impl FakeEndpoint {
        fn with(level: u8, muted: bool) -> Arc<Self> {
            let e = Arc::new(Self::default());
            *e.state.lock().unwrap() = Some(VolumeState { level, muted });
            e
        }
        /// O usuario mexeu no teclado ou no mixer.
        fn external(&self, state: Option<VolumeState>) {
            *self.state.lock().unwrap() = state;
            let l = self.listener.lock().unwrap().clone().expect("ouvinte");
            l.changed(state);
        }
    }

    impl AudioEndpoint for FakeEndpoint {
        fn state(&self) -> Option<VolumeState> {
            *self.state.lock().unwrap()
        }
        fn set(&self, state: VolumeState) -> Result<(), AppError> {
            if self.state.lock().unwrap().is_none() {
                return Err(AppError::NoAudioDevice);
            }
            *self.state.lock().unwrap() = Some(state);
            self.writes.lock().unwrap().push(state);
            Ok(())
        }
        fn subscribe(&self, listener: Arc<dyn AudioListener>) {
            *self.listener.lock().unwrap() = Some(listener);
        }
    }

    #[derive(Default)]
    struct Rec(Mutex<Vec<VolumeView>>);
    impl VolumeEvents for Rec {
        fn changed(&self, view: &VolumeView) {
            self.0.lock().unwrap().push(*view);
        }
    }

    fn setup(level: u8, muted: bool) -> (Arc<FakeEndpoint>, Arc<Rec>, Arc<VolumeService>) {
        let endpoint = FakeEndpoint::with(level, muted);
        let rec = Arc::new(Rec::default());
        let service = VolumeService::new(endpoint.clone(), rec.clone());
        (endpoint, rec, service)
    }

    fn view(level: u8, muted: bool) -> VolumeView {
        VolumeView {
            level,
            muted,
            available: true,
        }
    }

    #[test]
    fn step_grava_uma_vez_e_publica() {
        let (endpoint, rec, svc) = setup(50, false);
        assert_eq!(svc.step_volume(true), Ok(view(55, false)));
        assert_eq!(
            *endpoint.writes.lock().unwrap(),
            vec![VolumeState {
                level: 55,
                muted: false
            }]
        );
        assert_eq!(*rec.0.lock().unwrap(), vec![view(55, false)]);
    }

    #[test]
    fn rt_no_mudo_desmuta_e_sobe_e_lt_no_mudo_mantem_o_mudo() {
        let (_e, _r, svc) = setup(40, true);
        assert_eq!(svc.step_volume(false), Ok(view(35, true))); // CA-64
        assert_eq!(svc.step_volume(true), Ok(view(40, false))); // CA-40
    }

    #[test]
    fn mudo_alterna_e_devolve_o_nivel() {
        let (_e, _r, svc) = setup(30, false);
        assert_eq!(svc.toggle_mute(), Ok(view(30, true)));
        assert_eq!(svc.toggle_mute(), Ok(view(30, false)));
    }

    #[test]
    fn no_limite_nao_grava_e_nao_repete_evento() {
        let (endpoint, rec, svc) = setup(100, false);
        assert_eq!(svc.step_volume(true), Ok(view(100, false)));
        assert!(endpoint.writes.lock().unwrap().is_empty());
        // Primeiro estado conhecido: um evento; o mesmo estado de novo nao gera outro.
        assert_eq!(svc.step_volume(true), Ok(view(100, false)));
        assert_eq!(rec.0.lock().unwrap().len(), 1);
    }

    #[test]
    fn sem_dispositivo_get_indisponivel_e_acoes_no_audio_device() {
        let endpoint = Arc::new(FakeEndpoint::default());
        let svc = VolumeService::new(endpoint, Arc::new(Rec::default()));
        assert_eq!(svc.get(), VolumeView::UNAVAILABLE);
        assert_eq!(svc.step_volume(true), Err(AppError::NoAudioDevice));
        assert_eq!(svc.toggle_mute(), Err(AppError::NoAudioDevice));
    }

    #[test]
    fn mudanca_externa_vira_evento_uma_vez() {
        let (endpoint, rec, svc) = setup(50, false);
        svc.get();
        endpoint.external(Some(VolumeState {
            level: 70,
            muted: false,
        }));
        endpoint.external(Some(VolumeState {
            level: 70,
            muted: false,
        }));
        assert_eq!(*rec.0.lock().unwrap(), vec![view(70, false)]);
    }

    #[test]
    fn troca_de_dispositivo_sem_saida_publica_indisponivel_e_volta() {
        let (endpoint, rec, svc) = setup(50, false);
        svc.get();
        endpoint.external(None); // CA-42: o dispositivo padrao sumiu
        endpoint.external(Some(VolumeState {
            level: 20,
            muted: true,
        }));
        assert_eq!(
            *rec.0.lock().unwrap(),
            vec![VolumeView::UNAVAILABLE, view(20, true)]
        );
        assert_eq!(svc.get(), view(20, true));
    }

    #[test]
    fn evento_de_acao_propria_nao_duplica_com_o_ecoado_pelo_driver() {
        let (endpoint, rec, svc) = setup(50, false);
        svc.step_volume(true).unwrap();
        // Driver que ecoa a mudanca mesmo assim: igual ao ultimo publicado, nao repete.
        endpoint.external(Some(VolumeState {
            level: 55,
            muted: false,
        }));
        assert_eq!(rec.0.lock().unwrap().len(), 1);
    }
}
