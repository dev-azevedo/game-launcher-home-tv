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

/// Caracteres que nao desenham nada (espaco incluido): se o nome so tem esses, ele e vazio
/// (CA-61, QA-BUG-01, QA-BUG-04). Dentro de um nome com texto visivel eles ficam: o ZWJ, o seletor
/// de variacao e afins compoem emoji e escrita. Cobre os `Default_Ignorable_Code_Point` e os
/// formatos (`Cf`) sem glifo. Ficam de fora, de proposito, os `Cf` que desenham um sinal proprio
/// (sinais numericos arabes `U+0600..0605`, `U+06DD`, `U+0890..0891`, `U+08E2`, `U+070F`,
/// `U+110BD` e `U+110CD`).
fn is_invisible(c: char) -> bool {
    c.is_whitespace()
        || matches!(
            c,
            '\u{00AD}'                    // hifen condicional
            | '\u{034F}'                  // combining grapheme joiner
            | '\u{115F}' | '\u{1160}'     // fillers de hangul
            | '\u{17B4}' | '\u{17B5}'     // vogais inerentes khmer
            | '\u{180B}'..='\u{180F}'     // seletores e separador mongol
            | '\u{200B}'..='\u{200D}'     // ZWSP, ZWNJ, ZWJ
            | '\u{2060}'..='\u{206F}'     // word joiner, operadores, isolates e formatos obsoletos
            | '\u{2800}'                  // braille em branco
            | '\u{3164}'                  // filler de hangul
            | '\u{FE00}'..='\u{FE0F}'     // seletores de variacao
            | '\u{FEFF}'                  // BOM / ZWNBSP
            | '\u{FFA0}'                  // filler de hangul de meia largura
            | '\u{FFF0}'..='\u{FFFB}'     // nao atribuidos ignoraveis e anotacoes interlineares
            | '\u{13430}'..='\u{1343F}'   // controles de formato de hieroglifos egipcios
            | '\u{1BCA0}'..='\u{1BCA3}'   // controles de formato de estenografia
            | '\u{1D173}'..='\u{1D17A}'   // formatos musicais invisiveis
            | '\u{E0000}'..='\u{E0FFF}'   // tags e seletores de variacao suplementares
        )
}

/// Normaliza um nome vindo de fora (manifesto, nome de arquivo, driver) ou digitado pelo usuario.
/// O resultado pode ser vazio (inclusive para nome so de caracteres invisiveis): quem exige nome
/// (renomear) responde `EmptyName`.
pub fn normalize_display_name(raw: &str) -> String {
    let cleaned: String = raw.chars().filter(|c| !is_blocked(*c)).collect();
    let cut: String = cleaned
        .trim()
        .chars()
        .take(MAX_DISPLAY_NAME_CHARS)
        .collect();
    if cut.chars().all(is_invisible) {
        return String::new();
    }
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
    fn nome_so_de_invisiveis_vira_vazio_mas_invisivel_dentro_de_texto_fica() {
        for invisivel in [
            "\u{200B}",
            "\u{200B}\u{200C}\u{200D}",
            "\u{2060}",
            "\u{FEFF}",
            "\u{3164}",
            "\u{2800}",
            "\u{00AD}",
            "\u{115F}",
            " \u{200B}\u{A0}\u{3000} ",
            "\u{FE0F}",
            "\u{E0020}\u{E007F}",
            "\u{180F}",
            "\u{2065}",
            "\u{FFF0}\u{FFF8}",
            "\u{13430}\u{1343F}",
            "\u{1BCA0}\u{1BCA3}",
            "\u{1D173}\u{1D17A}",
        ] {
            assert_eq!(normalize_display_name(invisivel), "", "{invisivel:?}");
        }
        // Emoji composto (ZWJ, VS16) e nome com invisivel no meio continuam como vieram.
        let familia = "\u{1F468}\u{200D}\u{1F469}\u{200D}\u{1F467}";
        assert_eq!(normalize_display_name(familia), familia);
        assert_eq!(
            normalize_display_name("\u{2764}\u{FE0F}"),
            "\u{2764}\u{FE0F}"
        );
        assert_eq!(normalize_display_name("A\u{200B}B"), "A\u{200B}B");
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
