//! `SystemEvents` (BE-19, SEC-28): uma thread com janela oculta que escuta `WM_DISPLAYCHANGE` e
//! `WM_POWERBROADCAST` (suspender e acordar) e, no mesmo laco de mensagens, o evento de primeiro
//! plano por `SetWinEventHook(EVENT_SYSTEM_FOREGROUND, ..., WINEVENT_OUTOFCONTEXT |
//! WINEVENT_SKIPOWNPROCESS)`. **Unico** (junto com `foreground.rs`) a usar `SetWinEventHook`.
//!
//! O callback do hook so le o PID da janela e o caminho da imagem do processo: nao le titulo nem
//! conteudo, nao toca memoria de outro processo. Nunca `WINEVENT_INCONTEXT` (injecao de DLL) nem
//! `SetWindowsHookEx`.
//!
//! Observacao de desenho: a arquitetura citava `HWND_MESSAGE`, mas janelas so de mensagem **nao
//! recebem mensagens de broadcast** (documentacao do Win32), e `WM_DISPLAYCHANGE` e
//! `WM_POWERBROADCAST` chegam por broadcast. A janela aqui e de topo, sem estilo visivel, fora da
//! barra de tarefas e do Alt+Tab (`WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE`), nunca mostrada.

use std::cell::RefCell;
use std::sync::mpsc;
use std::sync::Arc;
use std::thread::JoinHandle;

use windows::core::w;
use windows::Win32::Foundation::{CloseHandle, HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::System::Threading::{
    GetCurrentThreadId, OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_WIN32,
    PROCESS_QUERY_LIMITED_INFORMATION,
};
use windows::Win32::UI::Accessibility::{SetWinEventHook, UnhookWinEvent, HWINEVENTHOOK};
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW, GetMessageW,
    GetWindowThreadProcessId, PostThreadMessageW, RegisterClassW, TranslateMessage,
    EVENT_SYSTEM_FOREGROUND, MSG, PBT_APMRESUMEAUTOMATIC, PBT_APMRESUMESUSPEND, PBT_APMSUSPEND,
    WINEVENT_OUTOFCONTEXT, WINEVENT_SKIPOWNPROCESS, WM_DISPLAYCHANGE, WM_POWERBROADCAST, WM_QUIT,
    WNDCLASSW, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_POPUP,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SystemEvent {
    /// Monitor ligado, desligado ou resolucao alterada (CA-66, CA-67).
    DisplayChanged,
    /// Voltou da suspensao (CA-46).
    Resumed,
    Suspending,
    /// Outra janela tomou o primeiro plano: PID e caminho da imagem (so leitura).
    ForegroundChanged {
        pid: u32,
        image: Option<String>,
    },
}

pub trait SystemEventSink: Send + Sync {
    fn event(&self, event: SystemEvent);
}

thread_local! {
    /// O callback do hook e o `WndProc` rodam na thread que instalou o hook / criou a janela.
    static SINK: RefCell<Option<Arc<dyn SystemEventSink>>> = const { RefCell::new(None) };
}

fn emit(e: SystemEvent) {
    SINK.with(|s| {
        if let Some(sink) = s.borrow().as_ref() {
            sink.event(e);
        }
    });
}

/// Traduz uma mensagem de janela no evento do Sofa. Separado do `WndProc` para o teste.
pub fn translate(msg: u32, wparam: usize) -> Option<SystemEvent> {
    match msg {
        WM_DISPLAYCHANGE => Some(SystemEvent::DisplayChanged),
        WM_POWERBROADCAST => match wparam as u32 {
            PBT_APMRESUMEAUTOMATIC | PBT_APMRESUMESUSPEND => Some(SystemEvent::Resumed),
            PBT_APMSUSPEND => Some(SystemEvent::Suspending),
            _ => None,
        },
        _ => None,
    }
}

unsafe extern "system" fn wndproc(h: HWND, msg: u32, w: WPARAM, l: LPARAM) -> LRESULT {
    if let Some(e) = translate(msg, w.0) {
        emit(e);
        return LRESULT(1); // TRUE: tratado
    }
    // SAFETY: repassa os mesmos argumentos ao tratamento padrao.
    unsafe { DefWindowProcW(h, msg, w, l) }
}

/// So le PID e imagem (SEC-28): nada de titulo, texto ou memoria.
unsafe extern "system" fn on_foreground(
    _hook: HWINEVENTHOOK,
    _event: u32,
    hwnd: HWND,
    _id_object: i32,
    _id_child: i32,
    _thread: u32,
    _time: u32,
) {
    let mut pid = 0u32;
    // SAFETY: `pid` e local valido.
    unsafe { GetWindowThreadProcessId(hwnd, Some(&mut pid)) };
    if pid == 0 {
        return;
    }
    let image = (|| {
        // SAFETY: handle aberto com direitos minimos e fechado aqui; buffer local.
        unsafe {
            let h = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid).ok()?;
            let mut buf = [0u16; 512];
            let mut n = buf.len() as u32;
            let ok = QueryFullProcessImageNameW(
                h,
                PROCESS_NAME_WIN32,
                windows::core::PWSTR(buf.as_mut_ptr()),
                &mut n,
            )
            .is_ok();
            let _ = CloseHandle(h);
            ok.then(|| String::from_utf16_lossy(&buf[..n as usize]))
        }
    })();
    emit(SystemEvent::ForegroundChanged { pid, image });
}

pub struct SystemEvents {
    thread_id: u32,
    hwnd: isize,
    handle: Option<JoinHandle<()>>,
}

impl SystemEvents {
    /// Sobe a thread. `foreground_hook = false` nao instala o hook de primeiro plano.
    pub fn spawn(
        sink: Arc<dyn SystemEventSink>,
        foreground_hook: bool,
    ) -> std::io::Result<SystemEvents> {
        let (tx, rx) = mpsc::channel::<Option<(u32, isize)>>();
        let handle = std::thread::Builder::new()
            .name("system-events".into())
            .spawn(move || run(sink, foreground_hook, &tx))?;
        match rx.recv() {
            Ok(Some((thread_id, hwnd))) => Ok(SystemEvents {
                thread_id,
                hwnd,
                handle: Some(handle),
            }),
            _ => {
                let _ = handle.join();
                Err(std::io::Error::other(
                    "janela de eventos do sistema nao criada",
                ))
            }
        }
    }

    /// `HWND` da janela oculta (para teste).
    pub fn hwnd(&self) -> isize {
        self.hwnd
    }
}

impl Drop for SystemEvents {
    fn drop(&mut self) {
        // SAFETY: posta `WM_QUIT` na propria thread de mensagens.
        unsafe {
            let _ = PostThreadMessageW(self.thread_id, WM_QUIT, WPARAM(0), LPARAM(0));
        }
        if let Some(h) = self.handle.take() {
            let _ = h.join();
        }
    }
}

fn run(
    sink: Arc<dyn SystemEventSink>,
    foreground_hook: bool,
    ready: &mpsc::Sender<Option<(u32, isize)>>,
) {
    SINK.with(|s| *s.borrow_mut() = Some(sink));
    // SAFETY: tudo nesta funcao acontece na thread dona da janela e do hook.
    unsafe {
        let Ok(hinst) = GetModuleHandleW(None) else {
            let _ = ready.send(None);
            return;
        };
        let class = w!("SofaSystemEvents");
        let wc = WNDCLASSW {
            lpfnWndProc: Some(wndproc),
            hInstance: hinst.into(),
            lpszClassName: class,
            ..Default::default()
        };
        RegisterClassW(&wc); // se ja registrada, o `CreateWindowExW` abaixo ainda funciona
        let Ok(hwnd) = CreateWindowExW(
            WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE,
            class,
            w!("Sofa system events"),
            WS_POPUP,
            0,
            0,
            0,
            0,
            None,
            None,
            hinst,
            None,
        ) else {
            let _ = ready.send(None);
            return;
        };
        let hook = foreground_hook.then(|| {
            SetWinEventHook(
                EVENT_SYSTEM_FOREGROUND,
                EVENT_SYSTEM_FOREGROUND,
                None,
                Some(on_foreground),
                0,
                0,
                WINEVENT_OUTOFCONTEXT | WINEVENT_SKIPOWNPROCESS,
            )
        });
        let _ = ready.send(Some((GetCurrentThreadId(), hwnd.0 as isize)));

        let mut msg = MSG::default();
        while GetMessageW(&mut msg, None, 0, 0).as_bool() {
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
        if let Some(h) = hook {
            let _ = UnhookWinEvent(h);
        }
        let _ = DestroyWindow(hwnd);
    }
    SINK.with(|s| *s.borrow_mut() = None);
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;
    use std::time::Duration;
    use windows::Win32::UI::WindowsAndMessaging::SendMessageW;

    #[test]
    fn traducao_das_mensagens() {
        assert_eq!(
            translate(WM_DISPLAYCHANGE, 0),
            Some(SystemEvent::DisplayChanged)
        );
        assert_eq!(
            translate(WM_POWERBROADCAST, PBT_APMRESUMEAUTOMATIC as usize),
            Some(SystemEvent::Resumed)
        );
        assert_eq!(
            translate(WM_POWERBROADCAST, PBT_APMRESUMESUSPEND as usize),
            Some(SystemEvent::Resumed)
        );
        assert_eq!(
            translate(WM_POWERBROADCAST, PBT_APMSUSPEND as usize),
            Some(SystemEvent::Suspending)
        );
        assert_eq!(translate(WM_POWERBROADCAST, 0x9999), None);
        assert_eq!(translate(0x0001, 0), None);
    }

    #[derive(Default)]
    struct Rec(Mutex<Vec<SystemEvent>>);
    impl SystemEventSink for Rec {
        fn event(&self, e: SystemEvent) {
            self.0.lock().unwrap().push(e);
        }
    }

    #[test]
    fn a_janela_recebe_as_mensagens_e_a_thread_encerra_limpa() {
        let rec = Arc::new(Rec::default());
        let se = SystemEvents::spawn(rec.clone(), true).expect("thread de eventos");
        let hwnd = HWND(se.hwnd() as *mut _);
        // SAFETY: a janela e nossa; `SendMessageW` entre threads espera o `WndProc`.
        unsafe {
            SendMessageW(hwnd, WM_DISPLAYCHANGE, WPARAM(32), LPARAM(0));
            SendMessageW(
                hwnd,
                WM_POWERBROADCAST,
                WPARAM(PBT_APMSUSPEND as usize),
                LPARAM(0),
            );
            SendMessageW(
                hwnd,
                WM_POWERBROADCAST,
                WPARAM(PBT_APMRESUMEAUTOMATIC as usize),
                LPARAM(0),
            );
        }
        std::thread::sleep(Duration::from_millis(50));
        let got = rec.0.lock().unwrap().clone();
        let relevant: Vec<_> = got
            .into_iter()
            .filter(|e| !matches!(e, SystemEvent::ForegroundChanged { .. }))
            .collect();
        assert_eq!(
            relevant,
            vec![
                SystemEvent::DisplayChanged,
                SystemEvent::Suspending,
                SystemEvent::Resumed
            ]
        );
        drop(se); // `join` nao trava
    }
}
