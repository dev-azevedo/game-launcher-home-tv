//! `HKCU\Software\Microsoft\Windows\CurrentVersion\Run` (BE-36, SEC-22): o valor `Sofa` com
//! `"<exe>" --autostart`, respeitando o `StartupApproved` (o usuario pode desligar a entrada no
//! Gerenciador de Tarefas). Unico arquivo que toca o `Run` e o `StartupApproved` (INV-03).
//! Escreve so no HKCU do usuario do processo (SEC-29); nunca em HKLM.

use launcher_core::application::ports::{RunEntry, RunState, SchedulerError};
use launcher_core::domain::autostart::{
    run_command_line, run_value_matches, startup_approved_disabled, RUN_VALUE_NAME,
};
use windows::core::PCWSTR;
use windows::Win32::Foundation::{
    ERROR_FILE_NOT_FOUND, ERROR_PATH_NOT_FOUND, ERROR_SUCCESS, WIN32_ERROR,
};
use windows::Win32::System::Registry::{
    RegCloseKey, RegCreateKeyExW, RegDeleteValueW, RegOpenKeyExW, RegQueryValueExW, RegSetValueExW,
    HKEY, HKEY_CURRENT_USER, KEY_QUERY_VALUE, KEY_SET_VALUE, REG_OPTION_NON_VOLATILE, REG_SZ,
    REG_VALUE_TYPE,
};

const RUN_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";
const APPROVED_KEY: &str =
    r"Software\Microsoft\Windows\CurrentVersion\Explorer\StartupApproved\Run";

fn w(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(Some(0)).collect()
}

fn err(e: WIN32_ERROR) -> SchedulerError {
    SchedulerError(e.0 as i32)
}

fn is_missing(e: WIN32_ERROR) -> bool {
    e == ERROR_FILE_NOT_FOUND || e == ERROR_PATH_NOT_FOUND
}

/// Chave do HKCU aberta, fechada no `Drop`.
struct Key(HKEY);

impl Key {
    fn open(
        path: &str,
        access: windows::Win32::System::Registry::REG_SAM_FLAGS,
    ) -> Result<Key, WIN32_ERROR> {
        let path = w(path);
        let mut h = HKEY::default();
        // SAFETY: `path` termina em nulo e vive durante a chamada; `h` e local.
        let r =
            unsafe { RegOpenKeyExW(HKEY_CURRENT_USER, PCWSTR(path.as_ptr()), 0, access, &mut h) };
        if r == ERROR_SUCCESS {
            Ok(Key(h))
        } else {
            Err(r)
        }
    }

    fn create(path: &str) -> Result<Key, WIN32_ERROR> {
        let path = w(path);
        let mut h = HKEY::default();
        // SAFETY: como em `open`; sem classe nem atributos de seguranca (padrao do usuario).
        let r = unsafe {
            RegCreateKeyExW(
                HKEY_CURRENT_USER,
                PCWSTR(path.as_ptr()),
                0,
                PCWSTR::null(),
                REG_OPTION_NON_VOLATILE,
                KEY_SET_VALUE | KEY_QUERY_VALUE,
                None,
                &mut h,
                None,
            )
        };
        if r == ERROR_SUCCESS {
            Ok(Key(h))
        } else {
            Err(r)
        }
    }

    /// `(tipo, bytes)` do valor; `Ok(None)` se nao existe.
    fn read(&self, name: &str) -> Result<Option<(REG_VALUE_TYPE, Vec<u8>)>, WIN32_ERROR> {
        let name = w(name);
        let mut ty = REG_VALUE_TYPE(0);
        let mut len = 0u32;
        // SAFETY: primeira chamada so pergunta o tamanho; a segunda enche um buffer desse tamanho.
        unsafe {
            let r = RegQueryValueExW(
                self.0,
                PCWSTR(name.as_ptr()),
                None,
                Some(&mut ty),
                None,
                Some(&mut len),
            );
            if is_missing(r) {
                return Ok(None);
            }
            if r != ERROR_SUCCESS {
                return Err(r);
            }
            let mut buf = vec![0u8; len as usize];
            let r = RegQueryValueExW(
                self.0,
                PCWSTR(name.as_ptr()),
                None,
                Some(&mut ty),
                Some(buf.as_mut_ptr()),
                Some(&mut len),
            );
            if r != ERROR_SUCCESS {
                return Err(r);
            }
            buf.truncate(len as usize);
            Ok(Some((ty, buf)))
        }
    }

    fn read_string(&self, name: &str) -> Result<Option<String>, WIN32_ERROR> {
        Ok(self.read(name)?.and_then(|(ty, bytes)| {
            (ty == REG_SZ).then(|| {
                let units: Vec<u16> = bytes
                    .chunks_exact(2)
                    .map(|c| u16::from_le_bytes([c[0], c[1]]))
                    .take_while(|&u| u != 0)
                    .collect();
                String::from_utf16_lossy(&units)
            })
        }))
    }

    fn set_string(&self, name: &str, value: &str) -> Result<(), WIN32_ERROR> {
        let name = w(name);
        let data: Vec<u8> = w(value).iter().flat_map(|u| u.to_le_bytes()).collect();
        // SAFETY: nome e dados vivem durante a chamada.
        let r = unsafe { RegSetValueExW(self.0, PCWSTR(name.as_ptr()), 0, REG_SZ, Some(&data)) };
        if r == ERROR_SUCCESS {
            Ok(())
        } else {
            Err(r)
        }
    }

    /// Apagar valor ausente nao e erro.
    fn delete(&self, name: &str) -> Result<(), WIN32_ERROR> {
        let name = w(name);
        // SAFETY: nome vive durante a chamada.
        let r = unsafe { RegDeleteValueW(self.0, PCWSTR(name.as_ptr())) };
        if r == ERROR_SUCCESS || is_missing(r) {
            Ok(())
        } else {
            Err(r)
        }
    }
}

impl Drop for Key {
    fn drop(&mut self) {
        // SAFETY: o handle foi aberto em `open`/`create` e so e fechado aqui.
        unsafe {
            let _ = RegCloseKey(self.0);
        }
    }
}

#[derive(Debug, Clone)]
pub struct WinRunEntry {
    value_name: String,
}

impl WinRunEntry {
    /// O valor real do Sofa (`Sofa`).
    pub fn new() -> Self {
        Self {
            value_name: RUN_VALUE_NAME.to_owned(),
        }
    }

    /// Nome de valor proprio, para os testes de integracao nao tocarem no valor real do usuario.
    pub fn with_value_name(name: &str) -> Self {
        Self {
            value_name: name.to_owned(),
        }
    }

    fn approved_disabled(&self) -> Result<bool, SchedulerError> {
        match Key::open(APPROVED_KEY, KEY_QUERY_VALUE) {
            Ok(k) => Ok(k
                .read(&self.value_name)
                .map_err(err)?
                .is_some_and(|(_, bytes)| startup_approved_disabled(&bytes))),
            Err(e) if is_missing(e) => Ok(false),
            Err(e) => Err(err(e)),
        }
    }
}

impl Default for WinRunEntry {
    fn default() -> Self {
        Self::new()
    }
}

impl RunEntry for WinRunEntry {
    fn state(&self, expected_exe: Option<&str>) -> Result<RunState, SchedulerError> {
        let key = match Key::open(RUN_KEY, KEY_QUERY_VALUE) {
            Ok(k) => k,
            Err(e) if is_missing(e) => return Ok(RunState::Absent),
            Err(e) => return Err(err(e)),
        };
        let Some(value) = key.read_string(&self.value_name).map_err(err)? else {
            return Ok(RunState::Absent);
        };
        if !run_value_matches(&value, expected_exe) {
            return Ok(RunState::Foreign);
        }
        Ok(if self.approved_disabled()? {
            RunState::DisabledByUser
        } else {
            RunState::Enabled
        })
    }

    fn set(&self, exe: &str) -> Result<(), SchedulerError> {
        let key = Key::create(RUN_KEY).map_err(err)?;
        key.set_string(&self.value_name, &run_command_line(exe))
            .map_err(err)?;
        // O usuario esta ligando a opcao de proposito: tira o "desligado" do Gerenciador de
        // Tarefas, senao o Windows ignoraria a entrada.
        if self.approved_disabled()? {
            if let Ok(k) = Key::open(APPROVED_KEY, KEY_SET_VALUE) {
                k.delete(&self.value_name).map_err(err)?;
            }
        }
        Ok(())
    }

    fn remove(&self) -> Result<(), SchedulerError> {
        match Key::open(RUN_KEY, KEY_SET_VALUE) {
            Ok(k) => k.delete(&self.value_name).map_err(err)?,
            Err(e) if is_missing(e) => {}
            Err(e) => return Err(err(e)),
        }
        match Key::open(APPROVED_KEY, KEY_SET_VALUE) {
            Ok(k) => k.delete(&self.value_name).map_err(err),
            Err(e) if is_missing(e) => Ok(()),
            Err(e) => Err(err(e)),
        }
    }
}
