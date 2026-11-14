//! Higiene do processo (SEC-20, SEC-23): pesquisa de DLL travada em System32 e na pasta do
//! aplicativo, e deteccao de execucao elevada. Nenhuma RN-xx aqui.

use windows::Win32::Foundation::{CloseHandle, HANDLE};
use windows::Win32::Security::{GetTokenInformation, TokenElevation, TOKEN_ELEVATION, TOKEN_QUERY};
use windows::Win32::System::LibraryLoader::{
    SetDefaultDllDirectories, LOAD_LIBRARY_SEARCH_APPLICATION_DIR, LOAD_LIBRARY_SEARCH_SYSTEM32,
};
use windows::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};

/// `SetDefaultDllDirectories(SYSTEM32 | APPLICATION_DIR)` (SEC-23). Deve ser a primeira chamada
/// do `main`, antes de qualquer carga de DLL por demanda. Devolve `false` se o Windows recusar;
/// o chamador segue (a protecao e uma camada extra) e registra no log quando ele existir.
pub fn set_default_dll_directories() -> bool {
    // SAFETY: chamada Win32 sem ponteiros; so altera a politica de busca de DLL do processo.
    unsafe {
        SetDefaultDllDirectories(LOAD_LIBRARY_SEARCH_SYSTEM32 | LOAD_LIBRARY_SEARCH_APPLICATION_DIR)
            .is_ok()
    }
}

/// `true` quando o token do processo esta elevado (`TokenElevation`). O Sofa nunca deve estar
/// (SEC-20, RN-27); se estiver, o chamador registra e sinaliza. Falha ao consultar = `false`.
pub fn is_process_elevated() -> bool {
    // SAFETY: handle de token aberto aqui, fechado em todos os caminhos; buffer `TOKEN_ELEVATION`
    // vive na pilha com o tamanho informado.
    unsafe {
        let mut token = HANDLE::default();
        if OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token).is_err() {
            return false;
        }
        let mut elevation = TOKEN_ELEVATION::default();
        let mut returned = 0u32;
        let ok = GetTokenInformation(
            token,
            TokenElevation,
            Some(std::ptr::from_mut(&mut elevation).cast()),
            std::mem::size_of::<TOKEN_ELEVATION>() as u32,
            &mut returned,
        )
        .is_ok();
        let _ = CloseHandle(token);
        ok && elevation.TokenIsElevated != 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dll_directories_aceita_a_politica_no_processo_de_teste() {
        assert!(set_default_dll_directories());
        // Idempotente.
        assert!(set_default_dll_directories());
    }

    #[test]
    fn processo_de_teste_nao_esta_elevado() {
        // Os testes rodam como usuario comum (a CI local nao eleva). Se o desenvolvedor rodar
        // elevado de proposito, o teste avisa em vez de passar em silencio.
        assert!(
            !is_process_elevated(),
            "rodando elevado: o Sofa nunca deve ser elevado (SEC-20)"
        );
    }
}
