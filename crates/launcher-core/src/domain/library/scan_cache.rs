//! Formato do `cache\scan.json` (arquitetura secao 5, BE-40): o resultado da ultima varredura das
//! lojas, para a biblioteca aparecer antes da proxima varredura. E cache descartavel, nunca
//! verdade: o jogo de loja so inicia a partir da deteccao **atual** (SEC-18), e arquivo invalido
//! e simplesmente ignorado. Leitura com schema fechado e limites.

use serde::{Deserialize, Serialize};

use super::id::{GameId, Origin};
use super::id_formats::is_epic_component;
use super::launch::LaunchSpec;
use super::model::DetectedGame;

pub const SCAN_CACHE_VERSION: u32 = 1;
pub const MAX_CACHED_GAMES: usize = 20_000;
const MAX_STRING_CHARS: usize = 1_024;
const MAX_ARGS: usize = 32;

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum ScanCacheError {
    #[error("JSON invalido ou fora do schema")]
    Schema,
    #[error("versao desconhecida")]
    Version,
    #[error("valor fora dos limites")]
    Limits,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct CacheFile {
    schema_version: u32,
    games: Vec<GameFile>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct GameFile {
    id: String,
    name: String,
    install_dir: Option<String>,
    launch: LaunchFile,
    cover_source: Option<String>,
    icon_source: Option<String>,
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase", deny_unknown_fields)]
enum LaunchFile {
    #[serde(rename_all = "camelCase")]
    SteamUri { app_id: u32 },
    #[serde(rename_all = "camelCase")]
    EpicUri {
        namespace: String,
        catalog_item_id: String,
        app_name: String,
    },
    #[serde(rename_all = "camelCase")]
    Exe {
        path: String,
        args: Vec<String>,
        working_dir: Option<String>,
    },
}

fn ok_str(s: &str) -> bool {
    s.chars().count() <= MAX_STRING_CHARS && !s.contains('\0')
}

fn ok_opt(s: &Option<String>) -> bool {
    match s {
        None => true,
        Some(v) => ok_str(v),
    }
}

pub fn serialize_scan(games: &[DetectedGame]) -> Vec<u8> {
    let file = CacheFile {
        schema_version: SCAN_CACHE_VERSION,
        games: games
            .iter()
            .map(|g| GameFile {
                id: g.id.to_string(),
                name: g.name.clone(),
                install_dir: g.install_dir.clone(),
                launch: match &g.launch {
                    LaunchSpec::SteamUri { app_id } => LaunchFile::SteamUri { app_id: *app_id },
                    LaunchSpec::EpicUri {
                        namespace,
                        catalog_item_id,
                        app_name,
                    } => LaunchFile::EpicUri {
                        namespace: namespace.clone(),
                        catalog_item_id: catalog_item_id.clone(),
                        app_name: app_name.clone(),
                    },
                    LaunchSpec::Exe {
                        path,
                        args,
                        working_dir,
                    } => LaunchFile::Exe {
                        path: path.clone(),
                        args: args.clone(),
                        working_dir: working_dir.clone(),
                    },
                },
                cover_source: g.cover_source.clone(),
                icon_source: g.icon_source.clone(),
            })
            .collect(),
    };
    // Tipos proprios e sem mapas com chave nao textual: a serializacao nao falha.
    serde_json::to_vec(&file).unwrap_or_default()
}

/// Valida forma, limites e coerencia entre o `GameId` e o modo de iniciar (um id Steam so com
/// URI Steam, Epic so com URI Epic do mesmo `AppName`, GOG so com executavel). Qualquer
/// violacao invalida o arquivo inteiro.
pub fn parse_scan(bytes: &[u8]) -> Result<Vec<DetectedGame>, ScanCacheError> {
    let file: CacheFile = serde_json::from_slice(bytes).map_err(|_| ScanCacheError::Schema)?;
    if file.schema_version != SCAN_CACHE_VERSION {
        return Err(ScanCacheError::Version);
    }
    if file.games.len() > MAX_CACHED_GAMES {
        return Err(ScanCacheError::Limits);
    }
    let mut out = Vec::with_capacity(file.games.len());
    for g in file.games {
        let id = GameId::parse(&g.id).map_err(|_| ScanCacheError::Limits)?;
        if !ok_str(&g.name)
            || !ok_opt(&g.install_dir)
            || !ok_opt(&g.cover_source)
            || !ok_opt(&g.icon_source)
        {
            return Err(ScanCacheError::Limits);
        }
        let launch = match (id.origin(), g.launch) {
            (Origin::Steam, LaunchFile::SteamUri { app_id }) if id.key() == app_id.to_string() => {
                LaunchSpec::SteamUri { app_id }
            }
            (
                Origin::Epic,
                LaunchFile::EpicUri {
                    namespace,
                    catalog_item_id,
                    app_name,
                },
            ) if id.key() == app_name
                && is_epic_component(&namespace)
                && is_epic_component(&catalog_item_id) =>
            {
                LaunchSpec::EpicUri {
                    namespace,
                    catalog_item_id,
                    app_name,
                }
            }
            (
                Origin::Gog,
                LaunchFile::Exe {
                    path,
                    args,
                    working_dir,
                },
            ) if ok_str(&path)
                && args.len() <= MAX_ARGS
                && args.iter().all(|a| ok_str(a))
                && ok_opt(&working_dir) =>
            {
                LaunchSpec::Exe {
                    path,
                    args,
                    working_dir,
                }
            }
            _ => return Err(ScanCacheError::Limits),
        };
        out.push(DetectedGame {
            id,
            name: g.name,
            install_dir: g.install_dir,
            launch,
            cover_source: g.cover_source,
            icon_source: g.icon_source,
        });
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn steam() -> DetectedGame {
        DetectedGame {
            id: GameId::parse("steam:620").unwrap(),
            name: "Portal 2".into(),
            install_dir: Some(r"C:\Jogos\Portal 2".into()),
            launch: LaunchSpec::SteamUri { app_id: 620 },
            cover_source: Some(r"C:\Steam\capa.jpg".into()),
            icon_source: None,
        }
    }

    fn epic() -> DetectedGame {
        DetectedGame {
            id: GameId::parse("epic:Sample").unwrap(),
            name: "Sample".into(),
            install_dir: Some(r"C:\Epic\Sample".into()),
            launch: LaunchSpec::EpicUri {
                namespace: "ns".into(),
                catalog_item_id: "item".into(),
                app_name: "Sample".into(),
            },
            cover_source: None,
            icon_source: Some(r"C:\Epic\Sample\s.exe".into()),
        }
    }

    fn gog() -> DetectedGame {
        DetectedGame {
            id: GameId::parse("gog:1000000001").unwrap(),
            name: "G".into(),
            install_dir: Some(r"C:\GOG\G".into()),
            launch: LaunchSpec::Exe {
                path: r"C:\GOG\G\g.exe".into(),
                args: vec!["-a".into(), "b c".into()],
                working_dir: Some(r"C:\GOG\G".into()),
            },
            cover_source: None,
            icon_source: Some(r"C:\GOG\G\g.exe".into()),
        }
    }

    #[test]
    fn ida_e_volta_preserva_as_tres_lojas() {
        let games = vec![steam(), epic(), gog()];
        assert_eq!(parse_scan(&serialize_scan(&games)).unwrap(), games);
        assert_eq!(parse_scan(&serialize_scan(&[])).unwrap(), Vec::new());
    }

    #[test]
    fn arquivo_adulterado_e_recusado_inteiro() {
        let ok = String::from_utf8(serialize_scan(&[steam()])).unwrap();
        let casos: Vec<(String, ScanCacheError)> = vec![
            ("{".into(), ScanCacheError::Schema),
            (
                ok.replace("\"schemaVersion\":1", "\"schemaVersion\":2"),
                ScanCacheError::Version,
            ),
            (
                ok.replace("\"games\"", "\"extra\":1,\"games\""),
                ScanCacheError::Schema,
            ),
            // ID manual nunca vem do cache.
            (
                ok.replace("steam:620", "manual:0123456789abcdef"),
                ScanCacheError::Limits,
            ),
            // ID Steam com URI de outro app: o cache nao troca o que sera iniciado.
            (
                ok.replace("\"appId\":620", "\"appId\":621"),
                ScanCacheError::Limits,
            ),
            // Modo de iniciar de outra loja.
            (
                ok.replace(
                    "\"kind\":\"steamUri\",\"appId\":620",
                    "\"kind\":\"exe\",\"path\":\"C:\\\\x.exe\",\"args\":[],\"workingDir\":null",
                ),
                ScanCacheError::Limits,
            ),
        ];
        for (text, esperado) in casos {
            assert_eq!(parse_scan(text.as_bytes()).unwrap_err(), esperado, "{text}");
        }
        // Epic: o `AppName` da URI precisa ser o do ID; GOG com URI nao vale.
        let e = String::from_utf8(serialize_scan(&[epic()])).unwrap();
        assert!(parse_scan(
            e.replace("\"appName\":\"Sample\"", "\"appName\":\"Outro\"")
                .as_bytes()
        )
        .is_err());
        assert!(parse_scan(
            e.replace("\"namespace\":\"ns\"", "\"namespace\":\"a b\"")
                .as_bytes()
        )
        .is_err());
        let g = String::from_utf8(serialize_scan(&[gog()])).unwrap();
        assert!(parse_scan(g.replace("gog:1000000001", "steam:1").as_bytes()).is_err());
    }

    #[test]
    fn limites_de_tamanho_e_de_texto() {
        let mut g = steam();
        g.name = "a".repeat(2000);
        assert_eq!(
            parse_scan(&serialize_scan(&[g])).unwrap_err(),
            ScanCacheError::Limits
        );
        let mut g = gog();
        if let LaunchSpec::Exe { args, .. } = &mut g.launch {
            *args = vec!["x".into(); 40];
        }
        assert_eq!(
            parse_scan(&serialize_scan(&[g])).unwrap_err(),
            ScanCacheError::Limits
        );
        let mut g = steam();
        g.install_dir = Some("C:\\a\u{0}b".into());
        assert_eq!(
            parse_scan(&serialize_scan(&[g])).unwrap_err(),
            ScanCacheError::Limits
        );
    }
}
