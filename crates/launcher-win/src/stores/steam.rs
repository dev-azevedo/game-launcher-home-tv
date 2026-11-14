//! Detector da Steam (BE-25, arquitetura 2.2, SP-08): `SteamPath` no HKCU, `libraryfolders.vdf`,
//! `appmanifest_<appid>.acf`, capa de `librarycache`. Le campos crus; a validacao e do dominio.
//!
//! **Pre-checagem de profundidade 32 e de 4 MB antes do `keyvalues-parser` (SEC-13).** O parser
//! e recursivo: com 10.000 niveis o processo morre por `STATUS_STACK_OVERFLOW`, que
//! `catch_unwind` nao captura. Por isso `vdf_depth_exceeds` roda antes de qualquer parse.

use std::collections::BTreeSet;
use std::fs;
use std::path::Path;
use std::sync::Arc;

use keyvalues_parser::{Obj, Value};
use launcher_core::application::ports::{ScanError, StoreScanner};
use launcher_core::domain::library::{
    local_components, normalize_local, steam_game, DetectedGame, Origin, SteamManifestRaw,
};

use super::registry::{Hive, RegistryReader};
use super::{is_plain_file, log_reject, read_limited, StoreEnv};

/// Profundidade maxima de aninhamento aceita no VDF (SEC-13).
pub const MAX_VDF_DEPTH: usize = 32;
/// Bibliotecas lidas de `libraryfolders.vdf` (uma maquina real tem poucas).
const MAX_LIBRARIES: usize = 32;
/// `appmanifest` lidos por biblioteca.
const MAX_MANIFESTS: usize = 5_000;

/// `true` se o texto aninha mais de `max` niveis de `{`. Conta `{`/`}` fora de aspas, sem
/// interpretar comentario: contar de mais so recusa um arquivo estranho, contar de menos deixaria
/// passar a recursao que derruba o processo. Roda em ~11 ms para 10 MB (SP-08).
pub fn vdf_depth_exceeds(text: &str, max: usize) -> bool {
    let mut depth = 0usize;
    let mut in_quotes = false;
    let mut escaped = false;
    for b in text.bytes() {
        if in_quotes {
            if escaped {
                escaped = false;
            } else if b == b'\\' {
                escaped = true;
            } else if b == b'"' {
                in_quotes = false;
            }
            continue;
        }
        match b {
            b'"' => in_quotes = true,
            b'{' => {
                depth += 1;
                if depth > max {
                    return true;
                }
            }
            b'}' => depth = depth.saturating_sub(1),
            _ => {}
        }
    }
    false
}

/// Chaves e valores de texto do primeiro nivel de um objeto VDF, como `(chave, texto)`.
fn strings_of(obj: &Obj<'_>) -> Vec<(String, String)> {
    obj.iter()
        .flat_map(|(k, vs)| {
            vs.iter().filter_map(move |v| match v {
                Value::Str(s) => Some((k.to_string(), s.to_string())),
                Value::Obj(_) => None,
            })
        })
        .collect()
}

fn get_ci<'a>(pairs: &'a [(String, String)], key: &str) -> Option<&'a str> {
    pairs
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case(key))
        .map(|(_, v)| v.as_str())
}

/// Le um VDF de terceiro com todas as travas: 4 MB, UTF-8, profundidade 32, parser sem `panic`.
/// Devolve o objeto raiz (`"libraryfolders" { ... }`) ja desmembrado em `f`.
fn with_root<T>(path: &Path, f: impl FnOnce(&str, &Obj<'_>) -> T) -> Result<T, &'static str> {
    let bytes = read_limited(path).map_err(|_| "ilegivel ou acima de 4 MB")?;
    let text = std::str::from_utf8(&bytes).map_err(|_| "nao e UTF-8")?;
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    if vdf_depth_exceeds(text, MAX_VDF_DEPTH) {
        return Err("aninhamento acima de 32 niveis");
    }
    let vdf = keyvalues_parser::parse(text).map_err(|_| "VDF invalido")?;
    match &vdf.value {
        Value::Obj(obj) => Ok(f(&vdf.key, obj)),
        Value::Str(_) => Err("raiz do VDF nao e objeto"),
    }
}

/// `appmanifest_<appid>.acf` -> campos crus do `AppState`.
pub fn read_manifest(path: &Path) -> Result<SteamManifestRaw, &'static str> {
    with_root(path, |key, obj| {
        if !key.eq_ignore_ascii_case("AppState") {
            return SteamManifestRaw::default();
        }
        let pairs = strings_of(obj);
        SteamManifestRaw {
            appid: get_ci(&pairs, "appid").map(str::to_owned),
            name: get_ci(&pairs, "name").map(str::to_owned),
            state_flags: get_ci(&pairs, "StateFlags").map(str::to_owned),
            installdir: get_ci(&pairs, "installdir").map(str::to_owned),
        }
    })
}

/// Caminhos das bibliotecas de `libraryfolders.vdf`: formato atual (`"0" { "path" ... }`) e o
/// antigo (`BaseInstallFolder_N`). O que nao reconhece e ignorado.
pub fn read_library_paths(path: &Path) -> Result<Vec<String>, &'static str> {
    with_root(path, |key, obj| {
        if !key.eq_ignore_ascii_case("libraryfolders") {
            return Vec::new();
        }
        let mut out = Vec::new();
        for (k, values) in obj.iter() {
            for v in values {
                match v {
                    Value::Obj(inner) if k.bytes().all(|b| b.is_ascii_digit()) => {
                        if let Some(p) = get_ci(&strings_of(inner), "path") {
                            out.push(p.to_owned());
                        }
                    }
                    Value::Str(p) if k.to_ascii_lowercase().starts_with("baseinstallfolder_") => {
                        out.push(p.to_string());
                    }
                    _ => {}
                }
            }
        }
        out
    })
}

/// `SteamPath` vem com barras normais e unidade minuscula (`c:/program files (x86)/steam`):
/// normaliza para `C:\...` antes de qualquer politica (SP-08).
pub fn normalize_steam_path(raw: &str) -> Option<String> {
    normalize_local(&raw.replace('/', "\\")).ok()
}

/// Capa 600x900 da biblioteca: layout antigo (`<appid>_library_600x900.jpg`) e novo
/// (`<appid>\<hash>\library_600x900.jpg`, subpastas de 1 nivel).
fn find_cover(steam_dir: &str, app_id: u32) -> Option<String> {
    let base = format!(r"{steam_dir}\appcache\librarycache");
    let old = format!(r"{base}\{app_id}_library_600x900.jpg");
    if is_plain_file(Path::new(&old)) {
        return Some(old);
    }
    let dir = format!(r"{base}\{app_id}");
    let direct = format!(r"{dir}\library_600x900.jpg");
    if is_plain_file(Path::new(&direct)) {
        return Some(direct);
    }
    for entry in fs::read_dir(&dir).ok()?.flatten().take(64) {
        if entry.file_type().is_ok_and(|t| t.is_dir()) {
            let candidate = entry.path().join("library_600x900.jpg");
            if is_plain_file(&candidate) {
                return candidate.to_str().map(str::to_owned);
            }
        }
    }
    None
}

pub struct SteamScanner {
    registry: Arc<dyn RegistryReader>,
    env: Arc<StoreEnv>,
}

impl SteamScanner {
    pub fn new(registry: Arc<dyn RegistryReader>, env: Arc<StoreEnv>) -> Self {
        Self { registry, env }
    }

    fn libraries(&self, steam_dir: &str) -> Vec<String> {
        let mut candidates = vec![steam_dir.to_owned()];
        match read_library_paths(Path::new(&format!(
            r"{steam_dir}\steamapps\libraryfolders.vdf"
        ))) {
            Ok(paths) => candidates.extend(paths),
            Err(motivo) => tracing::warn!(loja = "steam", motivo, "libraryfolders.vdf ignorado"),
        }
        let mut seen = BTreeSet::new();
        let mut out = Vec::new();
        for raw in candidates.into_iter().take(MAX_LIBRARIES * 2) {
            // Biblioteca: caminho local absoluto que existe (SEC-12). Unidade desconectada
            // (E-15, E-18), UNC e `..` ficam de fora; a raiz da unidade e uma biblioteca valida.
            if local_components(&raw).is_err() {
                tracing::warn!(loja = "steam", "biblioteca fora da regra de caminho local");
                continue;
            }
            let Some(canonical) = self.env.canon(&raw) else {
                tracing::debug!(loja = "steam", "biblioteca ausente (unidade desconectada?)");
                continue;
            };
            if seen.insert(canonical.to_lowercase()) {
                out.push(canonical);
            }
            if out.len() >= MAX_LIBRARIES {
                break;
            }
        }
        out
    }

    fn scan_library(
        &self,
        library: &str,
        steam_dir: &str,
        policy: &launcher_core::domain::library::InstallPathPolicy,
        out: &mut Vec<DetectedGame>,
    ) {
        let apps = format!(r"{library}\steamapps");
        let Ok(read) = fs::read_dir(&apps) else {
            tracing::debug!(loja = "steam", "biblioteca sem steamapps");
            return;
        };
        let canon = |p: &str| self.env.canon(p);
        for entry in read.flatten().take(MAX_MANIFESTS) {
            let name = entry.file_name();
            let Some(name) = name.to_str() else { continue };
            let lower = name.to_ascii_lowercase();
            if !(lower.starts_with("appmanifest_") && lower.ends_with(".acf")) {
                continue;
            }
            let path = entry.path();
            if !is_plain_file(&path) {
                continue;
            }
            let raw = match read_manifest(&path) {
                Ok(r) => r,
                Err(motivo) => {
                    tracing::warn!(loja = "steam", item = name, motivo, "manifesto ignorado");
                    continue;
                }
            };
            match steam_game(&raw, library, policy, &canon) {
                Ok(mut game) => {
                    if let launcher_core::domain::library::LaunchSpec::SteamUri { app_id } =
                        game.launch
                    {
                        game.cover_source = find_cover(steam_dir, app_id);
                    }
                    out.push(game);
                }
                Err(reason) => log_reject("steam", name, reason),
            }
        }
    }
}

impl StoreScanner for SteamScanner {
    fn origin(&self) -> Origin {
        Origin::Steam
    }

    fn scan(&self) -> Result<Vec<DetectedGame>, ScanError> {
        let raw = self
            .registry
            .read_string(Hive::CurrentUser, r"Software\Valve\Steam", "SteamPath")
            .ok_or(ScanError::NotInstalled)?;
        let steam_dir = normalize_steam_path(&raw)
            .and_then(|p| self.env.canon(&p))
            .ok_or(ScanError::NotInstalled)?;
        // A pasta do cliente da Steam nunca e pasta de jogo (SEC-16, RN-10).
        let policy = self.env.policy.with_forbidden([steam_dir.clone()]);
        let mut games = Vec::new();
        for library in self.libraries(&steam_dir) {
            self.scan_library(&library, &steam_dir, &policy, &mut games);
        }
        tracing::debug!(loja = "steam", jogos = games.len(), "varredura da steam");
        Ok(games)
    }
}

#[cfg(test)]
mod tests {
    use super::super::registry::fake::FakeRegistry;
    use super::super::testing::{env, fixtures, vdf_escape, write, TempDir};
    use super::*;
    use launcher_core::domain::library::LaunchSpec;
    use std::time::Instant;

    fn fixture(rel: &str) -> Vec<u8> {
        fs::read(fixtures().join("steam").join(rel)).unwrap()
    }

    /// Monta duas bibliotecas reais num temporario com os manifestos das fixtures e um
    /// `libraryfolders.vdf` apontando para elas. Devolve (raiz, steam_dir, lib2).
    struct Rig {
        _t: TempDir,
        steam: String,
        lib2: String,
        registry: FakeRegistry,
    }

    fn rig() -> Rig {
        let t = TempDir::new("steam");
        let root = t.text();
        let steam = format!(r"{root}\Steam");
        let lib2 = format!(r"{root}\SteamLibrary");
        let vdf = String::from_utf8(fixture("libraryfolders.vdf"))
            .unwrap()
            .replace(r"C:\\Program Files (x86)\\Steam", &vdf_escape(&steam))
            .replace(r"D:\\SteamLibrary", &vdf_escape(&lib2));
        write(
            Path::new(&format!(r"{steam}\steamapps\libraryfolders.vdf")),
            vdf.as_bytes(),
        );
        // Biblioteca principal: Portal 2 (instalado), nao-jogo 228980, incompletos e pendente.
        for (id, install) in [
            ("620", Some("Portal 2")),
            ("228980", Some("Steamworks Shared")),
            ("70", None),
            ("71", None),
            ("72", Some("Atualizacao Pendente")),
            ("73", Some("Pokemon Epico")),
        ] {
            let acf = fixture(&format!(r"steamapps\appmanifest_{id}.acf"));
            write(
                Path::new(&format!(r"{steam}\steamapps\appmanifest_{id}.acf")),
                &acf,
            );
            if let Some(dir) = install {
                fs::create_dir_all(format!(r"{steam}\steamapps\common\{dir}")).unwrap();
            }
        }
        // Segunda biblioteca (outra "unidade"): Team Fortress 2.
        write(
            Path::new(&format!(r"{lib2}\steamapps\appmanifest_440.acf")),
            &fixture(r"steamapps\appmanifest_440.acf"),
        );
        fs::create_dir_all(format!(r"{lib2}\steamapps\common\Team Fortress 2")).unwrap();
        let mut registry = FakeRegistry::default();
        registry.set(
            Hive::CurrentUser,
            r"Software\Valve\Steam",
            "SteamPath",
            &steam.to_lowercase().replace('\\', "/"), // como a Steam grava (SP-08)
        );
        Rig {
            _t: t,
            steam,
            lib2,
            registry,
        }
    }

    #[test]
    fn profundidade_conta_chaves_fora_de_aspas() {
        let ok = format!("{}{}", "\"a\" {".repeat(32), "}".repeat(32));
        assert!(!vdf_depth_exceeds(&ok, 32));
        let ruim = format!("{}{}", "\"a\" {".repeat(33), "}".repeat(33));
        assert!(vdf_depth_exceeds(&ruim, 32));
        // Chaves dentro de aspas (e aspas escapadas) nao contam.
        let aspas = format!("\"k\" \"{}\" \"x\\\"{{{{\" ", "{".repeat(100));
        assert!(!vdf_depth_exceeds(&aspas, 32));
        // `}` sobrando nao deixa o contador negativo.
        assert!(!vdf_depth_exceeds(
            &format!("{}{}", "}".repeat(5), "{".repeat(32)),
            32
        ));
    }

    #[test]
    fn vdf_de_1000_niveis_e_de_10_mb_sao_recusados_sem_parse() {
        let t = TempDir::new("steam-vdf");
        // 1.000 niveis: com 1 MB de pilha o parser aguenta, mas a regra e 32 (e 10.000 mataria).
        let fundo = format!(
            "\"AppState\" {}{}",
            "\"a\" {".repeat(1000),
            "}".repeat(1000)
        );
        let p = t.path().join("fundo.acf");
        fs::write(&p, fundo).unwrap();
        assert_eq!(
            read_manifest(&p).unwrap_err(),
            "aninhamento acima de 32 niveis"
        );
        // 10.000 niveis: sem a pre-checagem isto derrubaria o processo de teste.
        let muito = format!(
            "\"AppState\" {}{}",
            "\"a\" {".repeat(10_000),
            "}".repeat(10_000)
        );
        let p = t.path().join("muito.acf");
        fs::write(&p, muito).unwrap();
        assert_eq!(
            read_manifest(&p).unwrap_err(),
            "aninhamento acima de 32 niveis"
        );
        // 10 MB rasos: recusado pelo tamanho, sem ler.
        let p = t.path().join("grande.acf");
        let mut big = String::from("\"AppState\"\n{\n");
        while big.len() < 10 * 1024 * 1024 {
            big.push_str("\t\"chave\"\t\"valor\"\n");
        }
        big.push('}');
        fs::write(&p, &big).unwrap();
        let t0 = Instant::now();
        assert_eq!(read_manifest(&p).unwrap_err(), "ilegivel ou acima de 4 MB");
        assert!(t0.elapsed().as_millis() < 200);
        // Arquivo truncado e invalido sao `Err`, nunca `panic`.
        let p = t.path().join("trunc.acf");
        fs::write(&p, fixture(r"malicious\steamapps\appmanifest_9013.acf")).unwrap();
        assert_eq!(read_manifest(&p).unwrap_err(), "VDF invalido");
        let p = t.path().join("bin.acf");
        fs::write(&p, [0xff, 0xfe, 0x00, 0x80]).unwrap();
        assert_eq!(read_manifest(&p).unwrap_err(), "nao e UTF-8");
    }

    #[test]
    fn manifesto_valido_vira_campos_crus_e_libraryfolders_traz_as_duas_bibliotecas() {
        let t = TempDir::new("steam-raw");
        let p = t.path().join("a.acf");
        fs::write(&p, fixture(r"steamapps\appmanifest_620.acf")).unwrap();
        let m = read_manifest(&p).unwrap();
        assert_eq!(m.appid.as_deref(), Some("620"));
        assert_eq!(m.name.as_deref(), Some("Portal 2"));
        assert_eq!(m.state_flags.as_deref(), Some("4"));
        assert_eq!(m.installdir.as_deref(), Some("Portal 2"));
        let l = t.path().join("libraryfolders.vdf");
        fs::write(&l, fixture("libraryfolders.vdf")).unwrap();
        assert_eq!(
            read_library_paths(&l).unwrap(),
            [r"C:\Program Files (x86)\Steam", r"D:\SteamLibrary"]
        );
        // Formato antigo.
        fs::write(
            &l,
            "\"LibraryFolders\"\n{\n\t\"TimeNextStatsReport\"\t\"1\"\n\t\"BaseInstallFolder_1\"\t\"D:\\\\Antiga\"\n}\n",
        )
        .unwrap();
        assert_eq!(read_library_paths(&l).unwrap(), [r"D:\Antiga"]);
    }

    #[test]
    fn steam_path_com_barras_normais_e_minuscula_e_normalizado() {
        assert_eq!(
            normalize_steam_path("c:/program files (x86)/steam").as_deref(),
            Some(r"C:\program files (x86)\steam")
        );
        assert_eq!(normalize_steam_path("//srv/share/steam"), None);
        assert_eq!(normalize_steam_path(""), None);
    }

    #[test]
    fn varredura_com_as_fixtures_acha_so_os_jogos_instalados_nas_duas_bibliotecas() {
        let r = rig();
        let scanner = SteamScanner::new(Arc::new(r.registry), env());
        let games = scanner.scan().unwrap();
        let mut ids: Vec<String> = games.iter().map(|g| g.id.to_string()).collect();
        ids.sort();
        // 620 (instalado), 72 (StateFlags 6, atualizacao pendente), 73 (acento), 440 (outra
        // biblioteca). Fora: 228980 (nao-jogo), 70 e 71 (StateFlags 1 e 2... incompletos).
        assert_eq!(
            ids,
            ["steam:440", "steam:620", "steam:72", "steam:73"],
            "{ids:?}"
        );
        let portal = games.iter().find(|g| g.id.key() == "620").unwrap();
        assert_eq!(portal.name, "Portal 2");
        assert_eq!(portal.launch, LaunchSpec::SteamUri { app_id: 620 });
        assert!(portal
            .install_dir
            .as_deref()
            .unwrap()
            .ends_with(r"steamapps\common\Portal 2"));
        let tf2 = games.iter().find(|g| g.id.key() == "440").unwrap();
        assert!(tf2.install_dir.as_deref().unwrap().starts_with(&r.lib2));
        let _ = r.steam;
    }

    #[test]
    fn mesmo_jogo_em_duas_bibliotecas_aparece_uma_vez_depois_da_fusao() {
        let r = rig();
        // 620 tambem na segunda biblioteca.
        write(
            Path::new(&format!(r"{}\steamapps\appmanifest_620.acf", r.lib2)),
            &fixture(r"steamapps\appmanifest_620.acf"),
        );
        fs::create_dir_all(format!(r"{}\steamapps\common\Portal 2", r.lib2)).unwrap();
        let games = SteamScanner::new(Arc::new(r.registry), env())
            .scan()
            .unwrap();
        assert_eq!(games.iter().filter(|g| g.id.key() == "620").count(), 2);
        let merged = launcher_core::domain::library::merge_library(games, &[], &Default::default());
        assert_eq!(merged.iter().filter(|g| g.id.key() == "620").count(), 1);
    }

    #[test]
    fn manifestos_maliciosos_sao_recusados_e_o_resto_carrega() {
        let r = rig();
        let apps = format!(r"{}\steamapps", r.steam);
        let mal = fs::read_dir(fixtures().join(r"steam\malicious\steamapps")).unwrap();
        for e in mal.flatten() {
            let n = e.file_name().into_string().unwrap();
            if n.starts_with("appmanifest_") {
                fs::copy(e.path(), format!(r"{apps}\{n}")).unwrap();
            }
        }
        // Pastas que os manifestos apontam e que existem de verdade: so a validacao os barra.
        for d in ["Valido", "Html", "Bidi"] {
            fs::create_dir_all(format!(r"{apps}\common\{d}")).unwrap();
        }
        let games = SteamScanner::new(Arc::new(r.registry), env())
            .scan()
            .unwrap();
        let mut ids: Vec<String> = games.iter().map(|g| g.id.to_string()).collect();
        ids.sort();
        // Entram os 4 legitimos e os dois de nome estranho (id e pasta validos: o nome so e
        // normalizado). Os de `appid` com `//`, `?&`, espaco, `u32` estourado e os de `installdir`
        // com `..`, `/`, `:`, vazio, `.` ou ausente ficam de fora.
        assert_eq!(
            ids,
            [
                "steam:440",
                "steam:620",
                "steam:72",
                "steam:73",
                "steam:9008",
                "steam:9009"
            ],
            "{ids:?}"
        );
        let merged = launcher_core::domain::library::merge_library(games, &[], &Default::default());
        let nome = |k: &str| {
            merged
                .iter()
                .find(|e| e.id.key() == k)
                .unwrap()
                .display_name()
                .to_owned()
        };
        assert_eq!(nome("9008"), "<img src=x onerror=alert(1)>"); // literal: a UI so renderiza texto
        assert!(!nome("9009").contains('\u{202e}'));
    }

    #[test]
    fn steam_ausente_e_not_installed_e_unidade_desconectada_so_pula_a_biblioteca() {
        let vazio = FakeRegistry::default();
        assert_eq!(
            SteamScanner::new(Arc::new(vazio), env())
                .scan()
                .unwrap_err(),
            ScanError::NotInstalled
        );
        // SteamPath aponta para pasta que nao existe: tambem e "nao instalada".
        let mut ruim = FakeRegistry::default();
        ruim.set(
            Hive::CurrentUser,
            r"Software\Valve\Steam",
            "SteamPath",
            "z:/nao/existe",
        );
        assert_eq!(
            SteamScanner::new(Arc::new(ruim), env()).scan().unwrap_err(),
            ScanError::NotInstalled
        );
        // Biblioteca em unidade desconectada some; a principal continua (E-18).
        let mut r = rig();
        let vdf = format!(r"{}\steamapps\libraryfolders.vdf", r.steam);
        let text = fs::read_to_string(&vdf)
            .unwrap()
            .replace(&vdf_escape(&r.lib2), r"Z:\\Desconectada");
        fs::write(&vdf, text).unwrap();
        let games = SteamScanner::new(Arc::new(std::mem::take(&mut r.registry)), env())
            .scan()
            .unwrap();
        assert!(games.iter().all(|g| g.id.key() != "440"));
        assert!(games.iter().any(|g| g.id.key() == "620"));
    }

    #[test]
    fn biblioteca_unc_e_com_ponto_ponto_sao_ignoradas() {
        let r = rig();
        let vdf = format!(r"{}\steamapps\libraryfolders.vdf", r.steam);
        fs::write(&vdf, fixture(r"malicious\steamapps\libraryfolders.vdf")).unwrap();
        let games = SteamScanner::new(Arc::new(r.registry), env())
            .scan()
            .unwrap();
        // So a biblioteca do `SteamPath` (sempre incluida) vale.
        assert!(games.iter().all(|g| g.id.key() != "440"));
        assert!(games.iter().any(|g| g.id.key() == "620"));
    }

    #[test]
    fn capa_nos_dois_layouts_do_librarycache() {
        let r = rig();
        let cache = format!(r"{}\appcache\librarycache", r.steam);
        write(
            Path::new(&format!(r"{cache}\620_library_600x900.jpg")),
            b"jpg",
        );
        write(
            Path::new(&format!(r"{cache}\440\0123abcd\library_600x900.jpg")),
            b"jpg",
        );
        write(
            Path::new(&format!(r"{cache}\72\library_600x900.jpg")),
            b"jpg",
        );
        let games = SteamScanner::new(Arc::new(r.registry), env())
            .scan()
            .unwrap();
        let cover = |k: &str| {
            games
                .iter()
                .find(|g| g.id.key() == k)
                .unwrap()
                .cover_source
                .clone()
        };
        assert!(cover("620").unwrap().ends_with("620_library_600x900.jpg"));
        assert!(cover("440")
            .unwrap()
            .ends_with(r"440\0123abcd\library_600x900.jpg"));
        assert!(cover("72").unwrap().ends_with(r"72\library_600x900.jpg"));
        assert_eq!(cover("73"), None);
    }
}
