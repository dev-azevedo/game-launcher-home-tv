//! Detector da Epic (BE-26, arquitetura 2.2, SP-08): `%ProgramData%\Epic\EpicGamesLauncher\Data\
//! Manifests\*.item` (JSON). Remove o BOM UTF-8 antes do parse; campos desconhecidos sao
//! ignorados (sem `deny_unknown_fields`: o schema fechado vale so para os JSON do proprio Sofa);
//! campo com tipo errado derruba so aquele item. Validacao: `launcher_core::domain::library::
//! epic_game`.

use std::fs;
use std::path::PathBuf;
use std::sync::Arc;

use launcher_core::application::ports::{ScanError, StoreScanner};
use launcher_core::domain::library::{epic_game, DetectedGame, EpicManifestRaw, Origin};
use serde::Deserialize;

use super::{is_plain_file, log_reject, read_limited, StoreEnv};

/// `.item` lidos de uma vez (uma maquina real tem dezenas).
const MAX_ITEMS: usize = 5_000;

#[derive(Deserialize, Default)]
struct ItemJson {
    #[serde(rename = "AppName")]
    app_name: Option<String>,
    #[serde(rename = "MainGameAppName")]
    main_game_app_name: Option<String>,
    #[serde(rename = "CatalogNamespace")]
    catalog_namespace: Option<String>,
    #[serde(rename = "CatalogItemId")]
    catalog_item_id: Option<String>,
    #[serde(rename = "DisplayName")]
    display_name: Option<String>,
    #[serde(rename = "InstallLocation")]
    install_location: Option<String>,
    #[serde(rename = "LaunchExecutable")]
    launch_executable: Option<String>,
    #[serde(rename = "bIsIncompleteInstall")]
    incomplete: Option<bool>,
}

/// Bytes de um `.item` -> campos crus. BOM removido; JSON invalido ou com tipo errado = `Err`.
pub fn parse_item(bytes: &[u8]) -> Result<EpicManifestRaw, &'static str> {
    let bytes = bytes.strip_prefix(&[0xEF, 0xBB, 0xBF][..]).unwrap_or(bytes);
    let item: ItemJson = serde_json::from_slice(bytes).map_err(|_| "JSON invalido")?;
    Ok(EpicManifestRaw {
        app_name: item.app_name,
        main_game_app_name: item.main_game_app_name,
        catalog_namespace: item.catalog_namespace,
        catalog_item_id: item.catalog_item_id,
        display_name: item.display_name,
        install_location: item.install_location,
        launch_executable: item.launch_executable,
        incomplete: item.incomplete.unwrap_or(false),
    })
}

pub struct EpicScanner {
    /// `%ProgramData%\Epic\EpicGamesLauncher\Data\Manifests`; `None` se o Windows nao informou.
    manifests_dir: Option<PathBuf>,
    env: Arc<StoreEnv>,
}

impl EpicScanner {
    pub fn new(manifests_dir: Option<PathBuf>, env: Arc<StoreEnv>) -> Self {
        Self { manifests_dir, env }
    }
}

impl StoreScanner for EpicScanner {
    fn origin(&self) -> Origin {
        Origin::Epic
    }

    fn scan(&self) -> Result<Vec<DetectedGame>, ScanError> {
        let dir = self.manifests_dir.as_ref().ok_or(ScanError::NotInstalled)?;
        let read = fs::read_dir(dir).map_err(|e| match e.kind() {
            std::io::ErrorKind::NotFound => ScanError::NotInstalled,
            _ => ScanError::Failed,
        })?;
        let canon = |p: &str| self.env.canon(p);
        let mut games = Vec::new();
        for entry in read.flatten().take(MAX_ITEMS) {
            let name = entry.file_name();
            let Some(name) = name.to_str() else { continue };
            if !name.to_ascii_lowercase().ends_with(".item") {
                continue;
            }
            let path = entry.path();
            if !is_plain_file(&path) {
                continue;
            }
            let bytes = match read_limited(&path) {
                Ok(b) => b,
                Err(error) => {
                    tracing::warn!(loja = "epic", item = name, %error, "manifesto ilegivel ou acima de 4 MB");
                    continue;
                }
            };
            let raw = match parse_item(&bytes) {
                Ok(r) => r,
                Err(motivo) => {
                    tracing::warn!(loja = "epic", item = name, motivo, "manifesto ignorado");
                    continue;
                }
            };
            match epic_game(&raw, &self.env.policy, &canon) {
                Ok(game) => games.push(game),
                Err(reason) => log_reject("epic", name, reason),
            }
        }
        tracing::debug!(loja = "epic", jogos = games.len(), "varredura da epic");
        Ok(games)
    }
}

#[cfg(test)]
mod tests {
    use super::super::testing::{env, fixtures, write, TempDir};
    use super::*;
    use launcher_core::domain::library::LaunchSpec;
    use std::path::Path;

    fn item(dir: &str, file: &str) -> Vec<u8> {
        fs::read(fixtures().join("epic").join(dir).join(file)).unwrap()
    }

    /// Reescreve `InstallLocation` do manifesto para uma pasta real do temporario e cria o
    /// executavel dentro dela (as fixtures trazem caminhos de `Program Files`).
    fn install(
        t: &TempDir,
        bytes: &[u8],
        game_dir: &str,
        exe_rel: &str,
        create_exe: bool,
    ) -> Vec<u8> {
        let loc = format!(r"{}\{game_dir}", t.text());
        if create_exe {
            write(Path::new(&format!(r"{loc}\{exe_rel}")), b"MZ");
        }
        let mut v: serde_json::Value =
            serde_json::from_slice(bytes.strip_prefix(&[0xEF, 0xBB, 0xBF][..]).unwrap_or(bytes))
                .unwrap();
        v["InstallLocation"] = serde_json::Value::String(loc);
        v["LaunchExecutable"] = serde_json::Value::String(exe_rel.to_owned());
        serde_json::to_vec(&v).unwrap()
    }

    fn scan_dir(manifests: &Path) -> Result<Vec<DetectedGame>, ScanError> {
        EpicScanner::new(Some(manifests.to_path_buf()), env()).scan()
    }

    #[test]
    fn jogo_valido_dlc_incompleto_e_campo_desconhecido() {
        let t = TempDir::new("epic");
        let m = t.path().join("Manifests");
        // A1: jogo; B2: outra unidade (aqui, outra pasta); C3: incompleto; D4: DLC;
        // E5: campo desconhecido (continua jogo).
        let a1 = item("Manifests", "A1B2C3D4E5F6.item");
        write(
            &m.join("A1B2C3D4E5F6.item"),
            &install(&t, &a1, "SampleGameA", r"Bin\SampleGameA.exe", true),
        );
        let b2 = item("Manifests", "B2C3D4E5F6A1.item");
        write(
            &m.join("B2C3D4E5F6A1.item"),
            &install(&t, &b2, "Acentuado", "Jogo.exe", true),
        );
        let c3 = item("Manifests", "C3D4E5F6A1B2.item");
        write(
            &m.join("C3D4E5F6A1B2.item"),
            &install(&t, &c3, "Incompleto", "I.exe", true),
        );
        let d4 = item("Manifests", "D4E5F6A1B2C3.item");
        write(
            &m.join("D4E5F6A1B2C3.item"),
            &install(&t, &d4, "SampleGameA", r"Bin\SampleGameA.exe", false),
        );
        let e5 = item("Manifests", "E5F6A1B2C3D4.item");
        write(
            &m.join("E5F6A1B2C3D4.item"),
            &install(&t, &e5, "CampoNovo", "CampoNovo.exe", true),
        );
        write(&m.join("leiame.txt"), b"nao e item");

        let mut games = scan_dir(&m).unwrap();
        games.sort_by(|a, b| a.id.cmp(&b.id));
        let ids: Vec<String> = games.iter().map(|g| g.id.to_string()).collect();
        assert_eq!(ids.len(), 3, "{ids:?}");
        assert!(ids.contains(&"epic:SampleGameA".to_owned()));
        assert!(ids.contains(&"epic:CampoNovo".to_owned()));
        assert!(
            !ids.iter().any(|i| i.contains("Dlc")),
            "DLC nao entra: {ids:?}"
        );
        let a = games.iter().find(|g| g.id.key() == "SampleGameA").unwrap();
        assert!(
            matches!(&a.launch, LaunchSpec::EpicUri { app_name, .. } if app_name == "SampleGameA")
        );
        assert!(a
            .icon_source
            .as_deref()
            .unwrap()
            .ends_with(r"Bin\SampleGameA.exe"));
        assert_eq!(a.cover_source, None); // a Epic nao tem capa local: ficam o icone do exe e o placeholder
    }

    /// SEC-BUG-02: um `.item` com `InstallLocation` = pasta do cliente da Steam e executavel dentro
    /// de `steamapps\common` nao pode virar jogo da Epic: a pasta da Steam e proibida para todas as
    /// lojas (`StoreEnv::forbidden_dirs`).
    #[test]
    fn item_da_epic_apontando_para_a_pasta_da_steam_e_recusado() {
        use launcher_core::domain::library::InstallPathPolicy;

        let t = TempDir::new("epic-steam");
        let m = t.path().join("Manifests");
        let steam_dir = format!(r"{}\Steam", t.text());
        let a1 = item("Manifests", "A1B2C3D4E5F6.item");
        write(
            &m.join("A1B2C3D4E5F6.item"),
            &install(&t, &a1, "Steam", r"steamapps\common\X\x.exe", true),
        );
        let root = crate::path_canon::known::windows_dir().unwrap();
        let scan_with = |forbidden: Vec<String>| {
            let env = Arc::new(StoreEnv::new(
                InstallPathPolicy::new(&root, forbidden),
                Arc::new(crate::path_canon::FsPathResolver),
            ));
            EpicScanner::new(Some(m.clone()), env).scan().unwrap()
        };
        // Sem a pasta da Steam na lista, o manifesto malicioso passaria.
        assert_eq!(scan_with(Vec::new()).len(), 1);
        // A lista real inclui a pasta da Steam (so ela entra aqui; o temp fica no perfil).
        let forbidden = StoreEnv::forbidden_dirs(Some(steam_dir.clone()));
        assert!(forbidden.contains(&steam_dir), "{forbidden:?}");
        assert!(scan_with(vec![steam_dir]).is_empty());
    }

    #[test]
    fn bom_utf8_e_removido_antes_do_parse() {
        let com_bom = item("malicious\\Manifests", "m13.item");
        assert_eq!(&com_bom[..3], &[0xEF, 0xBB, 0xBF]);
        assert!(parse_item(&com_bom).is_ok());
        // Sem a remocao o serde_json recusaria: garante que o teste distingue.
        assert!(serde_json::from_slice::<serde_json::Value>(&com_bom).is_err());
        // De ponta a ponta: o jogo com BOM aparece.
        let t = TempDir::new("epic-bom");
        let m = t.path().join("Manifests");
        let mut bytes = vec![0xEF, 0xBB, 0xBF];
        bytes.extend(install(&t, &com_bom, "ComBom", "C.exe", true));
        write(&m.join("m13.item"), &bytes);
        let games = scan_dir(&m).unwrap();
        assert_eq!(games.len(), 1);
        assert_eq!(games[0].id.to_string(), "epic:ComBom");
    }

    #[test]
    fn json_truncado_tipo_errado_e_item_sem_catalogo_ficam_de_fora() {
        assert_eq!(
            parse_item(&item("malicious\\Manifests", "m12.item")).unwrap_err(),
            "JSON invalido"
        );
        assert_eq!(
            parse_item(&item("malicious\\Manifests", "m14.item")).unwrap_err(),
            "JSON invalido"
        );
        let sem = parse_item(&item("malicious\\Manifests", "m15.item")).unwrap();
        assert!(
            sem.catalog_namespace.as_deref().unwrap_or("").is_empty()
                || sem.catalog_item_id.as_deref().unwrap_or("").is_empty()
        );
    }

    #[test]
    fn manifestos_maliciosos_sao_recusados_com_o_resto_carregando() {
        let t = TempDir::new("epic-mal");
        let m = t.path().join("Manifests");
        // Um jogo bom para provar que o resto carrega.
        let a1 = item("Manifests", "A1B2C3D4E5F6.item");
        write(&m.join("bom.item"), &install(&t, &a1, "Bom", "B.exe", true));
        // Todos os maliciosos, sem mexer em nada (os caminhos deles nao existem ou sao proibidos).
        let dir = fixtures().join(r"epic\malicious\Manifests");
        for e in fs::read_dir(dir).unwrap().flatten() {
            let n = e.file_name().into_string().unwrap();
            write(&m.join(&n), &fs::read(e.path()).unwrap());
        }
        let games = scan_dir(&m).unwrap();
        let ids: Vec<String> = games.iter().map(|g| g.id.to_string()).collect();
        // `AppName` com `?`, `&`, `/` e espaco, `InstallLocation` raiz/UNC/Windows/Sofa, `..` no
        // exe e exe absoluto: nenhum entra. So o jogo bom (e nenhum com id fora do formato).
        assert_eq!(ids, ["epic:SampleGameA"], "{ids:?}");
    }

    #[test]
    fn nome_de_5000_caracteres_chega_inteiro_e_a_fusao_corta_em_256() {
        let t = TempDir::new("epic-long");
        let m = t.path().join("Manifests");
        let m09 = item("malicious\\Manifests", "m09.item");
        write(
            &m.join("m09.item"),
            &install(&t, &m09, "Longo", "L.exe", true),
        );
        let games = scan_dir(&m).unwrap();
        assert_eq!(games.len(), 1, "o item m09 e valido, so tem nome enorme");
        let merged = launcher_core::domain::library::merge_library(games, &[], &Default::default());
        assert!(merged[0].display_name().chars().count() <= 256);
    }

    #[test]
    fn epic_ausente_e_not_installed_sem_erro() {
        assert_eq!(
            EpicScanner::new(None, env()).scan().unwrap_err(),
            ScanError::NotInstalled
        );
        let t = TempDir::new("epic-none");
        assert_eq!(
            scan_dir(&t.path().join("NaoExiste")).unwrap_err(),
            ScanError::NotInstalled
        );
        // Pasta presente e vazia: nenhum jogo, sem erro.
        fs::create_dir_all(t.path().join("Vazia")).unwrap();
        assert!(scan_dir(&t.path().join("Vazia")).unwrap().is_empty());
    }

    #[test]
    fn item_acima_de_4_mb_e_ignorado() {
        let t = TempDir::new("epic-big");
        let m = t.path().join("Manifests");
        let mut big = br#"{"AppName":"X","pad":""#.to_vec();
        big.extend(std::iter::repeat(b'a').take(5 * 1024 * 1024));
        big.extend_from_slice(br#""}"#);
        write(&m.join("big.item"), &big);
        assert!(scan_dir(&m).unwrap().is_empty());
    }
}
