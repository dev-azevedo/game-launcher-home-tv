//! Detector do GOG (BE-27, arquitetura 2.2, SP-08): `HKLM\SOFTWARE\WOW6432Node\GOG.com\Games\<id>`
//! e `goggame-<id>.info` na pasta do jogo (`playTask` primario). **Sem SQLite do Galaxy** (AF-06).
//! Sem `.info`, valem `exe`/`workingDir` do registro, pelas mesmas politicas. Um `.info` que existe
//! mas e invalido derruba o jogo: nao cai no registro (seria contornar a tarefa primaria).
//!
//! Risco aberto E-17: nao confirmado se o Galaxy 2.0 sempre cria a chave (QA-17, QF-04).

use std::path::Path;
use std::sync::Arc;

use launcher_core::application::ports::{ScanError, StoreScanner};
use launcher_core::domain::library::id_formats::is_gog_id;
use launcher_core::domain::library::{
    gog_game, local_components, DetectedGame, GogGameRaw, GogInfoRaw, GogTaskRaw, Origin,
};
use serde::Deserialize;
use serde_json::Value;

use super::registry::{Hive, RegistryReader, MAX_SUBKEYS};
use super::{is_plain_file, log_reject, read_limited, StoreEnv};

const GAMES_KEY: &str = r"SOFTWARE\WOW6432Node\GOG.com\Games";

#[derive(Deserialize, Default)]
struct TaskJson {
    category: Option<String>,
    #[serde(rename = "isPrimary")]
    is_primary: Option<bool>,
    #[serde(rename = "type")]
    kind: Option<String>,
    path: Option<String>,
    arguments: Option<String>,
    #[serde(rename = "workingDir")]
    working_dir: Option<String>,
}

#[derive(Deserialize, Default)]
struct InfoJson {
    #[serde(rename = "gameId")]
    game_id: Option<Value>,
    name: Option<String>,
    #[serde(rename = "playTasks", default)]
    play_tasks: Vec<TaskJson>,
}

/// Bytes de um `goggame-<id>.info` -> campos crus. JSON invalido ou com tipo errado = `Err`.
pub fn parse_info(bytes: &[u8]) -> Result<GogInfoRaw, &'static str> {
    let bytes = bytes.strip_prefix(&[0xEF, 0xBB, 0xBF][..]).unwrap_or(bytes);
    let info: InfoJson = serde_json::from_slice(bytes).map_err(|_| ".info invalido")?;
    let game_id = match info.game_id {
        Some(Value::String(s)) => Some(s),
        Some(Value::Number(n)) => Some(n.to_string()),
        _ => None,
    };
    Ok(GogInfoRaw {
        game_id,
        name: info.name,
        play_tasks: info
            .play_tasks
            .into_iter()
            .map(|t| GogTaskRaw {
                category: t.category,
                is_primary: t.is_primary.unwrap_or(false),
                kind: t.kind,
                path: t.path,
                arguments: t.arguments,
                working_dir: t.working_dir,
            })
            .collect(),
    })
}

pub struct GogScanner {
    registry: Arc<dyn RegistryReader>,
    env: Arc<StoreEnv>,
}

impl GogScanner {
    pub fn new(registry: Arc<dyn RegistryReader>, env: Arc<StoreEnv>) -> Self {
        Self { registry, env }
    }

    fn value(&self, key: &str, name: &str) -> Option<String> {
        self.registry
            .read_string(Hive::LocalMachine, key, name)
            .filter(|v| !v.is_empty())
    }

    /// `Ok(None)` = sem `.info` (vale o registro); `Err` = existe e nao presta (jogo recusado).
    fn read_info(&self, path: &str, id: &str) -> Result<Option<GogInfoRaw>, &'static str> {
        // So toca no disco com ID e pasta de forma valida (nada de rede nem de nome forjado).
        if !is_gog_id(id) || local_components(path).is_err() {
            return Ok(None);
        }
        let Some(dir) = self.env.canon(path) else {
            return Ok(None);
        };
        let file = Path::new(&dir).join(format!("goggame-{id}.info"));
        if !is_plain_file(&file) {
            return Ok(None);
        }
        let bytes = read_limited(&file).map_err(|_| ".info ilegivel ou acima de 4 MB")?;
        parse_info(&bytes).map(Some)
    }
}

impl StoreScanner for GogScanner {
    fn origin(&self) -> Origin {
        Origin::Gog
    }

    fn scan(&self) -> Result<Vec<DetectedGame>, ScanError> {
        let subkeys = self.registry.subkeys(Hive::LocalMachine, GAMES_KEY);
        if subkeys.is_empty() {
            return Err(ScanError::NotInstalled);
        }
        let canon = |p: &str| self.env.canon(p);
        let mut games = Vec::new();
        for sub in subkeys.iter().take(MAX_SUBKEYS) {
            let key = format!(r"{GAMES_KEY}\{sub}");
            let game_id = self.value(&key, "gameID").unwrap_or_else(|| sub.clone());
            let path = self.value(&key, "path");
            let info = match path.as_deref().map(|p| self.read_info(p, &game_id)) {
                Some(Ok(info)) => info,
                Some(Err(motivo)) => {
                    tracing::warn!(loja = "gog", item = sub.as_str(), motivo, "jogo ignorado");
                    continue;
                }
                None => None,
            };
            let raw = GogGameRaw {
                game_id: Some(game_id),
                name: self.value(&key, "gameName"),
                path,
                exe: self.value(&key, "exe"),
                working_dir: self.value(&key, "workingDir"),
                launch_param: self.value(&key, "launchParam"),
                info,
            };
            match gog_game(&raw, &self.env.policy, &canon) {
                Ok(game) => games.push(game),
                Err(reason) => log_reject("gog", sub, reason),
            }
        }
        tracing::debug!(loja = "gog", jogos = games.len(), "varredura do gog");
        Ok(games)
    }
}

#[cfg(test)]
mod tests {
    use super::super::registry::fake::FakeRegistry;
    use super::super::testing::{env, fixtures, write, TempDir};
    use super::*;
    use launcher_core::domain::library::LaunchSpec;
    use std::fs;

    fn info_fixture(dir: &str, id: &str) -> Vec<u8> {
        fs::read(
            fixtures()
                .join("gog")
                .join(dir)
                .join(format!("goggame-{id}.info")),
        )
        .unwrap()
    }

    fn registry_fixture() -> Vec<(String, serde_json::Value)> {
        let text = fs::read_to_string(fixtures().join(r"gog\registry\games.json")).unwrap();
        let v: serde_json::Value = serde_json::from_str(&text).unwrap();
        v.as_object()
            .unwrap()
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect()
    }

    /// Registro fake a partir de `games.json`, com `path`/`exe`/`workingDir` reescritos para
    /// pastas reais do temporario quando o teste pede (`game_dirs`: id -> pasta relativa).
    fn registry(t: &TempDir, game_dirs: &[(&str, &str)]) -> FakeRegistry {
        let mut reg = FakeRegistry::default();
        for (id, fields) in registry_fixture() {
            reg.add_subkey(Hive::LocalMachine, GAMES_KEY, &id);
            let key = format!(r"{GAMES_KEY}\{id}");
            let mapped = game_dirs.iter().find(|(g, _)| *g == id).map(|(_, d)| *d);
            for (name, value) in fields.as_object().unwrap() {
                let mut text = value.as_str().unwrap().to_owned();
                if let Some(d) = mapped {
                    let base = format!(r"{}\{d}", t.text());
                    if name == "path" || name == "workingDir" {
                        text = base;
                    } else if name == "exe" || name == "launchCommand" {
                        let file = text.rsplit('\\').next().unwrap().to_owned();
                        text = format!(r"{base}\{file}");
                    }
                }
                reg.set(Hive::LocalMachine, &key, name, &text);
            }
        }
        reg
    }

    #[test]
    fn jogo_com_info_usa_a_tarefa_primaria_e_sem_info_usa_o_registro() {
        let t = TempDir::new("gog");
        let root = t.text();
        // 1000000001: com `.info` (tarefa primaria `SampleGog.exe`; as outras sao ignoradas).
        write(
            &Path::new(&root).join(r"G1\goggame-1000000001.info"),
            &info_fixture("valid", "1000000001"),
        );
        write(&Path::new(&root).join(r"G1\SampleGog.exe"), b"MZ");
        // 1000000002 do registro (sem `.info`): `exe` g.exe.
        write(&Path::new(&root).join(r"G2\g.exe"), b"MZ");
        let reg = registry(&t, &[("1000000001", "G1"), ("1000000002", "G2")]);
        let mut games = GogScanner::new(Arc::new(reg), env()).scan().unwrap();
        games.sort_by(|a, b| a.id.cmp(&b.id));
        let ids: Vec<String> = games.iter().map(|g| g.id.to_string()).collect();
        assert_eq!(ids, ["gog:1000000001", "gog:1000000002"], "{ids:?}");
        match &games[0].launch {
            LaunchSpec::Exe {
                path,
                args,
                working_dir,
            } => {
                assert!(path.ends_with(r"G1\SampleGog.exe"), "{path}");
                assert!(args.is_empty());
                assert!(working_dir.as_deref().unwrap().ends_with(r"\G1"));
            }
            other => panic!("esperava Exe, veio {other:?}"),
        }
        assert_eq!(games[0].name, "Sample GOG Game");
        match &games[1].launch {
            LaunchSpec::Exe { path, .. } => assert!(path.ends_with(r"G2\g.exe")),
            other => panic!("esperava Exe, veio {other:?}"),
        }
    }

    #[test]
    fn info_com_argumentos_entrega_a_lista() {
        let t = TempDir::new("gog-args");
        let root = t.text();
        write(
            &Path::new(&root).join(r"G\goggame-1000000002.info"),
            &info_fixture("valid", "1000000002"),
        );
        write(&Path::new(&root).join(r"G\bin\game.exe"), b"MZ");
        let reg = registry(&t, &[("1000000002", "G")]);
        let games = GogScanner::new(Arc::new(reg), env()).scan().unwrap();
        let g = games.iter().find(|g| g.id.key() == "1000000002").unwrap();
        match &g.launch {
            LaunchSpec::Exe {
                args, working_dir, ..
            } => {
                assert_eq!(args, &["-windowed", "-nointro", r"C:\Users\Public\x y"]);
                assert!(working_dir.as_deref().unwrap().ends_with(r"G\bin"));
            }
            other => panic!("esperava Exe, veio {other:?}"),
        }
    }

    #[test]
    fn dlc_sem_play_task_e_info_so_com_url_ficam_de_fora() {
        let t = TempDir::new("gog-dlc");
        let root = t.text();
        write(
            &Path::new(&root).join(r"Dlc\goggame-1000000003.info"),
            &info_fixture("valid", "1000000003"),
        );
        write(
            &Path::new(&root).join(r"Url\goggame-2000000006.info"),
            &info_fixture("malicious", "2000000006"),
        );
        let mut reg = FakeRegistry::default();
        for (id, dir) in [("1000000003", "Dlc"), ("2000000006", "Url")] {
            reg.add_subkey(Hive::LocalMachine, GAMES_KEY, id);
            let k = format!(r"{GAMES_KEY}\{id}");
            reg.set(Hive::LocalMachine, &k, "gameID", id);
            reg.set(Hive::LocalMachine, &k, "path", &format!(r"{root}\{dir}"));
            // O registro ofereceria um exe: com `.info` presente ele nao vale.
            reg.set(
                Hive::LocalMachine,
                &k,
                "exe",
                &format!(r"{root}\{dir}\x.exe"),
            );
            write(&Path::new(&root).join(format!(r"{dir}\x.exe")), b"MZ");
        }
        assert!(GogScanner::new(Arc::new(reg), env())
            .scan()
            .unwrap()
            .is_empty());
    }

    #[test]
    fn maliciosos_batch_cmd_ads_unc_lnk_pasta_raiz_e_id_invalido_sao_recusados() {
        let t = TempDir::new("gog-mal");
        let root = t.text();
        let mut reg = FakeRegistry::default();
        // `.info` maliciosos: cada um numa pasta real com o exe/arquivo que ele aponta.
        for n in 1..=8 {
            let id = format!("200000000{n}");
            let dir = format!(r"{root}\M{n}");
            write(
                &Path::new(&dir).join(format!("goggame-{id}.info")),
                &info_fixture("malicious", &id),
            );
            for f in ["start.bat", "g.exe", "atalho.lnk"] {
                write(&Path::new(&dir).join(f), b"MZ");
            }
            reg.add_subkey(Hive::LocalMachine, GAMES_KEY, &id);
            let k = format!(r"{GAMES_KEY}\{id}");
            reg.set(Hive::LocalMachine, &k, "gameID", &id);
            reg.set(Hive::LocalMachine, &k, "path", &dir);
        }
        // Do registro: `path` raiz, exe fora da pasta, id nao numerico.
        let mut fixture_reg = registry(&t, &[]);
        for (k, v) in std::mem::take(&mut fixture_reg.values) {
            reg.values.insert(k, v);
        }
        for (k, v) in std::mem::take(&mut fixture_reg.subkeys) {
            reg.subkeys.entry(k).or_default().extend(v);
        }
        let games = GogScanner::new(Arc::new(reg), env()).scan().unwrap();
        let ids: Vec<String> = games.iter().map(|g| g.id.to_string()).collect();
        // Nenhum dos maliciosos entra. Dos do registro, so os que nao existem no disco de teste
        // tambem ficam de fora (pastas fictícias): lista vazia.
        assert!(ids.is_empty(), "{ids:?}");
    }

    #[test]
    fn gog_ausente_e_not_installed() {
        assert_eq!(
            GogScanner::new(Arc::new(FakeRegistry::default()), env())
                .scan()
                .unwrap_err(),
            ScanError::NotInstalled
        );
    }

    #[test]
    fn info_truncado_nao_cai_no_registro() {
        let t = TempDir::new("gog-trunc");
        let root = t.text();
        write(
            &Path::new(&root).join(r"T\goggame-2000000008.info"),
            &info_fixture("malicious", "2000000008"),
        );
        write(&Path::new(&root).join(r"T\g.exe"), b"MZ");
        let mut reg = FakeRegistry::default();
        reg.add_subkey(Hive::LocalMachine, GAMES_KEY, "2000000008");
        let k = format!(r"{GAMES_KEY}\2000000008");
        reg.set(Hive::LocalMachine, &k, "gameID", "2000000008");
        reg.set(Hive::LocalMachine, &k, "path", &format!(r"{root}\T"));
        reg.set(Hive::LocalMachine, &k, "exe", &format!(r"{root}\T\g.exe"));
        assert!(GogScanner::new(Arc::new(reg), env())
            .scan()
            .unwrap()
            .is_empty());
    }

    #[test]
    fn game_id_numerico_como_numero_no_json_vale() {
        let info = parse_info(br#"{"gameId": 1207658930, "playTasks": []}"#).unwrap();
        assert_eq!(info.game_id.as_deref(), Some("1207658930"));
        assert!(parse_info(b"{truncado").is_err());
        assert!(parse_info(br#"{"playTasks": "nao e lista"}"#).is_err());
    }
}
