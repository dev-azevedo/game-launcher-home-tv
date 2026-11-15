//! Remocao dos dados do usuario (`sofa.exe --purge-user-data`, BE-41, SEC-25, SEC-31): so as
//! decisoes puras. Qual e o alvo, quando ele e valido e qual codigo de saida cada recusa leva. A
//! remocao por handle e do adapter (`launcher-win/src/purge.rs`).

/// Identificador do app, fixo no build: precisa ser igual ao `identifier` do `tauri.conf.json`
/// (um teste do `src-tauri` confere). Nunca vem de configuracao, registro ou argumento.
pub const APP_IDENTIFIER: &str = "app.sofa.launcher";

/// Arquivos que provam que a pasta e a de dados do Sofa (SEC-25): basta um deles.
pub const MARKER_FILES: [&str; 2] = ["library.json", "settings.json"];

/// Codigos de saida do modo de remocao (arquitetura 2.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PurgeExit {
    /// Tudo removido (ou nao havia nada a remover).
    Done = 0,
    /// Remocao parcial: algo ficou (arquivo em uso, ex.: `EBWebView\` com o WebView2 aberto).
    Partial = 1,
    /// Alvo invalido ou argumento extra: nada foi apagado.
    InvalidTarget = 2,
    /// Rodando elevado: recusa sem apagar nada (SEC-31).
    Elevated = 3,
    /// O Sofa esta aberto (mutex da instancia unica existe): nada e apagado (SEC-BUG-03). O
    /// desinstalador fecha o Sofa e tenta de novo; nunca cai no fallback elevado por causa disto.
    AppOpen = 4,
}

impl PurgeExit {
    pub fn code(self) -> i32 {
        self as i32
    }
}

/// O que se sabe do alvo, lido do disco pelo adapter.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TargetFacts {
    pub exists: bool,
    /// O proprio alvo e ponto de reanalise (junction, link simbolico).
    pub is_reparse_point: bool,
    /// O caminho final do alvo esta exatamente um nivel abaixo de `%LOCALAPPDATA%`.
    pub one_level_below_local_app_data: bool,
    /// Contem `library.json` ou `settings.json` (arquivo regular).
    pub has_marker: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PurgePlan {
    /// A pasta nao existe: nada a fazer, sucesso.
    NothingToDo,
    Delete,
}

/// Identificador aceito: 1 a 64 caracteres entre letras, digitos, `.`, `-` e `_`, sem comecar por
/// ponto e sem `..` (nunca separador, `:` nem nome relativo).
pub fn identifier_is_safe(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 64
        && !id.starts_with('.')
        && !id.contains("..")
        && id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'-' | b'_'))
}

/// `<LocalAppData>\<identifier>`, calculado so aqui. `local_app_data` precisa ser um caminho
/// absoluto de unidade (`C:\...`); UNC, `\\?\` e relativos sao recusados.
pub fn target_dir(local_app_data: &str, identifier: &str) -> Option<String> {
    let lad = local_app_data.trim_end_matches('\\');
    let b = lad.as_bytes();
    let absolute_drive =
        b.len() >= 3 && b[0].is_ascii_alphabetic() && b[1] == b':' && b[2] == b'\\';
    if !absolute_drive || lad.contains("..") || lad.chars().any(char::is_control) {
        return None;
    }
    identifier_is_safe(identifier).then(|| format!("{lad}\\{identifier}"))
}

pub use crate::domain::path::is_direct_child;

/// Decide se apaga (SEC-25). Qualquer duvida e `InvalidTarget` e nada e apagado.
pub fn plan(facts: &TargetFacts) -> Result<PurgePlan, PurgeExit> {
    if !facts.exists {
        return Ok(PurgePlan::NothingToDo);
    }
    if facts.is_reparse_point || !facts.one_level_below_local_app_data || !facts.has_marker {
        return Err(PurgeExit::InvalidTarget);
    }
    Ok(PurgePlan::Delete)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn facts(exists: bool, reparse: bool, level: bool, marker: bool) -> TargetFacts {
        TargetFacts {
            exists,
            is_reparse_point: reparse,
            one_level_below_local_app_data: level,
            has_marker: marker,
        }
    }

    #[test]
    fn alvo_e_o_identificador_sob_o_local_app_data() {
        assert_eq!(
            target_dir(r"C:\Users\ana\AppData\Local", APP_IDENTIFIER).as_deref(),
            Some(r"C:\Users\ana\AppData\Local\app.sofa.launcher")
        );
        assert_eq!(
            target_dir(r"C:\Users\ana\AppData\Local\", APP_IDENTIFIER).as_deref(),
            Some(r"C:\Users\ana\AppData\Local\app.sofa.launcher")
        );
    }

    #[test]
    fn alvo_recusa_local_app_data_estranho_e_identificador_perigoso() {
        for lad in [
            "",
            r"\\servidor\share",
            r"\\?\C:\Users\ana",
            r"Users\ana",
            r"C:Users",
            r"C:\Users\..\Windows",
            "C:\\Users\\a\u{7}na",
        ] {
            assert_eq!(target_dir(lad, APP_IDENTIFIER), None, "{lad:?}");
        }
        for id in [
            "",
            "..",
            ".sofa",
            r"app\sofa",
            "app/sofa",
            r"C:\Windows",
            "a..b",
            "app sofa",
            "app:sofa",
            &"x".repeat(65),
        ] {
            assert_eq!(target_dir(r"C:\Users\ana\AppData\Local", id), None, "{id}");
        }
    }

    #[test]
    fn plano_por_tabela() {
        // (existe, reparse, um nivel, marcador) -> resultado
        let tabela = [
            (
                facts(false, false, false, false),
                Ok(PurgePlan::NothingToDo),
            ),
            (facts(true, false, true, true), Ok(PurgePlan::Delete)),
            (facts(true, true, true, true), Err(PurgeExit::InvalidTarget)),
            (
                facts(true, false, false, true),
                Err(PurgeExit::InvalidTarget),
            ),
            (
                facts(true, false, true, false),
                Err(PurgeExit::InvalidTarget),
            ),
            (
                facts(true, true, false, false),
                Err(PurgeExit::InvalidTarget),
            ),
        ];
        for (f, esperado) in tabela {
            assert_eq!(plan(&f), esperado, "{f:?}");
        }
    }

    #[test]
    fn codigos_de_saida_sao_0_a_4() {
        assert_eq!(PurgeExit::Done.code(), 0);
        assert_eq!(PurgeExit::Partial.code(), 1);
        assert_eq!(PurgeExit::InvalidTarget.code(), 2);
        assert_eq!(PurgeExit::Elevated.code(), 3);
        assert_eq!(PurgeExit::AppOpen.code(), 4);
    }

    #[test]
    fn identificador_fixo_confere_com_o_esperado() {
        assert!(identifier_is_safe(APP_IDENTIFIER));
        assert_eq!(MARKER_FILES, ["library.json", "settings.json"]);
    }
}
