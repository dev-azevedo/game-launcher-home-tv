//! Confere no disco o `.exe` de um cadastro manual (SEC-09, RN-29, CA-72): canonicaliza, exige
//! unidade fixa ou removivel, arquivo regular e cabecalho `MZ` + `PE\0\0`. Le so o cabecalho
//! (no maximo 4 KB). A forma do caminho e do dominio (`check_manual_exe_path`).

use std::fs::File;
use std::io::{self, Read, Seek, SeekFrom};

use launcher_core::application::ports::ManualExeInspector;
use launcher_core::application::AppError;

use crate::path_canon::{canonicalize, drive_kind};

#[derive(Debug, Default, Clone, Copy)]
pub struct WinManualExeInspector;

fn map_io(error: &io::Error) -> AppError {
    match error.kind() {
        io::ErrorKind::PermissionDenied => AppError::AccessDenied,
        _ => AppError::NotFound,
    }
}

/// `MZ` no inicio, `e_lfanew` (offset 0x3C) apontando para `PE\0\0` dentro do arquivo.
fn has_pe_header(file: &mut File, len: u64) -> io::Result<bool> {
    let mut dos = [0u8; 0x40];
    if len < dos.len() as u64 {
        return Ok(false);
    }
    file.read_exact(&mut dos)?;
    if &dos[..2] != b"MZ" {
        return Ok(false);
    }
    let e_lfanew = u32::from_le_bytes([dos[0x3C], dos[0x3D], dos[0x3E], dos[0x3F]]);
    // O cabecalho PE tem de caber no arquivo; offset absurdo e arquivo forjado.
    if u64::from(e_lfanew) < 0x40 || u64::from(e_lfanew) + 4 > len {
        return Ok(false);
    }
    file.seek(SeekFrom::Start(u64::from(e_lfanew)))?;
    let mut sig = [0u8; 4];
    file.read_exact(&mut sig)?;
    Ok(&sig == b"PE\0\0")
}

impl ManualExeInspector for WinManualExeInspector {
    fn inspect(&self, path: &str) -> Result<String, AppError> {
        // Unidade pedida: local (mapeada de rede = PathNotAllowed, sem abrir nada nela).
        if !drive_kind(path).is_local() {
            return Err(AppError::PathNotAllowed);
        }
        let canonical = canonicalize(path).map_err(|e| map_io(&e))?;
        // Link que resolve para rede ou para outra unidade nao local.
        if !drive_kind(&canonical).is_local() {
            return Err(AppError::PathNotAllowed);
        }
        // Pasta chamada `x.exe` nao abre como arquivo (acesso negado): confere o tipo antes.
        let meta = std::fs::metadata(&canonical).map_err(|e| map_io(&e))?;
        if !meta.is_file() {
            return Err(AppError::NotAnExecutable);
        }
        let mut file = File::open(&canonical).map_err(|e| map_io(&e))?;
        match has_pe_header(&mut file, meta.len()) {
            Ok(true) => Ok(canonical),
            Ok(false) => Err(AppError::NotAnExecutable),
            Err(e) => match e.kind() {
                io::ErrorKind::UnexpectedEof => Err(AppError::NotAnExecutable),
                _ => Err(map_io(&e)),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicU32, Ordering};

    struct TempDir(PathBuf);
    impl TempDir {
        fn new() -> Self {
            static N: AtomicU32 = AtomicU32::new(0);
            let p = std::env::temp_dir().join(format!(
                "sofa-manual-{}-{}",
                std::process::id(),
                N.fetch_add(1, Ordering::SeqCst)
            ));
            fs::create_dir_all(&p).unwrap();
            Self(p)
        }
        fn path(&self) -> &Path {
            &self.0
        }
    }
    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    /// DOS stub minimo + `PE\0\0` em 0x80.
    fn fake_pe() -> Vec<u8> {
        let mut b = vec![0u8; 0x100];
        b[0] = b'M';
        b[1] = b'Z';
        b[0x3C] = 0x80;
        b[0x80..0x84].copy_from_slice(b"PE\0\0");
        b
    }

    fn inspect(p: &Path) -> Result<String, AppError> {
        WinManualExeInspector.inspect(p.to_str().unwrap())
    }

    #[test]
    fn exe_valido_devolve_o_caminho_canonico() {
        let t = TempDir::new();
        let exe = t.path().join("Meu Jogo & Cia.exe");
        fs::write(&exe, fake_pe()).unwrap();
        let c = inspect(&exe).unwrap();
        assert!(!c.starts_with(r"\\?\"));
        assert!(c.to_lowercase().ends_with("meu jogo & cia.exe"));
        // Outra caixa e barras normais resolvem para o mesmo caminho canonico.
        let alt = exe.to_str().unwrap().to_uppercase().replace('\\', "/");
        assert_eq!(
            WinManualExeInspector.inspect(&alt).unwrap().to_lowercase(),
            c.to_lowercase()
        );
    }

    #[test]
    fn txt_renomeado_para_exe_e_arquivos_forjados_sao_nao_executaveis() {
        let t = TempDir::new();
        let casos: Vec<(&str, Vec<u8>)> = vec![
            ("texto.exe", b"isto e so um texto".to_vec()),
            ("vazio.exe", Vec::new()),
            ("so_mz.exe", b"MZ".to_vec()),
            ("sem_pe.exe", {
                let mut b = fake_pe();
                b[0x80..0x84].copy_from_slice(b"XXXX");
                b
            }),
            ("lfanew_fora.exe", {
                let mut b = fake_pe();
                b[0x3C..0x40].copy_from_slice(&0xFFFF_FF00u32.to_le_bytes());
                b
            }),
            ("lfanew_dentro_do_dos.exe", {
                let mut b = fake_pe();
                b[0x3C] = 0x10;
                b
            }),
        ];
        for (nome, bytes) in casos {
            let p = t.path().join(nome);
            fs::write(&p, bytes).unwrap();
            assert_eq!(
                inspect(&p).unwrap_err(),
                AppError::NotAnExecutable,
                "{nome}"
            );
        }
    }

    #[test]
    fn pasta_chamada_exe_ausente_e_rede() {
        let t = TempDir::new();
        let dir = t.path().join("pasta.exe");
        fs::create_dir(&dir).unwrap();
        assert_eq!(inspect(&dir).unwrap_err(), AppError::NotAnExecutable);
        assert_eq!(
            inspect(&t.path().join("ausente.exe")).unwrap_err(),
            AppError::NotFound
        );
        for p in [r"\\127.0.0.1\c$\x.exe", r"\\?\C:\x.exe", r"x.exe"] {
            assert_eq!(
                WinManualExeInspector.inspect(p).unwrap_err(),
                AppError::PathNotAllowed,
                "{p}"
            );
        }
    }

    #[test]
    fn exe_de_verdade_do_windows_passa() {
        let sys = crate::path_canon::known::windows_dir().unwrap();
        let notepad = format!(r"{sys}\System32\notepad.exe");
        assert!(WinManualExeInspector.inspect(&notepad).is_ok());
    }
}
