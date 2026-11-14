//! Formato da chave de `GameId` por origem (contrato secao 1, SEC-11). Fonte unica: o
//! `GameId::parse` e os detectores usam estas funcoes. Nada e "sanitizado": fora do formato, o
//! jogo e ignorado.

/// `appid` da Steam: `^[0-9]{1,10}$` e cabe em `u32`.
pub fn parse_steam_app_id(s: &str) -> Option<u32> {
    if (1..=10).contains(&s.len()) && s.bytes().all(|b| b.is_ascii_digit()) {
        s.parse::<u32>().ok()
    } else {
        None
    }
}

/// `AppName`, `CatalogNamespace` e `CatalogItemId` da Epic: `^[A-Za-z0-9_.-]{1,128}$`.
pub fn is_epic_component(s: &str) -> bool {
    (1..=128).contains(&s.len())
        && s.bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'.' | b'-'))
}

/// `gameID` do GOG: `^[0-9]{1,20}$`.
pub fn is_gog_id(s: &str) -> bool {
    (1..=20).contains(&s.len()) && s.bytes().all(|b| b.is_ascii_digit())
}

/// Chave de jogo manual: 16 hexadecimais minusculos.
pub fn is_manual_key(s: &str) -> bool {
    s.len() == 16 && s.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn steam_por_tabela() {
        let casos: &[(&str, Option<u32>)] = &[
            ("620", Some(620)),
            ("4294967295", Some(u32::MAX)),
            ("4294967296", None),
            ("99999999999", None),
            ("", None),
            ("10//-console", None),
            ("x?action=foo&y", None),
            ("a b", None),
            ("620 ", None),
            ("-1", None),
            ("\u{661}", None), // digito arabe nao e ASCII
        ];
        for (s, esperado) in casos {
            assert_eq!(parse_steam_app_id(s), *esperado, "{s:?}");
        }
    }

    #[test]
    fn epic_gog_e_manual_por_tabela() {
        assert!(is_epic_component("SampleGameA"));
        assert!(is_epic_component(".."));
        assert!(!is_epic_component(""));
        assert!(!is_epic_component("x?action=foo&y"));
        assert!(!is_epic_component("a b"));
        assert!(!is_epic_component("a/b"));
        assert!(!is_epic_component(&"a".repeat(129)));
        assert!(is_epic_component(&"a".repeat(128)));
        assert!(is_gog_id("1000000001"));
        assert!(!is_gog_id("abc"));
        assert!(!is_gog_id(""));
        assert!(!is_gog_id(&"1".repeat(21)));
        assert!(is_manual_key("9f3a1c2b7d4e5a60"));
        assert!(!is_manual_key("9F3A1C2B7D4E5A60"));
    }
}
