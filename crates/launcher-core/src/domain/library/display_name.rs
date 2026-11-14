//! Nome exibido (SEC-07, CT-03): tudo que chega a UI como nome de jogo, de monitor ou de loja
//! passa por aqui. Remove controle (C0/C1) e override bidi, apara e corta em 256 caracteres.

pub const MAX_DISPLAY_NAME_CHARS: usize = 256;

fn is_blocked(c: char) -> bool {
    c.is_control()
        || matches!(
            c,
            '\u{061C}'           // marca de letra arabe
            | '\u{200E}' | '\u{200F}' // LRM, RLM
            | '\u{2028}' | '\u{2029}' // separadores de linha e de paragrafo
            | '\u{202A}'..='\u{202E}' // embedding e override
            | '\u{2066}'..='\u{2069}' // isolates
        )
}

/// Normaliza um nome vindo de fora (manifesto, nome de arquivo, driver) ou digitado pelo usuario.
/// O resultado pode ser vazio: quem exige nome (renomear) responde `EmptyName`.
pub fn normalize_display_name(raw: &str) -> String {
    let cleaned: String = raw.chars().filter(|c| !is_blocked(*c)).collect();
    let cut: String = cleaned
        .trim()
        .chars()
        .take(MAX_DISPLAY_NAME_CHARS)
        .collect();
    cut.trim_end().to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn remove_controle_bidi_e_apara() {
        let casos: &[(&str, &str)] = &[
            ("Portal 2", "Portal 2"),
            ("  Portal 2\t", "Portal 2"),
            ("a\u{0}b\u{7}c\nd\u{1b}[31m", "abcd[31m"),
            ("x\u{7f}y\u{80}z\u{9f}", "xyz"),
            ("\u{202E}exe.txt", "exe.txt"),
            ("a\u{2066}b\u{2069}c\u{202A}d\u{202C}", "abcd"),
            ("a\u{200E}b\u{200F}c\u{2028}d", "abcd"),
            (
                "<img src=x onerror=alert(1)>",
                "<img src=x onerror=alert(1)>",
            ), // so texto: a UI nao interpreta
            ("A\u{e7}\u{e3}o \u{1F3AE}", "A\u{e7}\u{e3}o \u{1F3AE}"),
            ("\u{202E}\u{0}  ", ""),
            ("", ""),
        ];
        for (entrada, esperado) in casos {
            assert_eq!(normalize_display_name(entrada), *esperado, "{entrada:?}");
        }
    }

    #[test]
    fn corta_em_256_caracteres_e_nao_deixa_espaco_na_ponta() {
        let longo = "a".repeat(5000);
        assert_eq!(normalize_display_name(&longo).chars().count(), 256);
        let com_espaco = format!("{} b", "a".repeat(255));
        let n = normalize_display_name(&com_espaco);
        assert_eq!(n, "a".repeat(255));
        let emoji = "\u{1F3AE}".repeat(300);
        assert_eq!(normalize_display_name(&emoji).chars().count(), 256);
    }
}
