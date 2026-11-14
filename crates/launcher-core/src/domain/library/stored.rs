//! `library.json` (ADR-0004): jogos manuais inteiros + camada do usuario (nome, oculto, ultimo
//! jogado) indexada por `GameId`. Jogos de loja nunca sao persistidos como verdade (SEC-18).
//!
//! Aqui ficam o formato, os limites e a migracao, tudo puro. A revalidacao dos jogos manuais
//! (SEC-09: `.exe`, caminho local, `MZ`/`PE`) acontece na carga da biblioteca (BE-29/BE-30), nao no
//! parse: o parse so garante forma e tamanho.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::{GameId, Origin};

pub const LIBRARY_SCHEMA_VERSION: u32 = 1;
/// Limites do ADR-0004.
pub const MAX_MANUAL_GAMES: usize = 5_000;
pub const MAX_STRING_CHARS: usize = 1_024;
/// Cada jogo de loja pode ter uma entrada de overlay; folga ampla sobre 5.000 + lojas.
pub const MAX_OVERLAY_ENTRIES: usize = 20_000;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ManualGameRecord {
    pub id: GameId,
    pub name: String,
    pub path: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct OverlayEntry {
    pub custom_name: Option<String>,
    pub hidden: bool,
    /// Inicio da ultima sessao, ms desde a epoca Unix.
    pub last_played_at: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct StoredLibrary {
    pub manual_games: Vec<ManualGameRecord>,
    pub overlay: BTreeMap<GameId, OverlayEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum LibraryFileError {
    #[error("JSON invalido ou fora do schema")]
    Schema,
    #[error("schemaVersion ausente ou desconhecido ({0})")]
    Version(u64),
    #[error("limite excedido ou valor invalido")]
    Limits,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct LibraryFile {
    schema_version: u32,
    #[serde(default)]
    manual_games: Vec<ManualGameFile>,
    #[serde(default)]
    overlay: BTreeMap<String, OverlayFile>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ManualGameFile {
    id: String,
    name: String,
    path: String,
}

#[derive(Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct OverlayFile {
    #[serde(default)]
    custom_name: Option<String>,
    #[serde(default)]
    hidden: bool,
    #[serde(default)]
    last_played_at: Option<i64>,
}

fn migrate(value: Value) -> Result<Value, LibraryFileError> {
    let version = value
        .get("schemaVersion")
        .and_then(Value::as_u64)
        .ok_or(LibraryFileError::Version(0))?;
    match version {
        1 => Ok(value),
        other => Err(LibraryFileError::Version(other)),
    }
}

fn within_limit(s: &str) -> bool {
    s.chars().count() <= MAX_STRING_CHARS && !s.contains('\0')
}

pub fn parse_library(bytes: &[u8]) -> Result<StoredLibrary, LibraryFileError> {
    let value: Value = serde_json::from_slice(bytes).map_err(|_| LibraryFileError::Schema)?;
    let value = migrate(value)?;
    let file: LibraryFile = serde_json::from_value(value).map_err(|_| LibraryFileError::Schema)?;

    if file.manual_games.len() > MAX_MANUAL_GAMES || file.overlay.len() > MAX_OVERLAY_ENTRIES {
        return Err(LibraryFileError::Limits);
    }

    let mut seen = BTreeSet::new();
    let mut manual_games = Vec::with_capacity(file.manual_games.len());
    for g in file.manual_games {
        let id = GameId::parse(&g.id).map_err(|_| LibraryFileError::Limits)?;
        if id.origin() != Origin::Manual
            || !seen.insert(id.clone())
            || g.name.is_empty()
            || g.path.is_empty()
            || !within_limit(&g.name)
            || !within_limit(&g.path)
        {
            return Err(LibraryFileError::Limits);
        }
        manual_games.push(ManualGameRecord {
            id,
            name: g.name,
            path: g.path,
        });
    }

    let mut overlay = BTreeMap::new();
    for (key, o) in file.overlay {
        let id = GameId::parse(&key).map_err(|_| LibraryFileError::Limits)?;
        if let Some(name) = &o.custom_name {
            if !within_limit(name) {
                return Err(LibraryFileError::Limits);
            }
        }
        if o.last_played_at.is_some_and(|t| t < 0) {
            return Err(LibraryFileError::Limits);
        }
        overlay.insert(
            id,
            OverlayEntry {
                custom_name: o.custom_name,
                hidden: o.hidden,
                last_played_at: o.last_played_at,
            },
        );
    }
    Ok(StoredLibrary {
        manual_games,
        overlay,
    })
}

pub fn serialize_library(lib: &StoredLibrary) -> Vec<u8> {
    let file = LibraryFile {
        schema_version: LIBRARY_SCHEMA_VERSION,
        manual_games: lib
            .manual_games
            .iter()
            .map(|g| ManualGameFile {
                id: g.id.to_string(),
                name: g.name.clone(),
                path: g.path.clone(),
            })
            .collect(),
        overlay: lib
            .overlay
            .iter()
            .map(|(id, o)| {
                (
                    id.to_string(),
                    OverlayFile {
                        custom_name: o.custom_name.clone(),
                        hidden: o.hidden,
                        last_played_at: o.last_played_at,
                    },
                )
            })
            .collect(),
    };
    serde_json::to_vec_pretty(&file).expect("biblioteca serializavel")
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn parse(v: Value) -> Result<StoredLibrary, LibraryFileError> {
        parse_library(v.to_string().as_bytes())
    }

    fn manual(id: &str) -> Value {
        json!({ "id": id, "name": "Meu Jogo", "path": "C:\\Jogos\\a.exe" })
    }

    #[test]
    fn arquivo_minimo_e_biblioteca_vazia() {
        assert_eq!(
            parse(json!({ "schemaVersion": 1 })).unwrap(),
            StoredLibrary::default()
        );
    }

    #[test]
    fn le_manuais_e_overlay() {
        let lib = parse(json!({
            "schemaVersion": 1,
            "manualGames": [manual("manual:9f3a1c2b7d4e5a60")],
            "overlay": {
                "steam:620": { "customName": "Portal 2 (coop)", "hidden": true, "lastPlayedAt": 1794610000000_i64 },
                "epic:SampleGameA": {}
            }
        }))
        .unwrap();
        assert_eq!(lib.manual_games.len(), 1);
        assert_eq!(
            lib.manual_games[0].id.to_string(),
            "manual:9f3a1c2b7d4e5a60"
        );
        let portal = &lib.overlay[&GameId::parse("steam:620").unwrap()];
        assert_eq!(portal.custom_name.as_deref(), Some("Portal 2 (coop)"));
        assert!(portal.hidden);
        assert_eq!(portal.last_played_at, Some(1_794_610_000_000));
        assert_eq!(
            lib.overlay[&GameId::parse("epic:SampleGameA").unwrap()],
            OverlayEntry::default()
        );
    }

    #[test]
    fn campo_desconhecido_leva_o_arquivo_para_backup() {
        // `launch.args` injetado (ameaca de SEC-18) e qualquer outro campo extra.
        for raw in [
            json!({ "schemaVersion": 1, "extra": 1 }),
            json!({ "schemaVersion": 1, "manualGames": [{ "id": "manual:9f3a1c2b7d4e5a60", "name": "x", "path": "C:\\a.exe", "args": "-evil" }] }),
            json!({ "schemaVersion": 1, "manualGames": [{ "id": "manual:9f3a1c2b7d4e5a60", "name": "x", "path": "C:\\a.exe", "launch": { "args": "-x" } }] }),
            json!({ "schemaVersion": 1, "overlay": { "steam:620": { "path": "C:\\x.exe" } } }),
            json!({ "schemaVersion": 1, "manualGames": "nao e lista" }),
        ] {
            assert_eq!(parse(raw.clone()), Err(LibraryFileError::Schema), "{raw}");
        }
        assert_eq!(parse_library(b"nao e json"), Err(LibraryFileError::Schema));
    }

    #[test]
    fn versao_ausente_ou_desconhecida() {
        assert_eq!(parse(json!({})), Err(LibraryFileError::Version(0)));
        assert_eq!(
            parse(json!({ "schemaVersion": 99 })),
            Err(LibraryFileError::Version(99))
        );
    }

    #[test]
    fn limites_de_quantidade() {
        let ok: Vec<Value> = (0..MAX_MANUAL_GAMES)
            .map(|i| manual(&format!("manual:{i:016x}")))
            .collect();
        assert!(parse(json!({ "schemaVersion": 1, "manualGames": ok })).is_ok());
        let too_many: Vec<Value> = (0..=MAX_MANUAL_GAMES)
            .map(|i| manual(&format!("manual:{i:016x}")))
            .collect();
        assert_eq!(
            parse(json!({ "schemaVersion": 1, "manualGames": too_many })),
            Err(LibraryFileError::Limits)
        );
    }

    #[test]
    fn limites_e_formato_de_strings_e_ids() {
        let longa = "a".repeat(1025);
        let casos = vec![
            json!({ "schemaVersion": 1, "manualGames": [{ "id": "manual:9f3a1c2b7d4e5a60", "name": longa, "path": "C:\\a.exe" }] }),
            json!({ "schemaVersion": 1, "manualGames": [{ "id": "manual:9f3a1c2b7d4e5a60", "name": "x", "path": longa }] }),
            json!({ "schemaVersion": 1, "manualGames": [{ "id": "manual:9f3a1c2b7d4e5a60", "name": "", "path": "C:\\a.exe" }] }),
            json!({ "schemaVersion": 1, "manualGames": [{ "id": "manual:9f3a1c2b7d4e5a60", "name": "x", "path": "" }] }),
            json!({ "schemaVersion": 1, "manualGames": [{ "id": "manual:9f3a1c2b7d4e5a60", "name": "x", "path": "C:\\a\u{0}.exe" }] }),
            // jogo manual com id de outra origem
            json!({ "schemaVersion": 1, "manualGames": [{ "id": "steam:620", "name": "x", "path": "C:\\a.exe" }] }),
            // id malformado
            json!({ "schemaVersion": 1, "manualGames": [{ "id": "manual:XYZ", "name": "x", "path": "C:\\a.exe" }] }),
            json!({ "schemaVersion": 1, "overlay": { "steam:10//-console": {} } }),
            json!({ "schemaVersion": 1, "overlay": { "steam:620": { "customName": longa } } }),
            json!({ "schemaVersion": 1, "overlay": { "steam:620": { "lastPlayedAt": -1 } } }),
        ];
        for raw in casos {
            assert_eq!(parse(raw.clone()), Err(LibraryFileError::Limits), "{raw}");
        }
        // 1.024 caracteres exatos passam.
        let exata = "b".repeat(1024);
        assert!(parse(
            json!({ "schemaVersion": 1, "overlay": { "steam:620": { "customName": exata } } })
        )
        .is_ok());
    }

    #[test]
    fn id_manual_duplicado_e_recusado() {
        let raw = json!({ "schemaVersion": 1, "manualGames": [manual("manual:9f3a1c2b7d4e5a60"), manual("manual:9f3a1c2b7d4e5a60")] });
        assert_eq!(parse(raw), Err(LibraryFileError::Limits));
    }

    #[test]
    fn ida_e_volta_preserva_tudo() {
        let mut lib = StoredLibrary::default();
        lib.manual_games.push(ManualGameRecord {
            id: GameId::parse("manual:9f3a1c2b7d4e5a60").unwrap(),
            name: "Ação — São João".into(),
            path: "D:\\Jogos\\Ação\\jogo.exe".into(),
        });
        lib.overlay.insert(
            GameId::parse("gog:1000000001").unwrap(),
            OverlayEntry {
                custom_name: Some("Nome".into()),
                hidden: true,
                last_played_at: Some(5),
            },
        );
        let back = parse_library(&serialize_library(&lib)).unwrap();
        assert_eq!(back, lib);
    }
}
