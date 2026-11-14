//! `PowerControl` e `RandomSource` no Win32 (BE-33, SEC-27, RN-15, CA-47, CA-49, AF-04, INV-05).
//!
//! - Desligar e reiniciar: `ExitWindowsEx(EWX_POWEROFF | EWX_REBOOT, FLAG_PLANNED)` **sem** nenhuma
//!   forma de forcar o fim de apps: o Windows mostra a tela de "apps impedindo o desligamento".
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
use windows::Win32::System::Shutdown::{
    ExitWindowsEx, EWX_POWEROFF, EWX_REBOOT, EXIT_WINDOWS_FLAGS, SHTDN_REASON_FLAG_PLANNED,
    SHTDN_REASON_MAJOR_OTHER, SHTDN_REASON_MINOR_OTHER, SHUTDOWN_REASON,
};
use windows::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};

/// Flags do `ExitWindowsEx` de cada acao; `None` para a suspensao (outro caminho). Separado para
/// o teste conferir que nunca ha bit de forca.
pub fn exit_flags(action: PowerAction) -> Option<(EXIT_WINDOWS_FLAGS, SHUTDOWN_REASON)> {
    let reason = SHTDN_REASON_MAJOR_OTHER | SHTDN_REASON_MINOR_OTHER | SHTDN_REASON_FLAG_PLANNED;
    match action {
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
        match exit_flags(action) {
            Some((flags, reason)) => {
                let _privilege = ShutdownPrivilege::enable()?;
                // SAFETY: chamada Win32 sem ponteiros; o privilegio vale ate o fim do escopo.
                unsafe { ExitWindowsEx(flags, reason) }.map_err(|e| {
                    tracing::warn!(%e, "energia: ExitWindowsEx recusou");
                    AppError::Rejected
                })
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

    #[test]
    fn flags_de_desligar_e_reiniciar_nunca_forcam() {
        let (off, reason) = exit_flags(PowerAction::Shutdown).unwrap();
        let (reboot, _) = exit_flags(PowerAction::Restart).unwrap();
        // EWX_POWEROFF (0x8) e EWX_REBOOT (0x2), nada alem disso: sem os bits de forcar (0x4 e
        // 0x10) nem os de avanco de tela de bloqueio.
        assert_eq!(off.0, 0x8);
        assert_eq!(reboot.0, 0x2);
        assert_eq!(off.0 & 0x14, 0);
        assert_eq!(reboot.0 & 0x14, 0);
        // Desligamento planejado.
        assert_ne!(reason.0 & 0x8000_0000, 0);
        assert!(exit_flags(PowerAction::Suspend).is_none());
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
