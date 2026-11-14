//! Politicas do executavel (SEC-09, SEC-14) e do caminho pedido ao navegador de pastas (SEC-09,
//! AF-02). Estende `check_launch_exe_path` (forma do caminho no lancamento) sem recria-lo.
//! Puro: existencia, tipo da unidade e cabecalho `MZ`/`PE` sao do adapter.

use super::install_path_policy::{local_components, InstallPathPolicy};
use super::launch::{check_launch_exe_path, LaunchPathError};

/// Por que o caminho de um jogo manual nao pode ser cadastrado.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ManualPathError {
    /// Vazio, grande demais, controle ou nao absoluto: `InvalidRequest`.
    Malformed,
    /// Rede, UNC, `\\?\`, `\\.\`: `PathNotAllowed` (CA-71, 1o momento).
    NotAllowed,
    /// ADS, nome reservado, `..` ou nao e `.exe`: `NotAnExecutable` (CA-72).
    NotExecutable,
}

/// SEC-09 sobre a forma do caminho de um jogo manual, antes e depois de canonicalizar. A
/// existencia, o tipo da unidade e o `MZ`+`PE` ficam com o adapter.
pub fn check_manual_exe_path(path: &str) -> Result<(), ManualPathError> {
    if path.is_empty() || path.chars().count() > 1024 || path.chars().any(char::is_control) {
        return Err(ManualPathError::Malformed);
    }
    if path.starts_with("\\\\") || path.starts_with("//") {
        return Err(ManualPathError::NotAllowed); // UNC e prefixos `\\?\` e `\\.\`
    }
    let b = path.as_bytes();
    if b.len() < 3 || !b[0].is_ascii_alphabetic() || b[1] != b':' || b[2] != b'\\' {
        return Err(ManualPathError::Malformed); // relativo, `C:jogo.exe`, `/`
    }
    check_launch_exe_path(path).map_err(|e| match e {
        LaunchPathError::Network => ManualPathError::NotAllowed,
        LaunchPathError::Invalid => ManualPathError::NotExecutable,
    })
}

/// Por que o executavel de um jogo de loja e recusado.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StoreExeError {
    /// Forma invalida, rede ou nao e `.exe` (script, `.lnk`, ADS): SEC-14.
    Invalid,
    /// Dentro de `%SystemRoot%` (`cmd.exe`, `powershell.exe`...): SEC-14.
    InSystemRoot,
}

/// SEC-14: o executavel resolvido de um jogo de loja e `.exe` e nao esta em `%SystemRoot%`. A
/// confinamento a pasta de instalacao e de `resolve_confined` (SEC-12).
pub fn check_store_exe(path: &str, policy: &InstallPathPolicy) -> Result<(), StoreExeError> {
    check_launch_exe_path(path).map_err(|_| StoreExeError::Invalid)?;
    if policy.in_system_root(path) {
        return Err(StoreExeError::InSystemRoot);
    }
    Ok(())
}

/// O que `fs_list_dir` deve listar.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BrowseTarget {
    /// Nulo ou vazio: as unidades locais.
    Drives,
    /// Pasta `X:\...` de forma valida (a unidade e conferida pelo adapter).
    Dir(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BrowseError {
    /// Grande demais, controle ou nao absoluto: `InvalidRequest`.
    Malformed,
    /// UNC, `\\?\`, `\\.\`, ADS, `..`: `PathNotAllowed`.
    NotAllowed,
}

/// Valida o caminho pedido a `fs_list_dir` (SEC-09): so `X:\...` local, sem prefixo especial,
/// `:` fora da unidade nem `.`/`..`.
pub fn parse_browse_path(path: Option<&str>) -> Result<BrowseTarget, BrowseError> {
    let Some(path) = path.filter(|p| !p.is_empty()) else {
        return Ok(BrowseTarget::Drives);
    };
    if path.chars().count() > 1024 || path.chars().any(char::is_control) {
        return Err(BrowseError::Malformed);
    }
    if path.starts_with("\\\\") || path.starts_with("//") {
        return Err(BrowseError::NotAllowed);
    }
    let b = path.as_bytes();
    if b.len() < 2 || !b[0].is_ascii_alphabetic() || b[1] != b':' {
        return Err(BrowseError::Malformed);
    }
    // `C:` e `C:\` listam a raiz da unidade; o resto passa pela forma local.
    let normalized = if b.len() == 2 {
        format!("{}:\\", &path[..1])
    } else {
        path.to_owned()
    };
    local_components(&normalized).map_err(|_| BrowseError::NotAllowed)?;
    Ok(BrowseTarget::Dir(normalized))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cadastro_manual_por_tabela() {
        use ManualPathError::*;
        let casos: &[(&str, Result<(), ManualPathError>)] = &[
            (r"C:\Jogos\Foo\foo.exe", Ok(())),
            (r"D:\Jogos\A & B\x.EXE", Ok(())),
            (r"C:\Jogos\atalho.lnk", Err(NotExecutable)),
            (r"C:\Jogos\run.bat", Err(NotExecutable)),
            (r"C:\Jogos\run.cmd", Err(NotExecutable)),
            (r"C:\Jogos\run.ps1", Err(NotExecutable)),
            (r"C:\Jogos\leia.txt", Err(NotExecutable)),
            (r"C:\jogo.exe:ads", Err(NotExecutable)),
            (r"C:\Jogos\NUL.exe", Err(NotExecutable)),
            (r"C:\Jogos\..\Windows\x.exe", Err(NotExecutable)),
            (r"\\127.0.0.1\c$\Jogos\x.exe", Err(NotAllowed)),
            (r"\\servidor\share\x.exe", Err(NotAllowed)),
            (r"\\?\C:\Jogos\x.exe", Err(NotAllowed)),
            (r"\\?\UNC\srv\share\x.exe", Err(NotAllowed)),
            (r"\\.\C:\x.exe", Err(NotAllowed)),
            (r"jogo.exe", Err(Malformed)),
            (r"C:jogo.exe", Err(Malformed)),
            (r"C:/Jogos/x.exe", Err(Malformed)),
            ("", Err(Malformed)),
            ("C:\\Jogos\\x\u{0}.exe", Err(Malformed)),
        ];
        for (path, esperado) in casos {
            assert_eq!(check_manual_exe_path(path), *esperado, "{path}");
        }
        assert_eq!(
            check_manual_exe_path(&format!("C:\\{}.exe", "a".repeat(1100))),
            Err(Malformed)
        );
    }

    #[test]
    fn exe_de_loja_nao_e_script_nem_do_sistema() {
        let policy = InstallPathPolicy::new(r"C:\Windows", Vec::new());
        let casos: &[(&str, Result<(), StoreExeError>)] = &[
            (r"C:\GOG Games\X\bin\game.exe", Ok(())),
            (r"C:\GOG Games\X\start.bat", Err(StoreExeError::Invalid)),
            (r"C:\GOG Games\X\a.lnk", Err(StoreExeError::Invalid)),
            (r"C:\GOG Games\X\a.ps1", Err(StoreExeError::Invalid)),
            (r"C:\GOG Games\X\a.exe:evil", Err(StoreExeError::Invalid)),
            (r"\\srv\share\a.exe", Err(StoreExeError::Invalid)),
            (
                r"C:\Windows\System32\cmd.exe",
                Err(StoreExeError::InSystemRoot),
            ),
            (r"c:\windows\explorer.exe", Err(StoreExeError::InSystemRoot)),
        ];
        for (path, esperado) in casos {
            assert_eq!(check_store_exe(path, &policy), *esperado, "{path}");
        }
    }

    #[test]
    fn navegador_de_pastas_por_tabela() {
        use BrowseError::*;
        let casos: &[(Option<&str>, Result<BrowseTarget, BrowseError>)] = &[
            (None, Ok(BrowseTarget::Drives)),
            (Some(""), Ok(BrowseTarget::Drives)),
            (Some(r"C:\"), Ok(BrowseTarget::Dir(r"C:\".into()))),
            (Some("C:"), Ok(BrowseTarget::Dir(r"C:\".into()))),
            (
                Some(r"D:\Jogos\X"),
                Ok(BrowseTarget::Dir(r"D:\Jogos\X".into())),
            ),
            (Some(r"\\127.0.0.1\c$"), Err(NotAllowed)),
            (Some(r"\\?\C:\Jogos"), Err(NotAllowed)),
            (Some(r"\\.\PhysicalDrive0"), Err(NotAllowed)),
            (Some(r"C:\Jogos\x:ads"), Err(NotAllowed)),
            (Some(r"C:\Jogos\..\Windows"), Err(NotAllowed)),
            (Some("Jogos"), Err(Malformed)),
            (Some("C:\\a\u{7}"), Err(Malformed)),
        ];
        for (path, esperado) in casos {
            assert_eq!(parse_browse_path(*path), *esperado, "{path:?}");
        }
    }
}
