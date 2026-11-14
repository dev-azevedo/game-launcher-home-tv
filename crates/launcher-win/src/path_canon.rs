//! Canonicalizacao de caminhos e tipo de unidade (SEC-09, SEC-12, AF-02). Adapter fino: as regras
//! sobre o que um caminho pode ser ficam em `launcher-core::domain::library`; aqui so o que
//! precisa do disco (resolver links, `GetDriveTypeW`, pastas conhecidas do Windows).

use std::io;
use std::os::windows::ffi::OsStrExt;
use std::path::Path;

use windows::core::PCWSTR;
use windows::Win32::Storage::FileSystem::{GetDriveTypeW, GetLogicalDrives};

/// Valores de retorno de `GetDriveTypeW`.
const DRIVE_REMOVABLE: u32 = 2;
const DRIVE_FIXED: u32 = 3;

/// Tipo da unidade (`GetDriveTypeW`). So `Fixed` e `Removable` sao locais para o Sofa.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DriveKind {
    Fixed,
    Removable,
    /// Rede, mapeada, CD, RAM disk, raiz inexistente.
    Other,
}

impl DriveKind {
    pub fn is_local(self) -> bool {
        matches!(self, DriveKind::Fixed | DriveKind::Removable)
    }
}

/// Tipo da unidade de uma letra (`'C'`).
pub fn drive_kind_of_letter(letter: char) -> DriveKind {
    if !letter.is_ascii_alphabetic() {
        return DriveKind::Other;
    }
    let root: Vec<u16> = format!("{}:\\", letter.to_ascii_uppercase())
        .encode_utf16()
        .chain(Some(0))
        .collect();
    // SAFETY: `root` termina em nulo e vive durante a chamada; so consulta.
    let t = unsafe { GetDriveTypeW(PCWSTR(root.as_ptr())) };
    match t {
        DRIVE_FIXED => DriveKind::Fixed,
        DRIVE_REMOVABLE => DriveKind::Removable,
        _ => DriveKind::Other,
    }
}

/// Tipo da unidade de um caminho `X:\...`. Qualquer outra forma (UNC, `\\?\`, relativo) e `Other`.
pub fn drive_kind(path: &str) -> DriveKind {
    let b = path.as_bytes();
    if b.len() >= 2 && b[0].is_ascii_alphabetic() && b[1] == b':' {
        drive_kind_of_letter(char::from(b[0]))
    } else {
        DriveKind::Other
    }
}

/// Unidades locais (fixas e removiveis) como `C:\`, `D:\`.
pub fn local_drives() -> Vec<String> {
    // SAFETY: sem argumentos; devolve a mascara de unidades.
    let mask = unsafe { GetLogicalDrives() };
    (0..26u8)
        .filter(|i| mask & (1 << i) != 0)
        .map(|i| char::from(b'A' + i))
        .filter(|c| drive_kind_of_letter(*c).is_local())
        .map(|c| format!("{c}:\\"))
        .collect()
}

/// Resolve links e devolve o caminho no formato do Sofa (`X:\...`, sem `\\?\`). `\\?\UNC\...`
/// continua com a forma de rede para a politica recusar. Mapeamento de unidade de rede resolve
/// para UNC e cai fora da regra de caminho local.
pub fn canonicalize(path: &str) -> io::Result<String> {
    let canonical = std::fs::canonicalize(Path::new(path))?;
    let text = canonical.to_string_lossy().into_owned();
    Ok(strip_verbatim(&text))
}

/// `\\?\C:\x` -> `C:\x`. `\\?\UNC\srv\share` -> `\\srv\share` (continua sendo rede).
pub fn strip_verbatim(path: &str) -> String {
    if let Some(rest) = path.strip_prefix(r"\\?\UNC\") {
        return format!(r"\\{rest}");
    }
    if let Some(rest) = path.strip_prefix(r"\\?\") {
        let b = rest.as_bytes();
        if b.len() >= 2 && b[0].is_ascii_alphabetic() && b[1] == b':' {
            return rest.to_owned();
        }
    }
    path.to_owned()
}

/// Como os detectores acham o disco (resolver links, saber se existe). Dois adapters: o do
/// sistema de arquivos e o de teste.
pub trait PathResolver: Send + Sync {
    /// Caminho canonico, ou `None` se nao existe (inclui unidade desconectada, E-18).
    fn canonicalize(&self, path: &str) -> Option<String>;
}

/// Resolve no disco de verdade. So devolve caminho em unidade local: o que resolve para rede ou
/// para unidade que nao e fixa/removivel conta como inexistente para as lojas.
#[derive(Debug, Default, Clone, Copy)]
pub struct FsPathResolver;

impl PathResolver for FsPathResolver {
    fn canonicalize(&self, path: &str) -> Option<String> {
        let canonical = canonicalize(path).ok()?;
        drive_kind(&canonical).is_local().then_some(canonical)
    }
}

// ----------------------------------------------------------------------- pastas conhecidas

/// Pastas do Windows usadas pelas politicas e pelos detectores. `SHGetKnownFolderPath` (nao
/// variavel de ambiente: o usuario controla o ambiente do processo, SP-08).
pub mod known {
    use std::path::PathBuf;

    use windows::core::GUID;
    use windows::Win32::System::Com::CoTaskMemFree;
    use windows::Win32::UI::Shell::{
        FOLDERID_LocalAppData, FOLDERID_Profile, FOLDERID_ProgramData, FOLDERID_ProgramFiles,
        FOLDERID_ProgramFilesX86, FOLDERID_RoamingAppData, FOLDERID_Windows, SHGetKnownFolderPath,
        KNOWN_FOLDER_FLAG,
    };

    fn folder(id: &GUID) -> Option<String> {
        // SAFETY: `SHGetKnownFolderPath` devolve memoria do COM, liberada com `CoTaskMemFree`
        // depois de copiar o texto; o ponteiro nao e usado depois.
        unsafe {
            let p = SHGetKnownFolderPath(id, KNOWN_FOLDER_FLAG(0), None).ok()?;
            let text = p.to_string().ok();
            CoTaskMemFree(Some(p.0.cast()));
            text
        }
    }

    pub fn windows_dir() -> Option<String> {
        folder(&FOLDERID_Windows)
    }
    pub fn program_files() -> Option<String> {
        folder(&FOLDERID_ProgramFiles)
    }
    pub fn program_files_x86() -> Option<String> {
        folder(&FOLDERID_ProgramFilesX86)
    }
    pub fn program_data() -> Option<String> {
        folder(&FOLDERID_ProgramData)
    }
    pub fn profile() -> Option<String> {
        folder(&FOLDERID_Profile)
    }
    pub fn local_app_data() -> Option<String> {
        folder(&FOLDERID_LocalAppData)
    }
    pub fn roaming_app_data() -> Option<String> {
        folder(&FOLDERID_RoamingAppData)
    }

    /// `%ProgramData%\Epic\EpicGamesLauncher\Data\Manifests`.
    pub fn epic_manifests_dir() -> Option<PathBuf> {
        Some(PathBuf::from(program_data()?).join(r"Epic\EpicGamesLauncher\Data\Manifests"))
    }
}

/// UTF-16 com terminador nulo para `PCWSTR`.
pub fn wide(path: &Path) -> Vec<u16> {
    path.as_os_str().encode_wide().chain(Some(0)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prefixo_verbatim_sai_e_unc_continua_rede() {
        assert_eq!(strip_verbatim(r"\\?\C:\Jogos\x.exe"), r"C:\Jogos\x.exe");
        assert_eq!(
            strip_verbatim(r"\\?\UNC\srv\share\x.exe"),
            r"\\srv\share\x.exe"
        );
        assert_eq!(strip_verbatim(r"C:\Jogos"), r"C:\Jogos");
        assert_eq!(
            strip_verbatim(r"\\?\GLOBALROOT\Device\x"),
            r"\\?\GLOBALROOT\Device\x"
        );
    }

    #[test]
    fn unidade_do_sistema_e_local_e_formas_estranhas_nao() {
        let sys = known::windows_dir().expect("pasta do Windows");
        assert!(drive_kind(&sys).is_local(), "{sys}");
        assert_eq!(drive_kind(r"\\127.0.0.1\c$\x"), DriveKind::Other);
        assert_eq!(drive_kind(r"\\?\C:\x"), DriveKind::Other);
        assert_eq!(drive_kind("jogo.exe"), DriveKind::Other);
        assert_eq!(drive_kind(""), DriveKind::Other);
        assert_eq!(drive_kind_of_letter('1'), DriveKind::Other);
        assert!(local_drives().iter().any(|d| sys.starts_with(d.as_str())));
    }

    #[test]
    fn canonicaliza_sem_o_prefixo_e_recusa_o_que_nao_existe() {
        let sys = known::windows_dir().unwrap();
        let c = FsPathResolver.canonicalize(&sys).unwrap();
        assert!(!c.starts_with(r"\\?\"), "{c}");
        // Maiusculas/minusculas e barras normais resolvem para a mesma pasta.
        let alt = sys.to_uppercase().replace('\\', "/");
        assert_eq!(
            FsPathResolver.canonicalize(&alt).as_deref(),
            Some(c.as_str())
        );
        assert_eq!(FsPathResolver.canonicalize(r"C:\nao\existe\mesmo"), None);
    }
}
