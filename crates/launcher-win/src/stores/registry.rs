//! Leitura do registro para os detectores (Steam: `HKCU`; GOG: `HKLM`). So leitura, valores de
//! texto com tamanho limitado. Um seam com dois adapters: o do Windows e o de teste.

use windows::core::{PCWSTR, PWSTR};
use windows::Win32::Foundation::ERROR_SUCCESS;
use windows::Win32::System::Registry::{
    RegCloseKey, RegEnumKeyExW, RegGetValueW, RegOpenKeyExW, HKEY, HKEY_CURRENT_USER,
    HKEY_LOCAL_MACHINE, KEY_READ, RRF_RT_REG_SZ,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Hive {
    CurrentUser,
    LocalMachine,
}

pub trait RegistryReader: Send + Sync {
    /// Valor de texto (`REG_SZ`) de `<hive>\<subkey>`, se existe e cabe no limite.
    fn read_string(&self, hive: Hive, subkey: &str, value: &str) -> Option<String>;
    /// Nomes das subchaves de `<hive>\<subkey>` (no maximo `MAX_SUBKEYS`).
    fn subkeys(&self, hive: Hive, subkey: &str) -> Vec<String>;
}

/// Maximo de caracteres de um valor lido (caminhos do Windows cabem em 1.024).
const MAX_VALUE_CHARS: usize = 2048;
/// Maximo de subchaves lidas de uma chave (jogos GOG).
pub const MAX_SUBKEYS: usize = 5_000;

#[derive(Debug, Default, Clone, Copy)]
pub struct WinRegistry;

fn root(hive: Hive) -> HKEY {
    match hive {
        Hive::CurrentUser => HKEY_CURRENT_USER,
        Hive::LocalMachine => HKEY_LOCAL_MACHINE,
    }
}

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(Some(0)).collect()
}

impl RegistryReader for WinRegistry {
    fn read_string(&self, hive: Hive, subkey: &str, value: &str) -> Option<String> {
        let (key, name) = (wide(subkey), wide(value));
        let mut buf = vec![0u16; MAX_VALUE_CHARS];
        let mut size = (buf.len() * 2) as u32;
        // SAFETY: buffers validos e do tamanho informado; so leitura de REG_SZ.
        let rc = unsafe {
            RegGetValueW(
                root(hive),
                PCWSTR(key.as_ptr()),
                PCWSTR(name.as_ptr()),
                RRF_RT_REG_SZ,
                None,
                Some(buf.as_mut_ptr().cast()),
                Some(&mut size),
            )
        };
        if rc != ERROR_SUCCESS {
            return None; // ausente, tipo errado ou maior que o limite (ERROR_MORE_DATA)
        }
        let len = buf.iter().position(|c| *c == 0).unwrap_or(buf.len());
        Some(String::from_utf16_lossy(&buf[..len]))
    }

    fn subkeys(&self, hive: Hive, subkey: &str) -> Vec<String> {
        let key = wide(subkey);
        let mut handle = HKEY::default();
        // SAFETY: `handle` local; fechado abaixo em todos os caminhos.
        let rc =
            unsafe { RegOpenKeyExW(root(hive), PCWSTR(key.as_ptr()), 0, KEY_READ, &mut handle) };
        if rc != ERROR_SUCCESS {
            return Vec::new();
        }
        let mut out = Vec::new();
        for index in 0..MAX_SUBKEYS as u32 {
            let mut name = [0u16; 256]; // limite de nome de chave do registro
            let mut len = name.len() as u32;
            // SAFETY: `name` e `len` consistentes; demais ponteiros opcionais nulos.
            let rc = unsafe {
                RegEnumKeyExW(
                    handle,
                    index,
                    PWSTR(name.as_mut_ptr()),
                    &mut len,
                    None,
                    PWSTR::null(),
                    None,
                    None,
                )
            };
            if rc != ERROR_SUCCESS {
                break; // ERROR_NO_MORE_ITEMS ou falha: fim da lista
            }
            out.push(String::from_utf16_lossy(&name[..len as usize]));
        }
        // SAFETY: `handle` aberto acima.
        unsafe {
            let _ = RegCloseKey(handle);
        }
        out
    }
}

#[cfg(test)]
pub(crate) mod fake {
    use super::*;
    use std::collections::BTreeMap;

    /// Registro de mentira: `(hive, subkey) -> valor -> dado` e subchaves por chave.
    #[derive(Default)]
    pub struct FakeRegistry {
        pub values: BTreeMap<(bool, String, String), String>,
        pub subkeys: BTreeMap<(bool, String), Vec<String>>,
    }

    fn is_user(h: Hive) -> bool {
        h == Hive::CurrentUser
    }

    impl FakeRegistry {
        pub fn set(&mut self, hive: Hive, key: &str, name: &str, value: &str) {
            self.values.insert(
                (is_user(hive), key.to_owned(), name.to_owned()),
                value.to_owned(),
            );
        }
        pub fn add_subkey(&mut self, hive: Hive, key: &str, sub: &str) {
            self.subkeys
                .entry((is_user(hive), key.to_owned()))
                .or_default()
                .push(sub.to_owned());
        }
    }

    impl RegistryReader for FakeRegistry {
        fn read_string(&self, hive: Hive, subkey: &str, value: &str) -> Option<String> {
            self.values
                .get(&(is_user(hive), subkey.to_owned(), value.to_owned()))
                .cloned()
        }
        fn subkeys(&self, hive: Hive, subkey: &str) -> Vec<String> {
            self.subkeys
                .get(&(is_user(hive), subkey.to_owned()))
                .cloned()
                .unwrap_or_default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn le_valor_e_subchaves_do_registro_de_verdade() {
        let r = WinRegistry;
        // `ProductName` existe em qualquer Windows.
        let name = r
            .read_string(
                Hive::LocalMachine,
                r"SOFTWARE\Microsoft\Windows NT\CurrentVersion",
                "ProductName",
            )
            .expect("ProductName");
        assert!(name.contains("Windows"), "{name}");
        assert_eq!(
            r.read_string(Hive::CurrentUser, r"Software\Sofa\NaoExiste", "x"),
            None
        );
        assert!(r
            .subkeys(Hive::CurrentUser, r"Software\Sofa\NaoExiste")
            .is_empty());
        assert!(!r.subkeys(Hive::CurrentUser, "Software").is_empty());
    }
}
