//! Como um jogo e iniciado (BE-16): `LaunchSpec` montado so pelo nucleo (SEC-03), templates
//! fixos das lojas (SEC-11) e a checagem do caminho do executavel no momento do lancamento
//! (CA-71, SEC-09). Puro: a existencia do arquivo e o tipo da unidade sao do adapter.

use super::id::GameId;

/// O que o `GameStarter` executa. Nunca vem da UI.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LaunchSpec {
    /// `steam://rungameid/<appid>` (template fixo, SEC-11).
    SteamUri { app_id: u32 },
    /// URI da Epic com os tres componentes ja validados (`^[A-Za-z0-9_.-]{1,128}$`).
    EpicUri {
        namespace: String,
        catalog_item_id: String,
        app_name: String,
    },
    /// Executavel direto (manual e GOG): `args` como lista, nunca texto de shell (SEC-10).
    Exe {
        path: String,
        args: Vec<String>,
        working_dir: Option<String>,
    },
}

/// Jogo resolvido pelo nucleo para iniciar: o `LaunchSpec` mais o que a sonda precisa.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LaunchTarget {
    pub game: GameId,
    pub spec: LaunchSpec,
    /// Pasta de instalacao canonica (SEC-16): base da varredura da sonda (lojas e jogo elevado).
    pub install_dir: Option<String>,
}

/// `steam://rungameid/<appid>`.
pub fn steam_uri(app_id: u32) -> String {
    format!("steam://rungameid/{app_id}")
}

/// `com.epicgames.launcher://apps/<ns>%3A<item>%3A<app>?action=launch&silent=true`, cada
/// componente com percent-encoding (SEC-11).
pub fn epic_uri(namespace: &str, catalog_item_id: &str, app_name: &str) -> String {
    format!(
        "com.epicgames.launcher://apps/{}%3A{}%3A{}?action=launch&silent=true",
        percent_encode(namespace),
        percent_encode(catalog_item_id),
        percent_encode(app_name)
    )
}

/// Tudo que nao for `A-Za-z0-9-._~` vira `%XX` (bytes UTF-8).
pub fn percent_encode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        if b.is_ascii_alphanumeric() || matches!(b, b'-' | b'.' | b'_' | b'~') {
            out.push(char::from(b));
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

/// Por que o caminho do executavel nao pode ser iniciado.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LaunchPathError {
    /// Caminho de rede (UNC, `\\?\UNC\`): `NetworkPathBlocked` (CA-71, 2o momento).
    Network,
    /// Forma nao aceita: prefixo especial, relativo, ADS, `..`, nome reservado, nao `.exe`.
    Invalid,
}

const RESERVED: &[&str] = &[
    "con", "prn", "aux", "nul", "com1", "com2", "com3", "com4", "com5", "com6", "com7", "com8",
    "com9", "lpt1", "lpt2", "lpt3", "lpt4", "lpt5", "lpt6", "lpt7", "lpt8", "lpt9",
];

/// Checagem de forma do caminho do `.exe` no lancamento. Aceita so `X:\...\nome.exe`
/// canonico, sem ADS nem `.`/`..` nem nome reservado. Unidade mapeada de rede
/// (`DRIVE_REMOTE`) e conferida pelo adapter, que tambem responde `NetworkPathBlocked`.
pub fn check_launch_exe_path(path: &str) -> Result<(), LaunchPathError> {
    const SEP: char = '\\';
    if path.is_empty() || path.chars().count() > 1024 || path.chars().any(char::is_control) {
        return Err(LaunchPathError::Invalid);
    }
    let lower = path.to_ascii_lowercase();
    if lower.starts_with("\\\\?\\unc\\") {
        return Err(LaunchPathError::Network);
    }
    if path.starts_with("\\\\") || path.starts_with("//") {
        // `\\?\X:\` e `\\.\` sao prefixos especiais; o resto e `\\servidor\recurso`.
        return Err(
            if path.starts_with("\\\\?\\") || path.starts_with("\\\\.\\") {
                LaunchPathError::Invalid
            } else {
                LaunchPathError::Network
            },
        );
    }
    let b = path.as_bytes();
    if b.len() < 4 || !b[0].is_ascii_alphabetic() || b[1] != b':' || b[2] != b'\\' {
        return Err(LaunchPathError::Invalid);
    }
    if path[2..].contains(':') || path.contains('/') {
        return Err(LaunchPathError::Invalid);
    }
    if !lower.ends_with(".exe") {
        return Err(LaunchPathError::Invalid);
    }
    for comp in path[3..].split(SEP) {
        let stem = comp.split('.').next().unwrap_or("").to_ascii_lowercase();
        if comp.is_empty()
            || comp == "."
            || comp == ".."
            || comp.ends_with('.')
            || comp.ends_with(' ')
            || RESERVED.contains(&stem.as_str())
            || comp.contains(['<', '>', '"', '|', '?', '*'])
        {
            return Err(LaunchPathError::Invalid);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn steam_usa_o_template_fixo() {
        assert_eq!(steam_uri(620), "steam://rungameid/620");
    }

    #[test]
    fn epic_codifica_cada_componente() {
        assert_eq!(
            epic_uri("ns", "item", "SampleGameA"),
            "com.epicgames.launcher://apps/ns%3Aitem%3ASampleGameA?action=launch&silent=true"
        );
        // Mesmo um valor que o formato recusaria nao escapa do componente.
        let uri = epic_uri("a?b", "c&d", "e f/g");
        assert!(
            uri.contains("a%3Fb%3Ac%26d%3Ae%20f%2Fg?action=launch&silent=true"),
            "{uri}"
        );
        assert_eq!(uri.matches('?').count(), 1);
        assert_eq!(uri.matches('&').count(), 1);
    }

    #[test]
    fn caminho_de_lancamento_por_tabela() {
        use LaunchPathError::{Invalid, Network};
        let casos: &[(&str, Result<(), LaunchPathError>)] = &[
            (r"C:\Jogos\Jogo A & B\game.exe", Ok(())),
            (r"D:\x\y.EXE", Ok(())),
            (r"\\servidor\share\game.exe", Err(Network)),
            (r"\\?\UNC\servidor\share\game.exe", Err(Network)),
            (r"\\127.0.0.1\c$\game.exe", Err(Network)),
            (r"\\?\C:\Jogos\game.exe", Err(Invalid)),
            (r"\\.\C:\game.exe", Err(Invalid)),
            (r"jogo.exe", Err(Invalid)),
            (r"C:jogo.exe", Err(Invalid)),
            (r"C:\Jogos\script.bat", Err(Invalid)),
            (r"C:\Jogos\atalho.lnk", Err(Invalid)),
            (r"C:\jogo.exe:ads", Err(Invalid)),
            (r"C:\Jogos\..\Windows\x.exe", Err(Invalid)),
            (r"C:\Jogos\NUL.exe", Err(Invalid)),
            (r"C:\Jogos\com1\x.exe", Err(Invalid)),
            (r"C:\Jogos\a?b.exe", Err(Invalid)),
            (r"C:\Jogos/x.exe", Err(Invalid)),
            ("", Err(Invalid)),
        ];
        for (path, esperado) in casos {
            assert_eq!(check_launch_exe_path(path), *esperado, "{path}");
        }
    }
}
