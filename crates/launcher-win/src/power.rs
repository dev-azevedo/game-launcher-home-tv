//! `PowerControl` e `RandomSource` no Win32 (BE-33, SEC-27, RN-15, CA-47, CA-49, AF-04, INV-05).
//!
//! - Desligar e reiniciar: `ExitWindowsEx` **sem** nenhuma forma de forcar o fim de apps: o Windows
//!   mostra a tela de "apps impedindo o desligamento". "Desligar" faz o mesmo que o menu Iniciar
//!   (D-44): com a inicializacao rapida ligada (`HiberbootEnabled = 1` e hibernacao disponivel, lidos
//!   na hora) usa `EWX_HYBRID_SHUTDOWN | EWX_SHUTDOWN | EWX_POWEROFF`; se isso falhar, tenta uma vez
//!   so `EWX_POWEROFF`. "Reiniciar" e sempre completo (`EWX_REBOOT`).
//! - Suspender: `SetSuspendState(FALSE, FALSE, FALSE)`.
//! - `SeShutdownPrivilege` e habilitado no token do proprio processo **so durante a chamada** e
//!   devolvido ao estado anterior em seguida (guarda RAII). Nao ha elevacao: o privilegio ja existe
//!   no token do usuario interativo, so vem desabilitado.
//! - Nada de `shutdown.exe` nem de shell.
//! - `GetPwrCapabilities` + `IsPwrSuspendAllowed` dizem se a suspensao existe e esta permitida.
//!
//! Os testes **nunca** chamam `execute` (desligaria o PC): conferem so as flags (`exit_flags`) e a
//! leitura de capacidades.

use launcher_core::application::ports::{PowerControl, RandomSource};
use launcher_core::application::{AppError, PowerCapabilities};
use launcher_core::domain::power::PowerAction;
use windows::core::PCWSTR;
use windows::Win32::Foundation::{
    CloseHandle, GetLastError, SetLastError, ERROR_NOT_ALL_ASSIGNED, ERROR_SUCCESS, HANDLE,
};
use windows::Win32::Security::Cryptography::{BCryptGenRandom, BCRYPT_USE_SYSTEM_PREFERRED_RNG};
use windows::Win32::Security::{
    AdjustTokenPrivileges, LookupPrivilegeValueW, LUID_AND_ATTRIBUTES, SE_PRIVILEGE_ENABLED,
    SE_SHUTDOWN_NAME, TOKEN_ADJUST_PRIVILEGES, TOKEN_PRIVILEGES, TOKEN_QUERY,
};
use windows::Win32::System::Power::SYSTEM_POWER_CAPABILITIES;
use windows::Win32::System::Power::{GetPwrCapabilities, IsPwrSuspendAllowed, SetSuspendState};
use windows::Win32::System::Registry::{RegGetValueW, HKEY_LOCAL_MACHINE, RRF_RT_REG_DWORD};
use windows::Win32::System::Shutdown::{
    ExitWindowsEx, EWX_HYBRID_SHUTDOWN, EWX_POWEROFF, EWX_REBOOT, EWX_SHUTDOWN, EXIT_WINDOWS_FLAGS,
    SHTDN_REASON_FLAG_PLANNED, SHTDN_REASON_MAJOR_OTHER, SHTDN_REASON_MINOR_OTHER, SHUTDOWN_REASON,
};
use windows::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};

/// Chave onde o Windows guarda `HiberbootEnabled` (inicializacao rapida).
const POWER_KEY: &str = r"SYSTEM\CurrentControlSet\Control\Session Manager\Power";

/// A inicializacao rapida esta valendo? So com `HiberbootEnabled = 1` **e** hibernacao disponivel
/// (`SystemS4` e arquivo de hibernacao presente): e a condicao em que o menu Iniciar desliga de
/// modo hibrido (D-44). Pura, para o teste cobrir a tabela.
pub fn fast_startup_active(hiberboot: Option<u32>, s4: bool, hiber_file: bool) -> bool {
    hiberboot == Some(1) && s4 && hiber_file
}

/// Le o estado da inicializacao rapida agora (nao fica em cache: o usuario pode mudar a opcao
/// com o Sofa aberto).
fn fast_startup_now() -> bool {
    let key: Vec<u16> = POWER_KEY.encode_utf16().chain(Some(0)).collect();
    let name: Vec<u16> = "HiberbootEnabled".encode_utf16().chain(Some(0)).collect();
    let mut value = 0u32;
    let mut size = std::mem::size_of::<u32>() as u32;
    // SAFETY: buffers validos e terminados em NUL; leitura de um DWORD em HKLM (so leitura).
    let read_ok = unsafe {
        RegGetValueW(
            HKEY_LOCAL_MACHINE,
            PCWSTR(key.as_ptr()),
            PCWSTR(name.as_ptr()),
            RRF_RT_REG_DWORD,
            None,
            Some(std::ptr::from_mut(&mut value).cast()),
            Some(&mut size),
        )
        .is_ok()
    };
    let hiberboot = read_ok.then_some(value);
    let mut caps = SYSTEM_POWER_CAPABILITIES::default();
    // SAFETY: `caps` e local, com o tamanho esperado pela API.
    let got = unsafe { GetPwrCapabilities(&mut caps).as_bool() };
    got && fast_startup_active(
        hiberboot,
        caps.SystemS4.as_bool(),
        caps.HiberFilePresent.as_bool(),
    )
}

/// Flags do `ExitWindowsEx` de cada acao; `None` para a suspensao (outro caminho). `hybrid` so
/// vale para "Desligar" (D-44); "Reiniciar" e sempre completo. Nunca ha bit de forca (INV-05).
pub fn exit_flags(
    action: PowerAction,
    hybrid: bool,
) -> Option<(EXIT_WINDOWS_FLAGS, SHUTDOWN_REASON)> {
    let reason = SHTDN_REASON_MAJOR_OTHER | SHTDN_REASON_MINOR_OTHER | SHTDN_REASON_FLAG_PLANNED;
    match action {
        // A documentacao do ExitWindowsEx combina o bit hibrido com EWX_SHUTDOWN.
        PowerAction::Shutdown if hybrid => {
            Some((EWX_HYBRID_SHUTDOWN | EWX_SHUTDOWN | EWX_POWEROFF, reason))
        }
        PowerAction::Shutdown => Some((EWX_POWEROFF, reason)),
        PowerAction::Restart => Some((EWX_REBOOT, reason)),
        PowerAction::Suspend => None,
    }
}

/// `SeShutdownPrivilege` habilitado enquanto o guarda viver; ao soltar, desabilita de novo.
struct ShutdownPrivilege {
    token: HANDLE,
    luid: windows::Win32::Foundation::LUID,
}

impl ShutdownPrivilege {
    fn enable() -> Result<Self, AppError> {
        // SAFETY: handle de token aberto aqui e fechado no `Drop` (ou logo abaixo, em erro);
        // ponteiros apontam para locais validos durante a chamada.
        unsafe {
            let mut token = HANDLE::default();
            OpenProcessToken(
                GetCurrentProcess(),
                TOKEN_ADJUST_PRIVILEGES | TOKEN_QUERY,
                &mut token,
            )
            .map_err(|e| {
                tracing::warn!(%e, "energia: nao abriu o token do processo");
                AppError::Rejected
            })?;
            let mut luid = windows::Win32::Foundation::LUID::default();
            if let Err(e) = LookupPrivilegeValueW(None, SE_SHUTDOWN_NAME, &mut luid) {
                tracing::warn!(%e, "energia: SeShutdownPrivilege desconhecido");
                let _ = CloseHandle(token);
                return Err(AppError::Rejected);
            }
            let guard = Self { token, luid };
            guard.set(true)?;
            Ok(guard)
        }
    }

    fn set(&self, enabled: bool) -> Result<(), AppError> {
        let privileges = TOKEN_PRIVILEGES {
            PrivilegeCount: 1,
            Privileges: [LUID_AND_ATTRIBUTES {
                Luid: self.luid,
                Attributes: if enabled {
                    SE_PRIVILEGE_ENABLED
                } else {
                    windows::Win32::Security::TOKEN_PRIVILEGES_ATTRIBUTES(0)
                },
            }],
        };
        // SAFETY: `privileges` vive durante a chamada; o token e nosso. `AdjustTokenPrivileges`
        // devolve sucesso mesmo quando o privilegio nao foi concedido: o erro esta no
        // `GetLastError`.
        unsafe {
            SetLastError(ERROR_SUCCESS);
            AdjustTokenPrivileges(self.token, false, Some(&privileges), 0, None, None).map_err(
                |e| {
                    tracing::warn!(%e, "energia: AdjustTokenPrivileges falhou");
                    AppError::Rejected
                },
            )?;
            if GetLastError() == ERROR_NOT_ALL_ASSIGNED {
                tracing::warn!("energia: o token nao tem SeShutdownPrivilege");
                return Err(AppError::Rejected);
            }
        }
        Ok(())
    }
}

impl Drop for ShutdownPrivilege {
    fn drop(&mut self) {
        let _ = self.set(false);
        // SAFETY: o handle foi aberto em `enable` e so e fechado aqui.
        unsafe {
            let _ = CloseHandle(self.token);
        }
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub struct WinPowerControl;

impl PowerControl for WinPowerControl {
    fn capabilities(&self) -> PowerCapabilities {
        // Desligar e reiniciar: `ExitWindowsEx` existe em todo Windows alvo. Suspender depende do
        // hardware (S1 a S3) e de nao estar desligada pelo Windows (CA-47).
        let mut caps = SYSTEM_POWER_CAPABILITIES::default();
        // SAFETY: `caps` e local, com o tamanho esperado pela API.
        let (got, allowed) = unsafe {
            (
                GetPwrCapabilities(&mut caps).as_bool(),
                IsPwrSuspendAllowed().as_bool(),
            )
        };
        let sleeps =
            got && (caps.SystemS1.as_bool() || caps.SystemS2.as_bool() || caps.SystemS3.as_bool());
        PowerCapabilities {
            shutdown: true,
            restart: true,
            suspend: sleeps && allowed,
        }
    }

    fn execute(&self, action: PowerAction) -> Result<(), AppError> {
        let hybrid = action == PowerAction::Shutdown && fast_startup_now();
        match exit_flags(action, hybrid) {
            Some((flags, reason)) => {
                let _privilege = ShutdownPrivilege::enable()?;
                // SAFETY: chamada Win32 sem ponteiros; o privilegio vale ate o fim do escopo.
                let first = unsafe { ExitWindowsEx(flags, reason) };
                match (first, hybrid) {
                    (Ok(()), _) => Ok(()),
                    (Err(e), true) => {
                        // Uma unica nova tentativa, sem o bit hibrido (desligamento completo).
                        tracing::warn!(%e, "energia: ExitWindowsEx recusou o desligamento hibrido; tentando o completo");
                        let (flags, reason) =
                            exit_flags(action, false).ok_or(AppError::Rejected)?;
                        // SAFETY: idem acima.
                        unsafe { ExitWindowsEx(flags, reason) }.map_err(|e| {
                            tracing::warn!(%e, "energia: ExitWindowsEx recusou");
                            AppError::Rejected
                        })
                    }
                    (Err(e), false) => {
                        tracing::warn!(%e, "energia: ExitWindowsEx recusou");
                        Err(AppError::Rejected)
                    }
                }
            }
            None => {
                // SAFETY: chamada Win32 sem ponteiros. Sem hibernar, sem forcar, com eventos de
                // despertar habilitados.
                let ok = unsafe { SetSuspendState(false, false, false) };
                if ok.as_bool() {
                    Ok(())
                } else {
                    tracing::warn!("energia: SetSuspendState recusou");
                    Err(AppError::Rejected)
                }
            }
        }
    }
}

/// CSPRNG do sistema (`BCryptGenRandom` com o RNG preferido).
#[derive(Debug, Default, Clone, Copy)]
pub struct SystemRandom;

impl RandomSource for SystemRandom {
    fn fill(&self, buf: &mut [u8]) -> Result<(), AppError> {
        // SAFETY: o buffer e uma fatia valida; sem handle de algoritmo (RNG preferido do sistema).
        let status = unsafe { BCryptGenRandom(None, buf, BCRYPT_USE_SYSTEM_PREFERRED_RNG) };
        if status.is_ok() {
            Ok(())
        } else {
            Err(AppError::Internal)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// So confere as flags: nenhuma chamada a `ExitWindowsEx` (desligaria o PC).
    #[test]
    fn flags_de_desligar_e_reiniciar_nunca_forcam() {
        let (off, reason) = exit_flags(PowerAction::Shutdown, false).unwrap();
        let (hybrid, hybrid_reason) = exit_flags(PowerAction::Shutdown, true).unwrap();
        let (reboot, _) = exit_flags(PowerAction::Restart, false).unwrap();
        let (reboot_hybrid_requested, _) = exit_flags(PowerAction::Restart, true).unwrap();
        // Completo: EWX_POWEROFF (0x8) e EWX_REBOOT (0x2), nada alem disso.
        assert_eq!(off.0, 0x8);
        assert_eq!(reboot.0, 0x2);
        // Hibrido (D-44): EWX_HYBRID_SHUTDOWN (0x400000) | EWX_SHUTDOWN (0x1) | EWX_POWEROFF (0x8).
        assert_eq!(hybrid.0, 0x0040_0000 | 0x1 | 0x8);
        // Reiniciar e sempre completo, mesmo pedindo o hibrido.
        assert_eq!(reboot_hybrid_requested.0, 0x2);
        // Sem os bits de forcar (EWX_FORCE 0x4 e EWX_FORCEIFHUNG 0x10) em nenhuma combinacao.
        for f in [off, hybrid, reboot, reboot_hybrid_requested] {
            assert_eq!(f.0 & 0x14, 0, "{:#x}", f.0);
        }
        // Desligamento planejado.
        assert_ne!(reason.0 & 0x8000_0000, 0);
        assert_ne!(hybrid_reason.0 & 0x8000_0000, 0);
        assert!(exit_flags(PowerAction::Suspend, false).is_none());
        assert!(exit_flags(PowerAction::Suspend, true).is_none());
    }

    #[test]
    fn inicializacao_rapida_so_com_hiberboot_ligado_e_hibernacao_disponivel() {
        let tabela = [
            (Some(1), true, true, true),
            (Some(1), false, true, false), // sem S4
            (Some(1), true, false, false), // sem arquivo de hibernacao
            (Some(0), true, true, false),  // opcao desligada
            (Some(2), true, true, false),  // valor fora do esperado
            (None, true, true, false),     // sem o valor
        ];
        for (hiberboot, s4, file, esperado) in tabela {
            assert_eq!(
                fast_startup_active(hiberboot, s4, file),
                esperado,
                "{hiberboot:?} {s4} {file}"
            );
        }
    }

    /// So le o registro e as capacidades: nao liga nem desliga nada.
    #[test]
    fn leitura_da_inicializacao_rapida_responde_sem_efeito() {
        eprintln!(
            "inicializacao rapida ativa neste PC: {}",
            fast_startup_now()
        );
    }

    /// So le: nao liga nem desliga nada.
    #[test]
    fn capacidades_do_pc_respondem_sem_efeito() {
        let caps = WinPowerControl.capabilities();
        assert!(caps.shutdown && caps.restart);
        eprintln!("suspend disponivel neste PC: {}", caps.suspend);
    }

    #[test]
    fn csprng_preenche_e_nao_repete() {
        let mut a = [0u8; 32];
        let mut b = [0u8; 32];
        SystemRandom.fill(&mut a).unwrap();
        SystemRandom.fill(&mut b).unwrap();
        assert_ne!(a, [0u8; 32]);
        assert_ne!(a, b);
    }
}
