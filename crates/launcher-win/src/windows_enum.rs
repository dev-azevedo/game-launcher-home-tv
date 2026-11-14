//! Janelas de topo de um conjunto de processos (jogo da sessao). Usado pelo primeiro plano
//! (minimizar, restaurar) e pela sonda (`WM_CLOSE`). So leitura da lista de janelas: nao le
//! titulo nem conteudo de janela de terceiros (SEC-18 b).

use windows::Win32::Foundation::{BOOL, HWND, LPARAM, TRUE};
use windows::Win32::UI::WindowsAndMessaging::{
    EnumWindows, GetWindow, GetWindowThreadProcessId, IsWindowVisible, GW_OWNER,
};

struct Query<'a> {
    pids: &'a [u32],
    found: Vec<isize>,
}

unsafe extern "system" fn collect(hwnd: HWND, lparam: LPARAM) -> BOOL {
    // SAFETY: `lparam` e o ponteiro para o `Query` vivo em `top_level_windows`.
    let q = unsafe { &mut *(lparam.0 as *mut Query<'_>) };
    let mut pid = 0u32;
    // SAFETY: `pid` e local valido.
    unsafe { GetWindowThreadProcessId(hwnd, Some(&mut pid)) };
    if q.pids.contains(&pid) {
        // Janela de topo "de verdade": visivel e sem dono (as de dialogo seguem a principal).
        // SAFETY: consultas sem efeito colateral.
        let top = unsafe {
            IsWindowVisible(hwnd).as_bool()
                && GetWindow(hwnd, GW_OWNER).map_or(true, |o| o.0.is_null())
        };
        if top {
            q.found.push(hwnd.0 as isize);
        }
    }
    TRUE
}

/// Janelas de topo visiveis e sem dono dos `pids`, em ordem de Z (a de cima primeiro).
pub fn top_level_windows(pids: &[u32]) -> Vec<isize> {
    if pids.is_empty() {
        return Vec::new();
    }
    let mut q = Query {
        pids,
        found: Vec::new(),
    };
    // SAFETY: `q` vive durante a chamada sincrona; o callback so a usa ali.
    unsafe {
        let _ = EnumWindows(Some(collect), LPARAM(std::ptr::from_mut(&mut q) as isize));
    }
    q.found
}

/// Handle de janela a partir do inteiro guardado.
pub fn hwnd(v: isize) -> HWND {
    HWND(v as *mut _)
}
