//! `AudioEndpoint` sobre o Core Audio (BE-32, CA-38, CA-41, CA-42, RN-13, SEC-27): volume mestre do
//! dispositivo de saida padrao.
//!
//! Tudo que e COM vive numa **thread MTA propria** (`sofa-audio`), criada na construcao: nenhum
//! objeto COM atravessa threads e a thread da janela nunca espera o driver de audio. Quem chama
//! (`state`, `set`) manda uma mensagem e espera a resposta por no maximo [`CALL_TIMEOUT`].
//!
//! - `IAudioEndpointVolume` do dispositivo padrao (`eRender`, `eConsole`);
//! - `IAudioEndpointVolumeCallback`: mudanca **externa** (teclado, mixer) vira `AudioListener`.
//!   Mudanca feita por `set` carrega um GUID de contexto nosso e e ignorada no callback (o caso de
//!   uso publica o resultado uma vez so);
//! - `IMMNotificationClient`: troca, entrada ou saida de dispositivo religa o endpoint na thread
//!   de audio e publica o novo estado (ou `None`, sem dispositivo).
//!
//! O callback do COM roda numa thread do pool do RPC: so le os dados, avisa o ouvinte (que nao
//! bloqueia) ou posta uma mensagem para a thread de audio. Nunca chama `state`/`set` (travaria).

use std::sync::mpsc::{self, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex, PoisonError};
use std::thread::JoinHandle;
use std::time::Duration;

use launcher_core::application::ports::{AudioEndpoint, AudioListener};
use launcher_core::application::AppError;
use launcher_core::domain::volume::VolumeState;
use windows::core::{implement, GUID, PCWSTR};
use windows::Win32::Foundation::BOOL;
use windows::Win32::Media::Audio::Endpoints::{
    IAudioEndpointVolume, IAudioEndpointVolumeCallback, IAudioEndpointVolumeCallback_Impl,
};
use windows::Win32::Media::Audio::{
    eConsole, eRender, EDataFlow, ERole, IMMDeviceEnumerator, IMMNotificationClient,
    IMMNotificationClient_Impl, MMDeviceEnumerator, AUDIO_VOLUME_NOTIFICATION_DATA, DEVICE_STATE,
};
use windows::Win32::System::Com::{
    CoCreateInstance, CoInitializeEx, CoUninitialize, CLSCTX_ALL, COINIT_MULTITHREADED,
};
use windows::Win32::UI::Shell::PropertiesSystem::PROPERTYKEY;

/// Tempo maximo que um comando espera a thread de audio (driver travado nao trava a UI).
const CALL_TIMEOUT: Duration = Duration::from_secs(2);

/// Contexto das gravacoes do Sofa: o callback reconhece e ignora o proprio eco.
const OWN_CONTEXT: GUID = GUID::from_u128(0x5e0f_ac0d_4a1d_9b1e_5f0a_0000_0000_0001);

// ---------------------------------------------------------------------------- conversoes puras

/// `0.0..=1.0` do Core Audio para `0..=100`, arredondado e preso a faixa. NaN e negativo = 0.
pub fn level_from_scalar(scalar: f32) -> u8 {
    if scalar.is_nan() || scalar <= 0.0 {
        return 0;
    }
    (scalar * 100.0).round().min(100.0) as u8
}

/// `0..=100` para o escalar do Core Audio.
pub fn scalar_from_level(level: u8) -> f32 {
    f32::from(level.min(100)) / 100.0
}

// ----------------------------------------------------------------------------------- mensagens

enum Msg {
    Get(Sender<Option<VolumeState>>),
    Set(VolumeState, Sender<Result<(), AppError>>),
    /// Dispositivo padrao mudou, entrou ou saiu.
    DeviceChanged,
    Stop,
}

type ListenerSlot = Arc<Mutex<Option<Arc<dyn AudioListener>>>>;

fn notify(slot: &ListenerSlot, state: Option<VolumeState>) {
    let listener = slot.lock().unwrap_or_else(PoisonError::into_inner).clone();
    if let Some(l) = listener {
        l.changed(state);
    }
}

// ---------------------------------------------------------------------------- callbacks do COM

#[implement(IAudioEndpointVolumeCallback)]
struct VolumeCallback {
    listener: ListenerSlot,
}

impl IAudioEndpointVolumeCallback_Impl for VolumeCallback_Impl {
    fn OnNotify(&self, pnotify: *mut AUDIO_VOLUME_NOTIFICATION_DATA) -> windows::core::Result<()> {
        // SAFETY: o Core Audio passa um ponteiro valido durante a chamada; so lemos campos Copy.
        let Some(data) = (unsafe { pnotify.as_ref() }) else {
            return Ok(());
        };
        if data.guidEventContext == OWN_CONTEXT {
            return Ok(());
        }
        notify(
            &self.listener,
            Some(VolumeState::new(
                level_from_scalar(data.fMasterVolume),
                data.bMuted.as_bool(),
            )),
        );
        Ok(())
    }
}

#[implement(IMMNotificationClient)]
struct DeviceNotifier {
    tx: Sender<Msg>,
}

impl DeviceNotifier_Impl {
    fn poke(&self) {
        let _ = self.tx.send(Msg::DeviceChanged);
    }
}

impl IMMNotificationClient_Impl for DeviceNotifier_Impl {
    fn OnDeviceStateChanged(
        &self,
        _id: &PCWSTR,
        _state: DEVICE_STATE,
    ) -> windows::core::Result<()> {
        self.poke();
        Ok(())
    }
    fn OnDeviceAdded(&self, _id: &PCWSTR) -> windows::core::Result<()> {
        self.poke();
        Ok(())
    }
    fn OnDeviceRemoved(&self, _id: &PCWSTR) -> windows::core::Result<()> {
        self.poke();
        Ok(())
    }
    fn OnDefaultDeviceChanged(
        &self,
        flow: EDataFlow,
        role: ERole,
        _id: &PCWSTR,
    ) -> windows::core::Result<()> {
        if flow == eRender && role == eConsole {
            self.poke();
        }
        Ok(())
    }
    fn OnPropertyValueChanged(
        &self,
        _id: &PCWSTR,
        _key: &PROPERTYKEY,
    ) -> windows::core::Result<()> {
        Ok(()) // muda toda hora (medidor); nao interessa
    }
}

// --------------------------------------------------------------------------- thread de audio

/// Endpoint ligado: o objeto de volume e o callback registrado nele.
struct Bound {
    volume: IAudioEndpointVolume,
    callback: IAudioEndpointVolumeCallback,
}

impl Bound {
    fn read(&self) -> Option<VolumeState> {
        // SAFETY: chamadas COM na thread dona do objeto.
        unsafe {
            let level = self.volume.GetMasterVolumeLevelScalar().ok()?;
            let muted = self.volume.GetMute().ok()?.as_bool();
            Some(VolumeState::new(level_from_scalar(level), muted))
        }
    }

    fn write(&self, state: VolumeState) -> Result<(), AppError> {
        // SAFETY: chamadas COM na thread dona do objeto; o GUID vive durante a chamada.
        unsafe {
            self.volume
                .SetMasterVolumeLevelScalar(scalar_from_level(state.level), &OWN_CONTEXT)
                .map_err(|e| {
                    tracing::warn!(%e, "audio: nao gravou o volume");
                    AppError::Internal
                })?;
            self.volume
                .SetMute(BOOL::from(state.muted), &OWN_CONTEXT)
                .map_err(|e| {
                    tracing::warn!(%e, "audio: nao gravou o mudo");
                    AppError::Internal
                })
        }
    }
}

impl Drop for Bound {
    fn drop(&mut self) {
        // SAFETY: desregistra o mesmo callback registrado em `bind`.
        unsafe {
            let _ = self.volume.UnregisterControlChangeNotify(&self.callback);
        }
    }
}

fn bind(enumerator: &IMMDeviceEnumerator, slot: &ListenerSlot) -> Option<Bound> {
    // SAFETY: chamadas COM na thread MTA de audio.
    unsafe {
        let device = enumerator.GetDefaultAudioEndpoint(eRender, eConsole).ok()?;
        let volume: IAudioEndpointVolume = device.Activate(CLSCTX_ALL, None).ok()?;
        let callback: IAudioEndpointVolumeCallback = VolumeCallback {
            listener: Arc::clone(slot),
        }
        .into();
        volume.RegisterControlChangeNotify(&callback).ok()?;
        Some(Bound { volume, callback })
    }
}

fn run(rx: &mpsc::Receiver<Msg>, tx: Sender<Msg>, slot: &ListenerSlot, ready: &Sender<bool>) {
    // SAFETY: inicializa o COM nesta thread e desfaz no fim; todos os objetos morrem antes.
    let init = unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) };
    if init.is_err() {
        tracing::error!("audio: CoInitializeEx falhou");
        let _ = ready.send(false);
        return;
    }
    {
        // SAFETY: criacao do enumerador na thread MTA.
        let enumerator: Option<IMMDeviceEnumerator> =
            unsafe { CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL).ok() };
        let Some(enumerator) = enumerator else {
            tracing::error!("audio: IMMDeviceEnumerator indisponivel");
            let _ = ready.send(false);
            // SAFETY: par do CoInitializeEx acima.
            unsafe { CoUninitialize() };
            return;
        };
        let notifier: IMMNotificationClient = DeviceNotifier { tx }.into();
        // SAFETY: registro do cliente de notificacao; desregistrado antes de sair.
        let registered = unsafe {
            enumerator
                .RegisterEndpointNotificationCallback(&notifier)
                .is_ok()
        };
        if !registered {
            tracing::warn!("audio: sem aviso de troca de dispositivo");
        }
        let mut bound = bind(&enumerator, slot);
        tracing::info!(dispositivo = bound.is_some(), "audio: endpoint ligado");
        let _ = ready.send(true);

        while let Ok(msg) = rx.recv() {
            match msg {
                Msg::Get(reply) => {
                    let _ = reply.send(bound.as_ref().and_then(Bound::read));
                }
                Msg::Set(state, reply) => {
                    let result = match &bound {
                        Some(b) => b.write(state),
                        None => Err(AppError::NoAudioDevice),
                    };
                    let _ = reply.send(result);
                }
                Msg::DeviceChanged => {
                    // Varias notificacoes vem juntas: junta tudo que ja esta na fila.
                    let mut stop = false;
                    while let Ok(next) = rx.try_recv() {
                        match next {
                            Msg::DeviceChanged => {}
                            Msg::Stop => stop = true,
                            Msg::Get(reply) => {
                                let _ = reply.send(bound.as_ref().and_then(Bound::read));
                            }
                            Msg::Set(_, reply) => {
                                let _ = reply.send(Err(AppError::Internal));
                            }
                        }
                    }
                    drop(bound.take()); // desregistra o callback do dispositivo antigo
                    bound = bind(&enumerator, slot);
                    let state = bound.as_ref().and_then(Bound::read);
                    tracing::info!(dispositivo = bound.is_some(), "audio: dispositivo mudou");
                    notify(slot, state);
                    if stop {
                        break;
                    }
                }
                Msg::Stop => break,
            }
        }

        drop(bound);
        if registered {
            // SAFETY: desregistra o mesmo cliente registrado acima.
            unsafe {
                let _ = enumerator.UnregisterEndpointNotificationCallback(&notifier);
            }
        }
    }
    // SAFETY: par do CoInitializeEx acima.
    unsafe { CoUninitialize() };
}

// ------------------------------------------------------------------------------------ adapter

pub struct WinAudioEndpoint {
    tx: Sender<Msg>,
    listener: ListenerSlot,
    handle: Mutex<Option<JoinHandle<()>>>,
}

impl WinAudioEndpoint {
    /// Sobe a thread de audio. Sem Core Audio (COM falhou), devolve erro: o launcher segue sem
    /// volume (`volume_get` = indisponivel).
    pub fn spawn() -> std::io::Result<Self> {
        let (tx, rx) = mpsc::channel::<Msg>();
        let listener: ListenerSlot = Arc::new(Mutex::new(None));
        let (ready_tx, ready_rx) = mpsc::channel::<bool>();
        let slot = Arc::clone(&listener);
        let self_tx = tx.clone();
        let handle = std::thread::Builder::new()
            .name("sofa-audio".into())
            .spawn(move || run(&rx, self_tx, &slot, &ready_tx))?;
        match ready_rx.recv() {
            Ok(true) => Ok(Self {
                tx,
                listener,
                handle: Mutex::new(Some(handle)),
            }),
            _ => {
                let _ = handle.join();
                Err(std::io::Error::other("thread de audio nao iniciou"))
            }
        }
    }

    fn ask<T>(&self, make: impl FnOnce(Sender<T>) -> Msg) -> Option<T> {
        let (reply_tx, reply_rx) = mpsc::channel();
        self.tx.send(make(reply_tx)).ok()?;
        match reply_rx.recv_timeout(CALL_TIMEOUT) {
            Ok(v) => Some(v),
            Err(RecvTimeoutError::Timeout) => {
                tracing::warn!("audio: a thread de audio nao respondeu no prazo");
                None
            }
            Err(RecvTimeoutError::Disconnected) => None,
        }
    }
}

impl AudioEndpoint for WinAudioEndpoint {
    fn state(&self) -> Option<VolumeState> {
        self.ask(Msg::Get).flatten()
    }

    fn set(&self, state: VolumeState) -> Result<(), AppError> {
        self.ask(|reply| Msg::Set(state, reply))
            .unwrap_or(Err(AppError::Internal))
    }

    fn subscribe(&self, listener: Arc<dyn AudioListener>) {
        *self.listener.lock().unwrap_or_else(PoisonError::into_inner) = Some(listener);
    }
}

impl Drop for WinAudioEndpoint {
    fn drop(&mut self) {
        let _ = self.tx.send(Msg::Stop);
        if let Some(h) = self
            .handle
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .take()
        {
            let _ = h.join();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nivel_do_escalar_arredonda_e_prende() {
        for (escalar, nivel) in [
            (0.0, 0),
            (0.004, 0),
            (0.005, 1),
            (0.55, 55),
            (0.5499, 55),
            (0.9999, 100),
            (1.0, 100),
            (1.7, 100),
            (-0.3, 0),
            (f32::NAN, 0),
        ] {
            assert_eq!(level_from_scalar(escalar), nivel, "{escalar}");
        }
    }

    #[test]
    fn escalar_e_nivel_dao_ida_e_volta_em_todos_os_niveis() {
        for level in 0..=100u8 {
            assert_eq!(level_from_scalar(scalar_from_level(level)), level);
        }
        assert_eq!(scalar_from_level(250), 1.0);
    }
}
