//! `GameId` e `Origin` (contrato `docs/api/launcher-tv.md` secao 1).
//!
//! O ID nunca compoe caminho em disco (CT-06): nem `epic:..`, que a regex aceita, nem nenhum outro.

use std::fmt;

use sha2::{Digest, Sha256};

use super::id_formats;

/// Comprimento maximo do ID inteiro (`epic:` + 128 e o maior caso valido).
const MAX_ID_LEN: usize = 5 + 1 + 128;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Origin {
    Steam,
    Epic,
    Gog,
    Manual,
}

impl Origin {
    pub fn prefix(self) -> &'static str {
        match self {
            Origin::Steam => "steam",
            Origin::Epic => "epic",
            Origin::Gog => "gog",
            Origin::Manual => "manual",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum GameIdError {
    #[error("GameId vazio ou grande demais")]
    Length,
    #[error("GameId sem separador `origem:chave`")]
    Shape,
    #[error("origem de GameId desconhecida")]
    UnknownOrigin,
    #[error("chave de GameId fora do formato da origem")]
    BadKey,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct GameId {
    origin: Origin,
    key: String,
}

impl GameId {
    /// Valida `"<origem>:<chave>"` conforme a tabela do contrato.
    pub fn parse(s: &str) -> Result<Self, GameIdError> {
        if s.is_empty() || s.len() > MAX_ID_LEN {
            return Err(GameIdError::Length);
        }
        let (prefix, key) = s.split_once(':').ok_or(GameIdError::Shape)?;
        let origin = match prefix {
            "steam" => Origin::Steam,
            "epic" => Origin::Epic,
            "gog" => Origin::Gog,
            "manual" => Origin::Manual,
            _ => return Err(GameIdError::UnknownOrigin),
        };
        let ok = match origin {
            Origin::Steam => id_formats::parse_steam_app_id(key).is_some(),
            Origin::Epic => id_formats::is_epic_component(key),
            Origin::Gog => id_formats::is_gog_id(key),
            Origin::Manual => id_formats::is_manual_key(key),
        };
        if ok {
            Ok(Self {
                origin,
                key: key.to_owned(),
            })
        } else {
            Err(GameIdError::BadKey)
        }
    }

    /// ID de jogo manual a partir do caminho **canonico** (CT-05): SHA-256 dos bytes UTF-16LE do
    /// caminho em maiusculas (mapeamento caractere a caractere, como a comparacao ordinal sem
    /// caixa do Windows), primeiros 8 bytes em hexadecimal minusculo.
    pub fn manual_from_canonical_path(canonical_path: &str) -> Self {
        let mut hasher = Sha256::new();
        for c in canonical_path.chars() {
            let mut up = c.to_uppercase();
            let mapped = match (up.next(), up.next()) {
                (Some(u), None) => u, // so mapeamentos 1:1; o eszett fica como esta
                _ => c,
            };
            let mut buf = [0u16; 2];
            for unit in mapped.encode_utf16(&mut buf) {
                hasher.update(unit.to_le_bytes());
            }
        }
        let digest = hasher.finalize();
        let mut key = String::with_capacity(16);
        for b in &digest[..8] {
            key.push_str(&format!("{b:02x}"));
        }
        Self {
            origin: Origin::Manual,
            key,
        }
    }

    pub fn origin(&self) -> Origin {
        self.origin
    }

    pub fn key(&self) -> &str {
        &self.key
    }
}

impl fmt::Display for GameId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.origin.prefix(), self.key)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aceita_exemplos_do_contrato() {
        for s in [
            "steam:620",
            "steam:4294967295",
            "epic:SampleGameA",
            "epic:..", // bem formado; nunca vira caminho (CT-06)
            "gog:1000000001",
            "manual:9f3a1c2b7d4e5a60",
        ] {
            let id = GameId::parse(s).unwrap_or_else(|e| panic!("{s}: {e}"));
            assert_eq!(id.to_string(), s);
        }
    }

    #[test]
    fn rejeita_formas_invalidas() {
        let longo_epic = format!("epic:{}", "a".repeat(129));
        let cases: [(&str, GameIdError); 14] = [
            ("", GameIdError::Length),
            ("steam", GameIdError::Shape),
            ("uplay:1", GameIdError::UnknownOrigin),
            ("Steam:620", GameIdError::UnknownOrigin),
            ("steam:", GameIdError::BadKey),
            ("steam:4294967296", GameIdError::BadKey), // nao cabe em u32
            ("steam:10//-console", GameIdError::BadKey),
            ("steam:620 ", GameIdError::BadKey),
            ("epic:x?action=foo&y", GameIdError::BadKey),
            ("epic:a b", GameIdError::BadKey),
            ("gog:12a", GameIdError::BadKey),
            ("manual:9F3A1C2B7D4E5A60", GameIdError::BadKey), // so minusculas
            ("manual:9f3a1c2b7d4e5a6", GameIdError::BadKey),
            (longo_epic.as_str(), GameIdError::BadKey),
        ];
        for (s, expected) in cases {
            assert_eq!(GameId::parse(s), Err(expected), "entrada: {s:?}");
        }
        assert_eq!(GameId::parse(&"x".repeat(200)), Err(GameIdError::Length));
    }

    #[test]
    fn id_manual_usa_sha256_utf16_em_maiusculas() {
        // Valores calculados fora do codigo (hashlib: sha256(path.upper().encode("utf-16le"))[:8]).
        let a = GameId::manual_from_canonical_path(r"C:\Jogos\Foo\foo.exe");
        assert_eq!(a.to_string(), "manual:bc19217062bdad32");
        let acentuado = GameId::manual_from_canonical_path("D:\\Jogos\\A\u{e7}\u{e3}o\\jogo.exe");
        assert_eq!(acentuado.to_string(), "manual:c23bc1060425f502");
    }

    #[test]
    fn id_manual_ignora_caixa_e_gera_id_valido() {
        let a = GameId::manual_from_canonical_path(r"C:\Jogos\Foo\foo.exe");
        let b = GameId::manual_from_canonical_path(r"c:\JOGOS\foo\FOO.EXE");
        assert_eq!(a, b);
        assert_eq!(GameId::parse(&a.to_string()), Ok(a));
        let outro = GameId::manual_from_canonical_path(r"C:\Jogos\Foo\bar.exe");
        assert_ne!(outro.key(), b.key());
    }
}
