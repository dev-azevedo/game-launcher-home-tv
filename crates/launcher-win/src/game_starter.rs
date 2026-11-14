//! `GameStarter` Win32 (BE-16): como cada jogo e iniciado (arquitetura 2.2, SEC-10, SEC-11, AF-01).
//!
//! - Exe (manual e GOG): `CreateProcessW` **suspenso**, sem shell, `lpApplicationName` absoluto,
//!   `bInheritHandles = FALSE`, argumentos montados como lista (nunca texto de shell), dentro de
//!   um Job Object; so entao `ResumeThread`. O pai do jogo e o Sofa, nunca `cmd.exe`/`powershell.exe`.
//! - Steam e Epic: `ShellExecuteExW` com o template fixo do dominio, verbo padrao.
//! - Erro 740 (`ERROR_ELEVATION_REQUIRED`): `NeedsElevation`. O UAC so acontece em
//!   `StartVerb::Elevated` (depois do aviso do Sofa): `ShellExecuteExW` com o verbo padrao
//!   (nunca `runas`), que o Windows resolve pelo manifesto do jogo.

use launcher_core::application::ports::{GameStarter, StartError, StartVerb, Started};
use launcher_core::domain::library::{
    check_launch_exe_path, epic_uri, steam_uri, LaunchPathError, LaunchSpec, LaunchTarget,
};
use windows::core::{PCWSTR, PWSTR};
use windows::Win32::Foundation::CloseHandle;
use windows::Win32::Storage::FileSystem::GetDriveTypeW;
use windows::Win32::System::Com::{
    CoInitializeEx, CoUninitialize, COINIT_APARTMENTTHREADED, COINIT_DISABLE_OLE1DDE,
};
use windows::Win32::System::Registry::{
    RegCloseKey, RegOpenKeyExW, HKEY, HKEY_CLASSES_ROOT, KEY_READ,
};
use windows::Win32::System::Threading::{
    CreateProcessW, ResumeThread, CREATE_SUSPENDED, PROCESS_INFORMATION, STARTUPINFOW,
};
use windows::Win32::UI::Shell::{
    ShellExecuteExW, SEE_MASK_FLAG_NO_UI, SEE_MASK_NOASYNC, SHELLEXECUTEINFOW,
};
use windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;

use crate::job::Job;

const DRIVE_REMOVABLE: u32 = 2;
const DRIVE_FIXED: u32 = 3;
const DRIVE_REMOTE: u32 = 4;
const ERROR_FILE_NOT_FOUND: u32 = 2;
const ERROR_PATH_NOT_FOUND: u32 = 3;
const ERROR_CANCELLED: u32 = 1223;
const ERROR_ELEVATION_REQUIRED: u32 = 740;

#[derive(Debug, Default)]
pub struct WinGameStarter;

/// Escapa um argumento pela regra do `CommandLineToArgvW` (a mesma de `CreateProcessW` e de
/// `lpParameters`): barras antes de aspas dobram, aspas viram `\"`, e o argumento so ganha aspas
/// se estiver vazio ou tiver espaco, tabulacao ou aspas.
pub fn quote_arg(arg: &str) -> String {
    if !arg.is_empty() && !arg.contains([' ', '\t', '\n', '\x0b', '"']) {
        return arg.to_owned();
    }
    let mut out = String::with_capacity(arg.len() + 2);
    out.push('"');
    let mut backslashes = 0usize;
    for c in arg.chars() {
        match c {
            '\\' => backslashes += 1,
            '"' => {
                out.extend(std::iter::repeat('\\').take(backslashes * 2 + 1));
                out.push('"');
                backslashes = 0;
            }
            _ => {
                out.extend(std::iter::repeat('\\').take(backslashes));
                out.push(c);
                backslashes = 0;
            }
        }
    }
    out.extend(std::iter::repeat('\\').take(backslashes * 2));
    out.push('"');
    out
}

/// `"<exe>" arg1 arg2`: o exe sempre entre aspas; cada argumento pela mesma rotina de escape.
pub fn build_command_line(exe: &str, args: &[String]) -> String {
    let mut line = format!("\"{exe}\"");
    for a in args {
        line.push(' ');
        line.push_str(&quote_arg(a));
    }
    line
}

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

/// Codigo Win32 de um erro do windows-rs (`HRESULT_FROM_WIN32`).
fn win32_code(e: &windows::core::Error) -> u32 {
    let hr = e.code().0 as u32;
    if hr & 0xFFFF_0000 == 0x8007_0000 {
        hr & 0xFFFF
    } else {
        hr
    }
}

/// Forma do caminho + tipo da unidade + arquivo existente.
fn check_exe(path: &str) -> Result<(), StartError> {
    match check_launch_exe_path(path) {
        Ok(()) => {}
        Err(LaunchPathError::Network) => return Err(StartError::NetworkPath),
        Err(LaunchPathError::Invalid) => return Err(StartError::FileMissing),
    }
    let root = wide(&path[..3]);
    // SAFETY: `root` e NUL-terminada e vive durante a chamada.
    let drive = unsafe { GetDriveTypeW(PCWSTR(root.as_ptr())) };
    if drive == DRIVE_REMOTE {
        return Err(StartError::NetworkPath);
    }
    if drive != DRIVE_FIXED && drive != DRIVE_REMOVABLE {
        return Err(StartError::FileMissing);
    }
    match std::fs::metadata(path) {
        Ok(m) if m.is_file() => Ok(()),
        _ => Err(StartError::FileMissing),
    }
}

/// O esquema de URI da loja esta registrado? Sem ele o Windows abriria um dialogo de "como abrir".
fn scheme_registered(scheme: &str) -> bool {
    let name = wide(scheme);
    let mut key = HKEY::default();
    // SAFETY: `name` NUL-terminada; `key` fechado logo depois se abriu.
    unsafe {
        if RegOpenKeyExW(
            HKEY_CLASSES_ROOT,
            PCWSTR(name.as_ptr()),
            0,
            KEY_READ,
            &mut key,
        )
        .is_ok()
        {
            let _ = RegCloseKey(key);
            true
        } else {
            false
        }
    }
}

fn scheme_of(spec: &LaunchSpec) -> Option<&'static str> {
    match spec {
        LaunchSpec::SteamUri { .. } => Some("steam"),
        LaunchSpec::EpicUri { .. } => Some("com.epicgames.launcher"),
        LaunchSpec::Exe { .. } => None,
    }
}

/// `ShellExecuteExW` com o verbo padrao, numa thread propria com COM em STA (alguns handlers de
/// protocolo precisam). `Err` leva o codigo Win32.
fn shell_execute(file: &str, params: Option<&str>, dir: Option<&str>) -> Result<(), u32> {
    let file = wide(file);
    let params = params.map(wide);
    let dir = dir.map(wide);
    std::thread::scope(|s| {
        s.spawn(|| {
            // SAFETY: COM inicializado e desfeito na mesma thread; os buffers vivem ate o fim do
            // escopo; `lpVerb` nulo = verbo padrao (nunca `runas`).
            unsafe {
                let com = CoInitializeEx(None, COINIT_APARTMENTTHREADED | COINIT_DISABLE_OLE1DDE);
                let mut info = SHELLEXECUTEINFOW {
                    cbSize: std::mem::size_of::<SHELLEXECUTEINFOW>() as u32,
                    fMask: SEE_MASK_NOASYNC | SEE_MASK_FLAG_NO_UI,
                    lpFile: PCWSTR(file.as_ptr()),
                    lpParameters: params
                        .as_ref()
                        .map_or(PCWSTR::null(), |p| PCWSTR(p.as_ptr())),
                    lpDirectory: dir.as_ref().map_or(PCWSTR::null(), |d| PCWSTR(d.as_ptr())),
                    nShow: SW_SHOWNORMAL.0,
                    ..Default::default()
                };
                let r = ShellExecuteExW(&mut info).map_err(|e| win32_code(&e));
                if com.is_ok() {
                    CoUninitialize();
                }
                r
            }
        })
        .join()
        .unwrap_or(Err(0))
    })
}

fn create_process(
    path: &str,
    args: &[String],
    working_dir: Option<&str>,
) -> Result<Started, StartError> {
    if path.contains('\0') || args.iter().any(|a| a.contains('\0')) {
        return Err(StartError::Failed);
    }
    let app = wide(path);
    let mut cmd = wide(&build_command_line(path, args));
    let dir = working_dir.map(wide);
    let si = STARTUPINFOW {
        cb: std::mem::size_of::<STARTUPINFOW>() as u32,
        ..Default::default()
    };
    let mut pi = PROCESS_INFORMATION::default();
    // SAFETY: buffers NUL-terminados vivem durante a chamada; `bInheritHandles = false`; os
    // handles devolvidos sao fechados abaixo em todos os caminhos.
    let created = unsafe {
        CreateProcessW(
            PCWSTR(app.as_ptr()),
            PWSTR(cmd.as_mut_ptr()),
            None,
            None,
            false,
            CREATE_SUSPENDED,
            None,
            dir.as_ref().map_or(PCWSTR::null(), |d| PCWSTR(d.as_ptr())),
            &si,
            &mut pi,
        )
    };
    if let Err(e) = created {
        return Err(match win32_code(&e) {
            ERROR_ELEVATION_REQUIRED => StartError::NeedsElevation { fingerprint: None },
            ERROR_FILE_NOT_FOUND | ERROR_PATH_NOT_FOUND => StartError::FileMissing,
            code => {
                tracing::warn!(codigo = code, "CreateProcessW falhou");
                StartError::Failed
            }
        });
    }
    // Job antes de soltar o processo: filhos ja nascem dentro dele.
    let job = match Job::new() {
        Ok(job) => match job.assign(pi.hProcess) {
            Ok(()) => Some(job),
            Err(e) => {
                tracing::warn!(erro = %e, "processo nao entrou no Job; a sonda varre por pasta");
                None
            }
        },
        Err(e) => {
            tracing::warn!(erro = %e, "Job Object nao criado; a sonda varre por pasta");
            None
        }
    };
    // SAFETY: handles do `CreateProcessW` acima.
    unsafe {
        ResumeThread(pi.hThread);
        let _ = CloseHandle(pi.hThread);
        let _ = CloseHandle(pi.hProcess);
    }
    Ok(Started {
        job: job.map(|j| Box::new(j) as Box<dyn std::any::Any + Send>),
    })
}

impl GameStarter for WinGameStarter {
    fn preflight(&self, target: &LaunchTarget) -> Result<(), StartError> {
        match &target.spec {
            LaunchSpec::Exe { path, .. } => check_exe(path),
            spec => {
                let scheme = scheme_of(spec).unwrap_or_default();
                if scheme_registered(scheme) {
                    Ok(())
                } else {
                    Err(StartError::StoreNotInstalled)
                }
            }
        }
    }

    fn start(&self, target: &LaunchTarget, verb: StartVerb) -> Result<Started, StartError> {
        self.preflight(target)?; // revalida imediatamente antes (SEC-09, AF-01)
        match (&target.spec, verb) {
            (LaunchSpec::SteamUri { app_id }, _) => shell_execute(&steam_uri(*app_id), None, None)
                .map(|()| Started::default())
                .map_err(|c| {
                    tracing::warn!(codigo = c, "ShellExecuteExW (Steam) falhou");
                    StartError::StoreNotInstalled
                }),
            (
                LaunchSpec::EpicUri {
                    namespace,
                    catalog_item_id,
                    app_name,
                },
                _,
            ) => shell_execute(&epic_uri(namespace, catalog_item_id, app_name), None, None)
                .map(|()| Started::default())
                .map_err(|c| {
                    tracing::warn!(codigo = c, "ShellExecuteExW (Epic) falhou");
                    StartError::StoreNotInstalled
                }),
            (
                LaunchSpec::Exe {
                    path,
                    args,
                    working_dir,
                },
                StartVerb::Normal,
            ) => create_process(path, args, working_dir.as_deref()).map_err(|e| match e {
                StartError::NeedsElevation { .. } => StartError::NeedsElevation {
                    fingerprint: self.fingerprint(target),
                },
                other => other,
            }),
            (
                LaunchSpec::Exe {
                    path,
                    args,
                    working_dir,
                },
                StartVerb::Elevated,
            ) => {
                // AF-01: verbo padrao, parametros pela mesma rotina de escape, so `.exe` (ja
                // conferido no preflight). Fora do Job: o Windows nao deixa (UIPI).
                let params = (!args.is_empty()).then(|| {
                    args.iter()
                        .map(|a| quote_arg(a))
                        .collect::<Vec<_>>()
                        .join(" ")
                });
                shell_execute(path, params.as_deref(), working_dir.as_deref())
                    .map(|()| Started::default())
                    .map_err(|c| match c {
                        ERROR_CANCELLED => StartError::ElevationDenied,
                        ERROR_FILE_NOT_FOUND | ERROR_PATH_NOT_FOUND => StartError::FileMissing,
                        other => {
                            tracing::warn!(codigo = other, "ShellExecuteExW (elevado) falhou");
                            StartError::Failed
                        }
                    })
            }
        }
    }

    fn fingerprint(&self, target: &LaunchTarget) -> Option<String> {
        let LaunchSpec::Exe { path, .. } = &target.spec else {
            return None;
        };
        let meta = std::fs::metadata(path).ok()?;
        let modified = meta
            .modified()
            .ok()
            .and_then(|m| m.duration_since(std::time::UNIX_EPOCH).ok())
            .map_or(0, |d| d.as_nanos());
        Some(format!(
            "{}|{}|{}",
            path.to_lowercase(),
            meta.len(),
            modified
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escape_de_argumentos_pela_regra_do_windows() {
        let casos = [
            ("simples", "simples"),
            ("", "\"\""),
            ("com espaco", "\"com espaco\""),
            ("a&b", "a&b"), // `&` nao e especial sem shell
            (r#"diz "oi""#, r#""diz \"oi\"""#),
            (r"C:\pasta\", r"C:\pasta\"),
            (r"C:\pasta com espaco\", r#""C:\pasta com espaco\\""#),
            (r#"a\"b"#, r#""a\\\"b""#),
        ];
        for (entrada, esperado) in casos {
            assert_eq!(quote_arg(entrada), esperado, "{entrada}");
        }
    }

    #[test]
    fn linha_de_comando_poe_o_exe_entre_aspas() {
        let line = build_command_line(
            r"C:\Jogos\A & B\game.exe",
            &["-windowed".into(), "um valor".into()],
        );
        assert_eq!(line, r#""C:\Jogos\A & B\game.exe" -windowed "um valor""#);
    }

    #[test]
    fn caminho_de_rede_e_forma_invalida_nao_chegam_ao_createprocess() {
        let t = |p: &str| LaunchTarget {
            game: launcher_core::domain::library::GameId::parse("manual:9f3a1c2b7d4e5a60").unwrap(),
            spec: LaunchSpec::Exe {
                path: p.into(),
                args: vec![],
                working_dir: None,
            },
            install_dir: None,
        };
        let s = WinGameStarter;
        assert_eq!(
            s.preflight(&t(r"\\127.0.0.1\c$\x.exe")),
            Err(StartError::NetworkPath)
        );
        assert_eq!(
            s.preflight(&t(r"C:\x\script.bat")),
            Err(StartError::FileMissing)
        );
        assert_eq!(
            s.preflight(&t(r"C:\nao\existe\x.exe")),
            Err(StartError::FileMissing)
        );
        assert_eq!(s.fingerprint(&t(r"C:\nao\existe\x.exe")), None);
    }
}
