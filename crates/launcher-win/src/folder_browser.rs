//! Navegador de pastas (CA-19, CA-71, SEC-09, AF-02): so unidades `DRIVE_FIXED`/`DRIVE_REMOVABLE`,
//! pastas e `.exe`; nunca le conteudo de arquivo; unidade de rede ou mapeada nao aparece e, se
//! pedida por caminho, responde `PathNotAllowed`. A regra de ordem, filtro e limite (CT-04) e do
//! dominio (`dir_listing`); aqui so o disco.

use std::fs;
use std::io;
use std::os::windows::fs::MetadataExt;

use launcher_core::application::ports::FolderBrowser;
use launcher_core::application::AppError;
use launcher_core::domain::library::{
    dir_listing, drives_listing, BrowseTarget, DirListing, RawEntry,
};

use crate::path_canon::{canonicalize, drive_kind, local_drives};

/// Entradas lidas do disco antes de classificar. Pasta com mais do que isso e cortada de
/// qualquer jeito (`truncated`), sem ler dezenas de milhares de nomes.
const MAX_RAW_ENTRIES: usize = 100_000;

const FILE_ATTRIBUTE_HIDDEN: u32 = 0x2;
const FILE_ATTRIBUTE_SYSTEM: u32 = 0x4;

#[derive(Debug, Default, Clone, Copy)]
pub struct WinFolderBrowser;

fn map_io(error: &io::Error) -> AppError {
    match error.kind() {
        io::ErrorKind::NotFound => AppError::NotFound,
        io::ErrorKind::PermissionDenied => AppError::AccessDenied,
        // `ERROR_DIRECTORY` (nao e pasta), caminho invalido e afins: para o usuario, nao existe.
        _ => AppError::NotFound,
    }
}

impl FolderBrowser for WinFolderBrowser {
    fn list(&self, target: &BrowseTarget) -> Result<DirListing, AppError> {
        match target {
            BrowseTarget::Drives => Ok(drives_listing(local_drives())),
            BrowseTarget::Dir(path) => list_dir(path),
        }
    }
}

fn list_dir(path: &str) -> Result<DirListing, AppError> {
    // Unidade pedida: tem que ser local (mapeada de rede = PathNotAllowed, sem tocar nela).
    if !drive_kind(path).is_local() {
        return Err(AppError::PathNotAllowed);
    }
    let canonical = canonicalize(path).map_err(|e| map_io(&e))?;
    // Junction ou link que sai para rede (ou para outra unidade nao local) tambem e recusado.
    if !drive_kind(&canonical).is_local() {
        return Err(AppError::PathNotAllowed);
    }
    let meta = fs::metadata(&canonical).map_err(|e| map_io(&e))?;
    if !meta.is_dir() {
        return Err(AppError::NotFound);
    }

    let mut raw = Vec::new();
    let mut capped = false;
    for entry in fs::read_dir(&canonical).map_err(|e| map_io(&e))? {
        let Ok(entry) = entry else { continue };
        if raw.len() >= MAX_RAW_ENTRIES {
            capped = true;
            break;
        }
        // `DirEntry::file_type`/`metadata` nao seguem links: link e junction ficam de fora, para
        // a navegacao nao sair para outro lugar sem passar pela regra de unidade.
        let Ok(file_type) = entry.file_type() else {
            continue;
        };
        if file_type.is_symlink() {
            continue;
        }
        let Ok(meta) = entry.metadata() else { continue };
        if meta.file_attributes() & (FILE_ATTRIBUTE_HIDDEN | FILE_ATTRIBUTE_SYSTEM) != 0 {
            continue;
        }
        // Nome que nao e Unicode valido nao volta intacto para a UI: fica de fora.
        let Ok(name) = entry.file_name().into_string() else {
            continue;
        };
        raw.push(RawEntry {
            name,
            is_dir: file_type.is_dir(),
        });
    }
    let mut listing = dir_listing(&canonical, raw);
    listing.truncated |= capped;
    Ok(listing)
}

#[cfg(test)]
mod tests {
    use super::*;
    use launcher_core::domain::library::{DirEntryKind, MAX_DIR_ENTRIES};
    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicU32, Ordering};

    struct TempDir(PathBuf);
    impl TempDir {
        fn new() -> Self {
            static N: AtomicU32 = AtomicU32::new(0);
            let p = std::env::temp_dir().join(format!(
                "sofa-browse-{}-{}",
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

    fn list(path: &str) -> Result<DirListing, AppError> {
        WinFolderBrowser.list(&BrowseTarget::Dir(path.to_owned()))
    }

    #[test]
    fn unidades_so_locais_e_so_do_tipo_drive() {
        let l = WinFolderBrowser.list(&BrowseTarget::Drives).unwrap();
        assert_eq!(l.path, None);
        assert!(!l.entries.is_empty());
        assert!(l.entries.iter().all(|e| e.kind == DirEntryKind::Drive));
        assert!(l.entries.iter().all(|e| drive_kind(&e.name).is_local()));
    }

    #[test]
    fn lista_so_pastas_e_exe_pastas_primeiro_sem_ler_conteudo() {
        let t = TempDir::new();
        fs::create_dir(t.path().join("Beta")).unwrap();
        fs::create_dir(t.path().join("alfa")).unwrap();
        fs::write(t.path().join("zeta.exe"), b"MZ").unwrap();
        fs::write(t.path().join("Alfa.EXE"), b"qualquer coisa").unwrap();
        fs::write(t.path().join("leia.txt"), b"x").unwrap();
        fs::write(t.path().join("run.bat"), b"x").unwrap();
        let l = list(t.path().to_str().unwrap()).unwrap();
        let v: Vec<(String, DirEntryKind)> =
            l.entries.iter().map(|e| (e.name.clone(), e.kind)).collect();
        assert_eq!(
            v,
            [
                ("alfa".to_owned(), DirEntryKind::Dir),
                ("Beta".to_owned(), DirEntryKind::Dir),
                ("Alfa.EXE".to_owned(), DirEntryKind::Exe),
                ("zeta.exe".to_owned(), DirEntryKind::Exe),
            ]
        );
        assert!(!l.truncated);
        assert!(l.parent.is_some());
        assert!(drive_kind(l.path.as_deref().unwrap()).is_local());
    }

    #[test]
    fn pasta_enorme_e_cortada_em_2000_com_aviso() {
        let t = TempDir::new();
        for i in 0..(MAX_DIR_ENTRIES + 50) {
            fs::write(t.path().join(format!("f{i:05}.exe")), b"").unwrap();
        }
        fs::create_dir(t.path().join("sub")).unwrap();
        let l = list(t.path().to_str().unwrap()).unwrap();
        assert_eq!(l.entries.len(), MAX_DIR_ENTRIES);
        assert!(l.truncated);
        assert_eq!(l.entries[0].name, "sub"); // pasta antes de .exe, mesmo no corte
    }

    #[test]
    fn rede_e_prefixos_especiais_respondem_path_not_allowed_sem_tocar_na_rede() {
        for p in [
            r"\\127.0.0.1\c$",
            r"\\?\C:\Windows",
            r"\\.\C:\",
            r"\\?\UNC\127.0.0.1\c$",
        ] {
            assert_eq!(list(p).unwrap_err(), AppError::PathNotAllowed, "{p}");
        }
    }

    #[test]
    fn inexistente_arquivo_e_acesso_negado() {
        assert_eq!(
            list(r"C:\nao\existe\mesmo").unwrap_err(),
            AppError::NotFound
        );
        let t = TempDir::new();
        let f = t.path().join("a.exe");
        fs::write(&f, b"MZ").unwrap();
        assert_eq!(list(f.to_str().unwrap()).unwrap_err(), AppError::NotFound);
    }

    #[test]
    fn link_para_pasta_nao_aparece_na_listagem() {
        let t = TempDir::new();
        let alvo = t.path().join("alvo");
        fs::create_dir(&alvo).unwrap();
        // Junction via `mklink /J` (nao exige privilegio); se o ambiente nao deixar, o teste passa.
        let junction = t.path().join("atalho");
        let ok = std::process::Command::new("cmd")
            .args(["/C", "mklink", "/J"])
            .arg(&junction)
            .arg(&alvo)
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false);
        let l = list(t.path().to_str().unwrap()).unwrap();
        let nomes: Vec<&str> = l.entries.iter().map(|e| e.name.as_str()).collect();
        assert!(nomes.contains(&"alvo"));
        if ok {
            assert!(!nomes.contains(&"atalho"), "{nomes:?}");
        }
    }
}
