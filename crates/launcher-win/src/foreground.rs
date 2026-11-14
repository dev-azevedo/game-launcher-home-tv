//! `WindowSystem` Win32 da escada de primeiro plano (BE-18, ADR-0003, SEC-28 rev. 5). **Unico**
//! arquivo com `SetForegroundWindow`, `SendInput`, `AttachThreadInput` e `BringWindowToTop`
//! (INV-03, INV-08). Cada metodo e um passo; a ordem e as condicoes sao do `ForegroundBroker`
//! (`launcher-core`).
//!
//! Nada aqui muda configuracao do sistema: sem `SPI_SETFOREGROUNDLOCKTIMEOUT`, sem hook global,
//! sem `uiAccess`, sem injecao.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc};
use std::time::Duration;

use launcher_core::application::ports::{Step3Result, WindowSystem};
use launcher_core::domain::settings::Step2Key;
use windows::Win32::Foundation::{CloseHandle, HANDLE, HWND};
use windows::Win32::Security::{
    GetSidSubAuthority, GetSidSubAuthorityCount, GetTokenInformation, TokenIntegrityLevel,
    TOKEN_MANDATORY_LABEL, TOKEN_QUERY,
};
use windows::Win32::System::StationsAndDesktops::{
    CloseDesktop, GetUserObjectInformationW, OpenInputDesktop, DESKTOP_CONTROL_FLAGS,
    DESKTOP_READOBJECTS, UOI_NAME,
};
use windows::Win32::System::Threading::{
    AttachThreadInput, GetCurrentProcess, GetCurrentThreadId, OpenProcess, OpenProcessToken,
    PROCESS_QUERY_LIMITED_INFORMATION,
};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYBD_EVENT_FLAGS, KEYEVENTF_KEYUP,
    VIRTUAL_KEY, VK_F24, VK_MENU,
};
use windows::Win32::UI::WindowsAndMessaging::{
    AllowSetForegroundWindow, BringWindowToTop, GetForegroundWindow, GetWindowThreadProcessId,
    IsIconic, SetForegroundWindow, SetWindowPos, ShowWindow, ShowWindowAsync, ASFW_ANY,
    HWND_NOTOPMOST, HWND_TOPMOST, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SWP_SHOWWINDOW,
    SW_MINIMIZE, SW_RESTORE,
};

use crate::windows_enum::{hwnd as to_hwnd, top_level_windows};

// ------------------------------------------------------------------------------ degrau 3

/// As quatro chamadas do degrau 3 atras de um trait, para testar a guarda RAII sem Win32.
pub trait AttachOps: Send + Sync + 'static {
    fn foreground_thread(&self) -> u32;
    fn current_thread(&self) -> u32;
    fn attach(&self, from: u32, to: u32) -> bool;
    fn detach(&self, from: u32, to: u32);
    /// `SetForegroundWindow` + `BringWindowToTop` na janela do Sofa.
    fn bring_launcher(&self);
}

/// Desanexa no `Drop`: em sucesso, em erro (`panic`) e quando a thread volta depois do tempo
/// limite (SEC-28: o anexo nunca fica pendurado).
struct DetachGuard<'a, O: AttachOps> {
    ops: &'a O,
    from: u32,
    to: u32,
    attached: bool,
}

impl<O: AttachOps> Drop for DetachGuard<'_, O> {
    fn drop(&mut self) {
        if self.attached {
            self.ops.detach(self.from, self.to);
        }
    }
}

fn attach_and_bring<O: AttachOps>(ops: &O) -> bool {
    let to = ops.foreground_thread();
    let from = ops.current_thread();
    let attached = to != 0 && ops.attach(from, to);
    let _guard = DetachGuard {
        ops,
        from,
        to,
        attached,
    };
    ops.bring_launcher();
    attached
}

/// Libera a marca de "thread pendente" mesmo se a thread entrar em `panic`.
struct PendingGuard(Arc<AtomicBool>);

impl Drop for PendingGuard {
    fn drop(&mut self) {
        self.0.store(false, Ordering::Release);
    }
}

/// No maximo uma thread do degrau 3 pendente (SEC-28): se a anterior (thread do jogo travada)
/// ainda nao voltou, a nova tentativa nao cria outra e devolve `Pending`.
pub struct Step3Runner<O: AttachOps> {
    ops: Arc<O>,
    pending: Arc<AtomicBool>,
}

impl<O: AttachOps> Step3Runner<O> {
    pub fn new(ops: O) -> Self {
        Self {
            ops: Arc::new(ops),
            pending: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn run(&self, timeout: Duration) -> Step3Result {
        if self.pending.swap(true, Ordering::AcqRel) {
            return Step3Result::Pending;
        }
        let (tx, rx) = mpsc::channel();
        let ops = Arc::clone(&self.ops);
        let pending = PendingGuard(Arc::clone(&self.pending));
        let spawned = std::thread::Builder::new()
            .name("foreground-step3".into())
            .spawn(move || {
                let _pending = pending;
                let ok = attach_and_bring(&*ops);
                let _ = tx.send(ok);
            });
        if spawned.is_err() {
            self.pending.store(false, Ordering::Release);
            return Step3Result::Failed;
        }
        match rx.recv_timeout(timeout) {
            Ok(true) => Step3Result::Done,
            Ok(false) | Err(mpsc::RecvTimeoutError::Disconnected) => Step3Result::Failed,
            Err(mpsc::RecvTimeoutError::Timeout) => Step3Result::TimedOut,
        }
    }
}

// ------------------------------------------------------------------------------- Win32

struct Win32Attach {
    launcher: isize,
}

impl AttachOps for Win32Attach {
    fn foreground_thread(&self) -> u32 {
        // SAFETY: consultas sem efeito colateral.
        unsafe {
            let fg = GetForegroundWindow();
            if fg.0.is_null() {
                0
            } else {
                GetWindowThreadProcessId(fg, None)
            }
        }
    }

    fn current_thread(&self) -> u32 {
        // SAFETY: sem argumentos.
        unsafe { GetCurrentThreadId() }
    }

    fn attach(&self, from: u32, to: u32) -> bool {
        // SAFETY: ids de thread; pareado com `detach` pela guarda.
        unsafe { AttachThreadInput(from, to, true).as_bool() }
    }

    fn detach(&self, from: u32, to: u32) {
        // SAFETY: desfaz o `attach` acima.
        unsafe {
            let _ = AttachThreadInput(from, to, false);
        }
    }

    fn bring_launcher(&self) {
        let h = to_hwnd(self.launcher);
        // SAFETY: `h` e a janela do proprio Sofa.
        unsafe {
            let _ = SetForegroundWindow(h);
            let _ = BringWindowToTop(h);
        }
    }
}

/// Nivel de integridade (RID) do token de um processo. `None` se nao der para consultar.
fn integrity_rid(process: HANDLE) -> Option<u32> {
    // SAFETY: token aberto e fechado aqui; o buffer tem o tamanho informado e alinhamento de u64.
    unsafe {
        let mut token = HANDLE::default();
        OpenProcessToken(process, TOKEN_QUERY, &mut token).ok()?;
        let mut len = 0u32;
        let _ = GetTokenInformation(token, TokenIntegrityLevel, None, 0, &mut len);
        if len == 0 {
            let _ = CloseHandle(token);
            return None;
        }
        let mut buf = vec![0u64; (len as usize).div_ceil(8)];
        let ok = GetTokenInformation(
            token,
            TokenIntegrityLevel,
            Some(buf.as_mut_ptr().cast()),
            len,
            &mut len,
        )
        .is_ok();
        let _ = CloseHandle(token);
        if !ok {
            return None;
        }
        let label = &*(buf.as_ptr().cast::<TOKEN_MANDATORY_LABEL>());
        let sid = label.Label.Sid;
        let count = *GetSidSubAuthorityCount(sid);
        if count == 0 {
            return None;
        }
        Some(*GetSidSubAuthority(sid, u32::from(count) - 1))
    }
}

pub struct Win32WindowSystem {
    launcher: isize,
    step3: Step3Runner<Win32Attach>,
}

impl Win32WindowSystem {
    /// `launcher` e o `HWND` da janela `main` do Sofa.
    pub fn new(launcher: isize) -> Self {
        Self {
            launcher,
            step3: Step3Runner::new(Win32Attach { launcher }),
        }
    }

    fn hwnd(&self) -> HWND {
        to_hwnd(self.launcher)
    }
}

impl WindowSystem for Win32WindowSystem {
    fn sleep_ms(&self, ms: u64) {
        std::thread::sleep(Duration::from_millis(ms));
    }

    /// O desktop de entrada e `Default` (nao a tela de bloqueio, o UAC nem Ctrl+Alt+Del).
    fn desktop_is_interactive(&self) -> bool {
        // SAFETY: handle de desktop aberto e fechado aqui; buffer de nome local.
        unsafe {
            let Ok(desk) = OpenInputDesktop(DESKTOP_CONTROL_FLAGS(0), false, DESKTOP_READOBJECTS)
            else {
                return false; // sem acesso ao desktop de entrada: desktop seguro
            };
            let mut name = [0u16; 64];
            let mut needed = 0u32;
            let ok = GetUserObjectInformationW(
                HANDLE(desk.0),
                UOI_NAME,
                Some(name.as_mut_ptr().cast()),
                std::mem::size_of_val(&name) as u32,
                Some(&mut needed),
            )
            .is_ok();
            let _ = CloseDesktop(desk);
            if !ok {
                return false;
            }
            let len = name.iter().position(|c| *c == 0).unwrap_or(name.len());
            String::from_utf16_lossy(&name[..len]).eq_ignore_ascii_case("Default")
        }
    }

    fn launcher_is_foreground(&self) -> bool {
        // SAFETY: consulta sem efeito colateral.
        unsafe { GetForegroundWindow() == self.hwnd() }
    }

    fn foreground_is_elevated(&self) -> bool {
        // SAFETY: handles abertos e fechados aqui.
        unsafe {
            let fg = GetForegroundWindow();
            if fg.0.is_null() {
                return false;
            }
            let mut pid = 0u32;
            GetWindowThreadProcessId(fg, Some(&mut pid));
            let Some(mine) = integrity_rid(GetCurrentProcess()) else {
                return false;
            };
            let Ok(h) = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) else {
                // Nao abre nem com direitos minimos: processo de integridade maior ou protegido.
                return true;
            };
            let theirs = integrity_rid(h);
            let _ = CloseHandle(h);
            theirs.map_or(true, |t| t > mine)
        }
    }

    fn prepare_launcher_topmost(&self) {
        // SAFETY: janela do proprio Sofa; sem ativar (a ativacao e o objetivo da escada).
        unsafe {
            let _ = SetWindowPos(
                self.hwnd(),
                HWND_TOPMOST,
                0,
                0,
                0,
                0,
                SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE | SWP_SHOWWINDOW,
            );
        }
    }

    fn release_topmost(&self) {
        // SAFETY: janela do proprio Sofa.
        unsafe {
            let _ = SetWindowPos(
                self.hwnd(),
                HWND_NOTOPMOST,
                0,
                0,
                0,
                0,
                SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
            );
        }
    }

    fn step1_set_foreground(&self) {
        // SAFETY: janela do proprio Sofa.
        unsafe {
            let _ = SetForegroundWindow(self.hwnd());
        }
    }

    fn step3_attach(&self, timeout_ms: u64) -> Step3Result {
        self.step3.run(Duration::from_millis(timeout_ms))
    }

    /// Um unico par down/up (SEC-28), sem outra tecla, mouse ou texto.
    fn step2_key_and_foreground(&self, key: Step2Key) {
        let vk: VIRTUAL_KEY = match key {
            Step2Key::F24 => VK_F24,
            Step2Key::Alt => VK_MENU,
        };
        let make = |up: bool| INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: vk,
                    wScan: 0,
                    dwFlags: if up {
                        KEYEVENTF_KEYUP
                    } else {
                        KEYBD_EVENT_FLAGS(0)
                    },
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        };
        // SAFETY: dois `INPUT` validos na pilha; tamanho correto da estrutura.
        unsafe {
            let _ = SendInput(
                &[make(false), make(true)],
                std::mem::size_of::<INPUT>() as i32,
            );
            let _ = SetForegroundWindow(self.hwnd());
        }
    }

    fn step4_minimize_games(&self, pids: &[u32]) -> usize {
        let windows = top_level_windows(pids);
        // SAFETY: janelas de terceiros so recebem pedido assincrono de minimizar.
        unsafe {
            for w in &windows {
                let _ = ShowWindowAsync(to_hwnd(*w), SW_MINIMIZE);
            }
            let _ = SetForegroundWindow(self.hwnd());
        }
        windows.len()
    }

    fn front_game(&self, pids: &[u32]) -> bool {
        let windows = top_level_windows(pids);
        let Some(w) = windows.first() else {
            return false;
        };
        let h = to_hwnd(*w);
        // SAFETY: o Sofa esta na frente (tem o direito); so restaura e ativa a janela do jogo.
        unsafe {
            if IsIconic(h).as_bool() {
                let _ = ShowWindow(h, SW_RESTORE);
            }
            SetForegroundWindow(h).as_bool()
        }
    }

    fn game_is_foreground(&self, pids: &[u32]) -> bool {
        // SAFETY: consultas sem efeito colateral.
        unsafe {
            let fg = GetForegroundWindow();
            if fg.0.is_null() {
                return false;
            }
            let mut pid = 0u32;
            GetWindowThreadProcessId(fg, Some(&mut pid));
            pids.contains(&pid)
        }
    }

    fn allow_set_foreground_any(&self) {
        // SAFETY: sem ponteiros; permite que o jogo recem-iniciado tome o primeiro plano (CA-27).
        unsafe {
            let _ = AllowSetForegroundWindow(ASFW_ANY);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicU32;
    use std::sync::Mutex;

    #[derive(Default)]
    struct Fake {
        attaches: AtomicU32,
        detaches: AtomicU32,
        attach_ok: bool,
        /// `bring_launcher` demora (thread do jogo travada) ou da `panic`.
        bring_sleep_ms: u64,
        bring_panics: bool,
        log: Mutex<Vec<&'static str>>,
    }

    impl AttachOps for Arc<Fake> {
        fn foreground_thread(&self) -> u32 {
            7
        }
        fn current_thread(&self) -> u32 {
            9
        }
        fn attach(&self, _f: u32, _t: u32) -> bool {
            self.attaches.fetch_add(1, Ordering::SeqCst);
            self.log.lock().unwrap().push("attach");
            self.attach_ok
        }
        fn detach(&self, _f: u32, _t: u32) {
            self.detaches.fetch_add(1, Ordering::SeqCst);
            self.log.lock().unwrap().push("detach");
        }
        fn bring_launcher(&self) {
            self.log.lock().unwrap().push("bring");
            if self.bring_sleep_ms > 0 {
                std::thread::sleep(Duration::from_millis(self.bring_sleep_ms));
            }
            assert!(!self.bring_panics, "falha simulada no degrau 3");
        }
    }

    fn wait_until(mut cond: impl FnMut() -> bool) {
        for _ in 0..200 {
            if cond() {
                return;
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        panic!("condicao nao atendida a tempo");
    }

    #[test]
    fn sucesso_anexa_traz_e_desanexa() {
        let f = Arc::new(Fake {
            attach_ok: true,
            ..Fake::default()
        });
        let r = Step3Runner::new(f.clone());
        assert_eq!(r.run(Duration::from_millis(500)), Step3Result::Done);
        assert_eq!(*f.log.lock().unwrap(), ["attach", "bring", "detach"]);
    }

    #[test]
    fn anexo_recusado_nao_desanexa_o_que_nao_anexou_e_falha() {
        let f = Arc::new(Fake::default()); // attach_ok = false
        let r = Step3Runner::new(f.clone());
        assert_eq!(r.run(Duration::from_millis(500)), Step3Result::Failed);
        assert_eq!(f.detaches.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn desanexa_tambem_em_erro() {
        let f = Arc::new(Fake {
            attach_ok: true,
            bring_panics: true,
            ..Fake::default()
        });
        let r = Step3Runner::new(f.clone());
        assert_eq!(r.run(Duration::from_millis(500)), Step3Result::Failed);
        wait_until(|| f.detaches.load(Ordering::SeqCst) == 1);
        // A marca de pendente tambem foi liberada: a proxima tentativa roda.
        assert_ne!(r.run(Duration::from_millis(500)), Step3Result::Pending);
    }

    #[test]
    fn desanexa_tambem_no_tempo_limite_e_so_uma_thread_fica_pendente() {
        let f = Arc::new(Fake {
            attach_ok: true,
            bring_sleep_ms: 300,
            ..Fake::default()
        });
        let r = Step3Runner::new(f.clone());
        assert_eq!(r.run(Duration::from_millis(50)), Step3Result::TimedOut);
        // A thread anterior ainda nao voltou: nao cria outra, segue para o degrau 2.
        assert_eq!(r.run(Duration::from_millis(50)), Step3Result::Pending);
        assert_eq!(
            f.attaches.load(Ordering::SeqCst),
            1,
            "uma unica thread de degrau 3"
        );
        // Quando a thread volta, ela desanexa e a marca e liberada.
        wait_until(|| f.detaches.load(Ordering::SeqCst) == 1);
        wait_until(|| r.run(Duration::from_millis(500)) != Step3Result::Pending);
    }

    #[test]
    fn sem_janela_de_jogo_nada_e_minimizado() {
        let ws = Win32WindowSystem::new(0);
        assert_eq!(ws.step4_minimize_games(&[]), 0);
        assert!(!ws.front_game(&[]));
    }
}
