//! "Jogo" sintetico dos testes de integracao (BE-16, BE-17, BE-18, BE-21). Nao faz parte do
//! produto: o bundler empacota so o `sofa.exe`.
//!
//!   sofa-test-game sleep <ms>                         dorme e sai
//!   sofa-test-game spawn <exe> <child_ms> <parent_ms> inicia um filho (`sleep`) e dorme
//!   sofa-test-game dump <arquivo> [args...]           grava cada argumento em uma linha e sai
//!   sofa-test-game window <ms> [ignore-close]         janela de topo; `WM_CLOSE` fecha, a menos
//!                                                     que `ignore-close`; sai sozinho em <ms>

use std::time::{Duration, Instant};

use windows::core::w;
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::Graphics::Gdi::{GetStockObject, BLACK_BRUSH, HBRUSH};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW, LoadCursorW, PeekMessageW,
    PostQuitMessage, RegisterClassW, TranslateMessage, CW_USEDEFAULT, IDC_ARROW, MSG, PM_REMOVE,
    WM_CLOSE, WM_DESTROY, WM_QUIT, WNDCLASSW, WS_EX_APPWINDOW, WS_OVERLAPPEDWINDOW, WS_VISIBLE,
};

static mut IGNORE_CLOSE: bool = false;

unsafe extern "system" fn wndproc(h: HWND, m: u32, w: WPARAM, l: LPARAM) -> LRESULT {
    match m {
        WM_CLOSE => {
            // SAFETY: lido so na thread da janela; escrito antes de criar a janela.
            if unsafe { *std::ptr::addr_of!(IGNORE_CLOSE) } {
                return LRESULT(0); // jogo que ignora o pedido de fechamento
            }
            // SAFETY: janela propria.
            unsafe {
                let _ = DestroyWindow(h);
            }
            LRESULT(0)
        }
        WM_DESTROY => {
            // SAFETY: sem argumentos.
            unsafe { PostQuitMessage(0) };
            LRESULT(0)
        }
        // SAFETY: repassa ao tratamento padrao.
        _ => unsafe { DefWindowProcW(h, m, w, l) },
    }
}

fn window(ms: u64, ignore_close: bool) {
    // SAFETY: tudo na thread principal, que e a dona da janela.
    unsafe {
        *std::ptr::addr_of_mut!(IGNORE_CLOSE) = ignore_close;
        let hinst = GetModuleHandleW(None).expect("modulo");
        let class = w!("SofaTestGame");
        let wc = WNDCLASSW {
            lpfnWndProc: Some(wndproc),
            hInstance: hinst.into(),
            lpszClassName: class,
            hCursor: LoadCursorW(None, IDC_ARROW).expect("cursor"),
            hbrBackground: HBRUSH(GetStockObject(BLACK_BRUSH).0),
            ..Default::default()
        };
        RegisterClassW(&wc);
        let _hwnd = CreateWindowExW(
            WS_EX_APPWINDOW,
            class,
            w!("Sofa test game"),
            WS_OVERLAPPEDWINDOW | WS_VISIBLE,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            640,
            360,
            None,
            None,
            hinst,
            None,
        )
        .expect("janela");
        let start = Instant::now();
        let mut msg = MSG::default();
        while start.elapsed() < Duration::from_millis(ms) {
            while PeekMessageW(&mut msg, None, 0, 0, PM_REMOVE).as_bool() {
                if msg.message == WM_QUIT {
                    return;
                }
                let _ = TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
            std::thread::sleep(Duration::from_millis(5));
        }
    }
}

fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    let num = |i: usize| a.get(i).and_then(|s| s.parse::<u64>().ok()).unwrap_or(0);
    match a.first().map(String::as_str) {
        Some("sleep") => std::thread::sleep(Duration::from_millis(num(1))),
        Some("spawn") => {
            let _ = std::process::Command::new(&a[1])
                .args(["sleep", &a[2]])
                .spawn();
            std::thread::sleep(Duration::from_millis(num(3)));
        }
        Some("dump") => {
            let body = a[2..].join("\n");
            let _ = std::fs::write(&a[1], body);
        }
        Some("window") => window(num(1), a.get(2).map(String::as_str) == Some("ignore-close")),
        _ => {}
    }
}
