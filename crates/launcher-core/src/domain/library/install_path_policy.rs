//! Pasta de instalacao valida (SEC-12, SEC-16): forma local, profundidade minima, pastas proibidas
//! e confinamento por componente. Puro: o adapter entrega caminhos ja canonicos (links
//! resolvidos) e o ambiente (`%SystemRoot%`, `%ProgramFiles%`, pasta da loja, pasta do Sofa).
//!
//! Nada aqui e "sanitizado": caminho fora da regra e recusado e o jogo e ignorado na deteccao.

use crate::domain::session::path_is_within;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PathPolicyError {
    /// Nao e `X:\...` local: relativo, UNC, `\\?\`, `\\.\`, ADS, `.`/`..`, controle, tamanho.
    NotLocalAbsolute,
    /// Menos de 2 componentes abaixo da raiz da unidade (`C:\`, `C:\Jogos`).
    TooShallow,
    /// E uma pasta proibida ou contem uma (raiz de sistema, perfil, loja, Sofa).
    ForbiddenFolder,
    /// Dentro de `%SystemRoot%`.
    InsideSystemRoot,
    /// Caminho que escapa da base (`..`, absoluto fora dela, `:`) ou vazio.
    EscapesBase,
}

const MAX_PATH_CHARS: usize = 1024;

fn bad_component(c: &str) -> bool {
    c == "."
        || c == ".."
        || c.ends_with('.')
        || c.ends_with(' ')
        || c.contains([':', '<', '>', '"', '|', '?', '*'])
}

/// Componentes de um caminho `X:\a\b` (sem a unidade) se a forma e local e absoluta. Aceita `/`
/// como separador e ignora barras duplicadas e a final; recusa `.`/`..`, `:` fora da unidade,
/// caractere invalido do Windows e componente terminado em ponto ou espaco.
pub fn local_components(path: &str) -> Result<Vec<&str>, PathPolicyError> {
    if path.is_empty()
        || path.chars().count() > MAX_PATH_CHARS
        || path.chars().any(char::is_control)
    {
        return Err(PathPolicyError::NotLocalAbsolute);
    }
    let b = path.as_bytes();
    if b.len() < 3 || !b[0].is_ascii_alphabetic() || b[1] != b':' || !matches!(b[2], b'\\' | b'/') {
        return Err(PathPolicyError::NotLocalAbsolute);
    }
    let mut out = Vec::new();
    for c in path[3..].split(['\\', '/']).filter(|c| !c.is_empty()) {
        if bad_component(c) {
            return Err(PathPolicyError::NotLocalAbsolute);
        }
        out.push(c);
    }
    Ok(out)
}

/// Forma canonica de texto: `X:\a\b` (unidade em maiuscula, `\`, sem barra final).
pub fn normalize_local(path: &str) -> Result<String, PathPolicyError> {
    let comps = local_components(path)?;
    let drive = path[..1].to_ascii_uppercase();
    if comps.is_empty() {
        return Ok(format!("{drive}:\\"));
    }
    Ok(format!("{drive}:\\{}", comps.join("\\")))
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstallPathPolicy {
    system_root: String,
    forbidden: Vec<String>,
}

impl InstallPathPolicy {
    /// `forbidden`: `%ProgramFiles%`, `%ProgramFiles(x86)%`, `%ProgramData%`, `%USERPROFILE%`,
    /// `%LOCALAPPDATA%`, `%APPDATA%`, pastas das lojas e do Sofa. Entrada que nao e caminho local
    /// valido e descartada (nao ha o que proteger com ela).
    pub fn new(system_root: &str, forbidden: impl IntoIterator<Item = String>) -> Self {
        Self {
            system_root: normalize_local(system_root).unwrap_or_default(),
            forbidden: forbidden
                .into_iter()
                .filter_map(|p| normalize_local(&p).ok())
                .collect(),
        }
    }

    /// Uma copia com mais pastas proibidas (a pasta de uma loja so e conhecida na varredura).
    pub fn with_forbidden(&self, extra: impl IntoIterator<Item = String>) -> Self {
        let mut forbidden = self.forbidden.clone();
        forbidden.extend(extra.into_iter().filter_map(|p| normalize_local(&p).ok()));
        Self {
            system_root: self.system_root.clone(),
            forbidden,
        }
    }

    pub fn system_root(&self) -> &str {
        &self.system_root
    }

    /// SEC-16: a pasta usada para associar processos (RN-10) e para confinar executaveis.
    /// Devolve a forma canonica de texto.
    pub fn check_install_dir(&self, dir: &str) -> Result<String, PathPolicyError> {
        let comps = local_components(dir)?;
        let dir = normalize_local(dir)?;
        if !self.system_root.is_empty() && path_is_within(&self.system_root, &dir) {
            return Err(PathPolicyError::InsideSystemRoot);
        }
        // "Nem conter": a pasta nao pode ser nem ancestral de uma pasta proibida (`C:\Program
        // Files` e proibida; `C:\Program Files\Epic Games\X` esta dentro dela e e valida).
        if self.forbidden.iter().any(|f| path_is_within(&dir, f)) {
            return Err(PathPolicyError::ForbiddenFolder);
        }
        if comps.len() < 2 {
            return Err(PathPolicyError::TooShallow);
        }
        Ok(dir)
    }

    /// `true` se o caminho esta em `%SystemRoot%` (SEC-14).
    pub fn in_system_root(&self, path: &str) -> bool {
        !self.system_root.is_empty() && path_is_within(&self.system_root, path)
    }
}

/// SEC-12: resolve `rel` contra `base` e exige que o resultado fique dentro da base, por
/// componente (`C:\Jogos2` nao esta dentro de `C:\Jogos`). `rel` pode ser relativo (`bin\a.exe`)
/// ou absoluto ja dentro da base. `..`, `.`, `:` e unidade diferente sao recusados, nao resolvidos.
pub fn resolve_confined(base: &str, rel: &str) -> Result<String, PathPolicyError> {
    let base = normalize_local(base).map_err(|_| PathPolicyError::EscapesBase)?;
    if rel.is_empty() || rel.chars().count() > MAX_PATH_CHARS || rel.chars().any(char::is_control) {
        return Err(PathPolicyError::EscapesBase);
    }
    let rb = rel.as_bytes();
    if rel.starts_with(['\\', '/']) || (rb.len() >= 2 && rb[1] == b':') {
        // Absoluto (ou com unidade): so vale se for local e ja estiver dentro da base.
        let abs = normalize_local(rel).map_err(|_| PathPolicyError::EscapesBase)?;
        return if abs.len() > base.len() && path_is_within(&base, &abs) {
            Ok(abs)
        } else {
            Err(PathPolicyError::EscapesBase)
        };
    }
    let mut parts = Vec::new();
    for c in rel.split(['\\', '/']).filter(|c| !c.is_empty()) {
        if bad_component(c) {
            return Err(PathPolicyError::EscapesBase);
        }
        parts.push(c);
    }
    if parts.is_empty() {
        return Err(PathPolicyError::EscapesBase);
    }
    let joined = format!("{base}\\{}", parts.join("\\"));
    if path_is_within(&base, &joined) {
        Ok(joined)
    } else {
        Err(PathPolicyError::EscapesBase)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn policy() -> InstallPathPolicy {
        InstallPathPolicy::new(
            r"C:\Windows",
            [
                r"C:\Program Files".to_owned(),
                r"C:\Program Files (x86)".to_owned(),
                r"C:\ProgramData".to_owned(),
                r"C:\Users\ana".to_owned(),
                r"C:\Users\ana\AppData\Local".to_owned(),
                r"C:\Program Files (x86)\Steam".to_owned(),
                r"C:\Program Files\Sofa".to_owned(),
            ],
        )
    }

    #[test]
    fn pasta_de_instalacao_por_tabela() {
        use PathPolicyError::*;
        let casos: &[(&str, Result<&str, PathPolicyError>)] = &[
            (r"C:\Jogos\X", Ok(r"C:\Jogos\X")),
            (r"c:/Jogos/X/", Ok(r"C:\Jogos\X")),
            (
                r"D:\SteamLibrary\steamapps\common\Portal 2",
                Ok(r"D:\SteamLibrary\steamapps\common\Portal 2"),
            ),
            (
                r"C:\Program Files\Epic Games\SampleGameA",
                Ok(r"C:\Program Files\Epic Games\SampleGameA"),
            ),
            (
                r"C:\Program Files (x86)\Steam\steamapps\common\X",
                Ok(r"C:\Program Files (x86)\Steam\steamapps\common\X"),
            ),
            (r"C:\", Err(ForbiddenFolder)), // contem as pastas proibidas
            (r"C:", Err(NotLocalAbsolute)),
            (r"C:\Jogos", Err(TooShallow)),
            (r"C:\Windows", Err(InsideSystemRoot)),
            (r"C:\Windows\System32", Err(InsideSystemRoot)),
            (r"c:\WINDOWS\Temp\x", Err(InsideSystemRoot)),
            (r"C:\Program Files", Err(ForbiddenFolder)),
            (r"C:\Program Files (x86)\Steam", Err(ForbiddenFolder)),
            (r"C:\Program Files\Sofa", Err(ForbiddenFolder)),
            (r"C:\Users\ana", Err(ForbiddenFolder)),
            (r"C:\Users\ana\AppData\Local", Err(ForbiddenFolder)),
            (r"C:\Users", Err(ForbiddenFolder)),
            (r"\\127.0.0.1\c$\Jogo\X", Err(NotLocalAbsolute)),
            (r"\\?\C:\Jogos\X", Err(NotLocalAbsolute)),
            (r"C:\Jogos\..\Windows", Err(NotLocalAbsolute)),
            (r"C:\Jogos\X:ads", Err(NotLocalAbsolute)),
            (r"Jogos\X", Err(NotLocalAbsolute)),
            ("", Err(NotLocalAbsolute)),
        ];
        for (dir, esperado) in casos {
            assert_eq!(
                policy().check_install_dir(dir),
                esperado.map(str::to_owned),
                "{dir}"
            );
        }
    }

    #[test]
    fn prefixo_de_texto_nao_e_confinamento() {
        assert_eq!(
            resolve_confined(r"C:\Jogos\X", r"bin\game.exe").as_deref(),
            Ok(r"C:\Jogos\X\bin\game.exe")
        );
        assert_eq!(
            resolve_confined(r"C:\Jogos\X", r"C:\Jogos\X\bin\game.exe").as_deref(),
            Ok(r"C:\Jogos\X\bin\game.exe")
        );
        assert_eq!(
            resolve_confined(r"C:\Jogos\X", r"bin/game.exe").as_deref(),
            Ok(r"C:\Jogos\X\bin\game.exe")
        );
        let fora: &[&str] = &[
            r"C:\Jogos\X2\game.exe", // `C:\Jogos\X2` nao esta dentro de `C:\Jogos\X`
            r"..\..\Windows\System32\cmd.exe",
            r"bin\..\..\y.exe",
            r"C:\Windows\System32\cmd.exe",
            r"D:\Jogos\X\game.exe",
            r"\\srv\share\game.exe",
            r"\Windows\x.exe",
            r"bin\a:b.exe",
            r"game.exe:ads",
            r"C:\Jogos\X", // a propria base nao e um arquivo dentro dela
            "",
            ".",
        ];
        for rel in fora {
            assert_eq!(
                resolve_confined(r"C:\Jogos\X", rel),
                Err(PathPolicyError::EscapesBase),
                "{rel}"
            );
        }
    }
}
