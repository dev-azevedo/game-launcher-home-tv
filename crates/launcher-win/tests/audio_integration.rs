//! BE-32 no Core Audio de verdade (`--features win-integration`): comando -> evento em < 200 ms e
//! volume mudado por fora (teclado, mixer) -> `volume://changed`. O volume e o mudo originais do
//! usuario sao lidos antes e **restaurados** ao fim, mesmo se um assert falhar (guarda no `Drop`).
//! Sem dispositivo de saida, o teste confere `available = false` e `NoAudioDevice`.
#![cfg(feature = "win-integration")]

use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use launcher_core::application::ports::{AudioEndpoint, AudioListener, VolumeEvents};
use launcher_core::application::{AppError, VolumeService, VolumeView};
use launcher_core::domain::volume::VolumeState;
use launcher_win::audio::{level_from_scalar, WinAudioEndpoint};
use windows::Win32::Media::Audio::Endpoints::IAudioEndpointVolume;
use windows::Win32::Media::Audio::{eConsole, eRender, IMMDeviceEnumerator, MMDeviceEnumerator};
use windows::Win32::System::Com::{
    CoCreateInstance, CoInitializeEx, CoUninitialize, CLSCTX_ALL, COINIT_MULTITHREADED,
};

struct Events(Mutex<Sender<VolumeView>>);
impl VolumeEvents for Events {
    fn changed(&self, view: &VolumeView) {
        let _ = self.0.lock().unwrap().send(*view);
    }
}

/// Devolve o volume e o mudo do usuario no `Drop`.
struct Restore {
    endpoint: Arc<WinAudioEndpoint>,
    original: VolumeState,
}
impl Drop for Restore {
    fn drop(&mut self) {
        let _ = self.endpoint.set(self.original);
        let now = self.endpoint.state();
        assert_eq!(now, Some(self.original), "volume original nao restaurado");
    }
}

fn wait(rx: &Receiver<VolumeView>, within: Duration) -> Option<VolumeView> {
    rx.recv_timeout(within).ok()
}

#[test]
fn comando_gera_evento_em_menos_de_200_ms_e_o_valor_do_usuario_e_restaurado() {
    let endpoint = Arc::new(WinAudioEndpoint::spawn().expect("thread de audio"));
    let (tx, rx) = channel();
    let service = VolumeService::new(
        endpoint.clone() as Arc<dyn AudioEndpoint>,
        Arc::new(Events(Mutex::new(tx))),
    );

    let Some(original) = endpoint.state() else {
        // Maquina sem saida de audio (CA-42 / sem dispositivo).
        assert_eq!(service.get(), VolumeView::UNAVAILABLE);
        assert_eq!(service.step_volume(true), Err(AppError::NoAudioDevice));
        assert_eq!(service.toggle_mute(), Err(AppError::NoAudioDevice));
        return;
    };
    let _restore = Restore {
        endpoint: endpoint.clone(),
        original,
    };
    eprintln!("original: {original:?}");
    assert!(service.get().available);

    // Passo (para baixo se ja estiver perto do topo): 5 pontos, evento em < 200 ms.
    let up = original.level <= 90;
    let t0 = Instant::now();
    let after = service.step_volume(up).expect("passo");
    let event = wait(&rx, Duration::from_millis(200)).expect("volume://changed em 200 ms");
    assert!(
        t0.elapsed() < Duration::from_millis(200),
        "{:?}",
        t0.elapsed()
    );
    let expected = if up {
        original.level + 5
    } else {
        original.level.saturating_sub(5)
    };
    assert_eq!(after.level, expected);
    assert_eq!(event, after);
    // O que o Windows tem agora e o mesmo que o evento disse.
    assert_eq!(endpoint.state().unwrap().level, expected);

    // Mudo alterna e o nivel guardado e o mesmo (CA-39).
    let t0 = Instant::now();
    let muted = service.toggle_mute().expect("mudo");
    assert_eq!(muted.muted, !original.muted);
    assert_eq!(muted.level, expected);
    assert!(wait(&rx, Duration::from_millis(200)).is_some());
    assert!(t0.elapsed() < Duration::from_millis(200));
    let unmuted = service.toggle_mute().expect("desmudo");
    assert_eq!(unmuted.muted, original.muted);
    assert_eq!(unmuted.level, expected);
}

#[test]
fn volume_mudado_por_fora_chega_ao_ouvinte_em_menos_de_200_ms() {
    let endpoint = Arc::new(WinAudioEndpoint::spawn().expect("thread de audio"));
    let Some(original) = endpoint.state() else {
        return; // sem dispositivo: nada a observar
    };
    let _restore = Restore {
        endpoint: endpoint.clone(),
        original,
    };

    struct L(Mutex<Sender<Option<VolumeState>>>);
    impl AudioListener for L {
        fn changed(&self, state: Option<VolumeState>) {
            let _ = self.0.lock().unwrap().send(state);
        }
    }
    let (tx, rx) = channel();
    endpoint.subscribe(Arc::new(L(Mutex::new(tx))));

    // "O teclado": outro cliente do mesmo endpoint, sem o contexto do Sofa.
    let target = if original.level <= 90 {
        original.level + 5
    } else {
        original.level - 5
    };
    let target_scalar = f32::from(target) / 100.0;
    let t0 = Instant::now();
    std::thread::spawn(move || {
        // SAFETY: COM proprio desta thread de teste; todos os objetos morrem antes do uninit.
        unsafe {
            CoInitializeEx(None, COINIT_MULTITHREADED).ok().unwrap();
            {
                let en: IMMDeviceEnumerator =
                    CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL).unwrap();
                let dev = en.GetDefaultAudioEndpoint(eRender, eConsole).unwrap();
                let vol: IAudioEndpointVolume = dev.Activate(CLSCTX_ALL, None).unwrap();
                vol.SetMasterVolumeLevelScalar(target_scalar, std::ptr::null())
                    .unwrap();
                assert_eq!(
                    level_from_scalar(vol.GetMasterVolumeLevelScalar().unwrap()),
                    target
                );
            }
            CoUninitialize();
        }
    })
    .join()
    .expect("cliente externo");
    let got = rx
        .recv_timeout(Duration::from_millis(200))
        .expect("callback de volume externo em 200 ms");
    assert_eq!(got.map(|s| s.level), Some(target));
    assert!(
        t0.elapsed() < Duration::from_millis(400),
        "{:?}",
        t0.elapsed()
    );
}
