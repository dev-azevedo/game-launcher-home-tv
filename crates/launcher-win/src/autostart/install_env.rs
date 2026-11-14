//! Fatos do Windows sobre a instalacao (BE-36, DR-04, SEC-21): o `sofa.exe` em execucao, as pastas
//! `Program Files` (por `SHGetKnownFolderPath`, nao por variavel de ambiente, que o usuario
//! controla) e o `InstallDefaults` gravado pelo instalador em `HKLM` (so leitura).

use launcher_core::application::ports::InstallEnv;
use windows::core::PCWSTR;
use windows::Win32::Foundation::ERROR_SUCCESS;
use windows::Win32::System::Registry::{
    RegGetValueW, HKEY_LOCAL_MACHINE, RRF_RT_ANY, RRF_RT_REG_DWORD, RRF_SUBKEY_WOW6432KEY,
    RRF_SUBKEY_WOW6464KEY,
};

use crate::path_canon::{canonicalize, known, strip_verbatim};

const INSTALL_KEY: &str = r"Software\app.sofa.launcher";
const INSTALL_VALUE: &str = "InstallDefaults";

#[derive(Debug, Default, Clone, Copy)]
pub struct WinInstallEnv;

fn w(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(Some(0)).collect()
}

/// DWORD de `HKLM\Software\app.sofa.launcher\InstallDefaults`. Valor de outro tipo = `None`
/// (SEC-21: so DWORD 0/1 vale). Le a visao de 64 bits e, na falta dela, a de 32.
pub fn read_install_default() -> Option<u32> {
    let key = w(INSTALL_KEY);
    let name = w(INSTALL_VALUE);
    for view in [RRF_SUBKEY_WOW6464KEY, RRF_SUBKEY_WOW6432KEY] {
        let mut value = 0u32;
        let mut len = std::mem::size_of::<u32>() as u32;
        // SAFETY: nomes terminados em nulo vivos durante a chamada; `value` tem o tamanho de `len`.
        let r = unsafe {
            RegGetValueW(
                HKEY_LOCAL_MACHINE,
                PCWSTR(key.as_ptr()),
                PCWSTR(name.as_ptr()),
                RRF_RT_REG_DWORD | view,
                None,
                Some(std::ptr::from_mut(&mut value).cast()),
                Some(&mut len),
            )
        };
        if r == ERROR_SUCCESS {
            return Some(value);
        }
    }
    None
}

/// Existe o valor, mesmo de outro tipo? (para o log: "InstallDefaults com tipo inesperado").
pub fn install_default_present_any_type() -> bool {
    let key = w(INSTALL_KEY);
    let name = w(INSTALL_VALUE);
    // SAFETY: como em `read_install_default`, sem buffer de saida.
    unsafe {
        RegGetValueW(
            HKEY_LOCAL_MACHINE,
            PCWSTR(key.as_ptr()),
            PCWSTR(name.as_ptr()),
            RRF_RT_ANY | RRF_SUBKEY_WOW6464KEY,
            None,
            None,
            None,
        ) == ERROR_SUCCESS
    }
}

impl InstallEnv for WinInstallEnv {
    fn current_exe(&self) -> Option<String> {
        let exe = std::env::current_exe().ok()?;
        let canonical = canonicalize(exe.to_str()?).ok()?;
        Some(strip_verbatim(&canonical))
    }

    fn program_files(&self) -> Vec<String> {
        known::program_files()
            .and_then(|p| canonicalize(&p).ok())
            .map(|p| strip_verbatim(&p))
            .into_iter()
            .collect()
    }

    fn install_default(&self) -> Option<u32> {
        let v = read_install_default();
        if v.is_none() && install_default_present_any_type() {
            tracing::warn!(
                "InstallDefaults existe com tipo diferente de DWORD; tratado como desligado"
            );
        }
        v
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exe_em_execucao_e_canonico_e_program_files_existe() {
        let env = WinInstallEnv;
        let exe = env.current_exe().expect("exe em execucao");
        assert!(exe.chars().nth(1) == Some(':'), "{exe}");
        assert!(!exe.starts_with(r"\\?\"), "{exe}");
        let pf = env.program_files();
        assert_eq!(pf.len(), 1);
        assert!(pf[0].to_lowercase().contains("program files"), "{pf:?}");
    }

    /// O binario de teste roda de `target\debug`, nunca de `Program Files` (DR-04).
    #[test]
    fn binario_de_teste_nao_esta_em_program_files() {
        let env = WinInstallEnv;
        let exe = env.current_exe().unwrap();
        assert!(!launcher_core::domain::autostart::is_under_program_files(
            &exe,
            &env.program_files()
        ));
    }

    #[test]
    fn leitura_de_install_defaults_nao_falha_sem_a_chave() {
        // Sem instalacao nesta maquina a chave nao existe; com ela, vale o DWORD. So nao pode
        // entrar em panico.
        let _ = read_install_default();
    }
}
