//! INV-02 / SEC-03 / SEC-05 / CT-08: a lista fechada de 30 comandos esta igual em todos os lugares
//! (contrato, `command_list.rs`, `generate_handler!`, `build.rs` e `capabilities/main.json`), e a
//! capability nao ganha nada alem disso.

use std::collections::BTreeSet;
use std::fs;
use std::path::PathBuf;

use serde_json::Value;
use sofa_lib::commands::list::COMMAND_NAMES;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn read(rel: &str) -> String {
    fs::read_to_string(root().join(rel)).unwrap_or_else(|e| panic!("{rel}: {e}"))
}

fn set(items: impl IntoIterator<Item = String>) -> BTreeSet<String> {
    items.into_iter().collect()
}

fn list_set() -> BTreeSet<String> {
    set(COMMAND_NAMES.iter().map(|s| (*s).to_owned()))
}

fn kebab(name: &str) -> String {
    name.replace('_', "-")
}

#[test]
fn lista_tem_30_nomes_unicos() {
    assert_eq!(COMMAND_NAMES.len(), 30);
    assert_eq!(list_set().len(), 30);
}

/// Tabela da secao 2 do contrato (`| N | `comando` |`).
#[test]
fn contrato_secao_2_lista_os_mesmos_30_comandos() {
    let doc = read("../docs/api/launcher-tv.md");
    let section = doc
        .split("## 2. Comandos")
        .nth(1)
        .and_then(|s| s.split("\n## 3.").next())
        .expect("secao 2 do contrato");
    let in_doc = set(section.lines().filter_map(|l| {
        let mut cells = l.split('|').map(str::trim);
        cells.next()?; // antes da primeira barra
        let n = cells.next()?;
        let name = cells.next()?;
        n.parse::<u32>().ok()?;
        name.strip_prefix('`')?.strip_suffix('`').map(str::to_owned)
    }));
    assert_eq!(in_doc, list_set());
}

#[test]
fn generate_handler_registra_exatamente_a_lista() {
    let lib = read("src/lib.rs");
    let block = lib
        .split("generate_handler![")
        .nth(1)
        .and_then(|s| s.split(']').next())
        .expect("generate_handler! em lib.rs");
    let in_handler = set(block
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_owned));
    assert_eq!(in_handler, list_set());
}

#[test]
fn build_rs_gera_o_app_manifest_a_partir_da_lista() {
    let build = read("build.rs");
    assert!(build.contains("include!(\"command_list.rs\")"));
    assert!(build.contains("AppManifest::new().commands(COMMAND_NAMES)"));
    assert!(build.contains("app_manifest("));
}

fn capability() -> Value {
    serde_json::from_str(&read("capabilities/main.json")).expect("main.json valido")
}

fn permissions(cap: &Value) -> Vec<String> {
    cap["permissions"]
        .as_array()
        .expect("permissions")
        .iter()
        .map(|p| p.as_str().expect("permissao como texto").to_owned())
        .collect()
}

#[test]
fn capability_main_so_tem_allow_dos_30_comandos_e_listen() {
    let cap = capability();
    assert_eq!(cap["identifier"], "main");
    assert_eq!(cap["windows"], serde_json::json!(["main"]));
    assert!(
        cap.get("remote").is_none(),
        "SEC-05: capability sem `remote`"
    );
    assert!(cap.get("webviews").is_none());

    let perms = permissions(&cap);
    let expected = set(COMMAND_NAMES
        .iter()
        .map(|n| format!("allow-{}", kebab(n)))
        .chain([
            "core:event:allow-listen".to_owned(),
            "core:event:allow-unlisten".to_owned(),
        ]));
    assert_eq!(set(perms.clone()), expected);
    assert_eq!(perms.len(), expected.len(), "permissao duplicada");
}

#[test]
fn capability_nao_libera_nada_proibido() {
    for p in permissions(&capability()) {
        assert!(p != "core:default" && !p.starts_with("core:window"), "{p}");
        assert!(
            !p.starts_with("core:webview") && !p.starts_with("core:app"),
            "{p}"
        );
        for plugin in [
            "shell",
            "fs",
            "http",
            "opener",
            "dialog",
            "process",
            "updater",
            "sql",
            "autostart",
        ] {
            assert!(
                !p.starts_with(&format!("{plugin}:")),
                "SEC-04: plugin proibido {p}"
            );
        }
    }
    // So esta capability existe.
    let files: Vec<_> = fs::read_dir(root().join("capabilities"))
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    assert_eq!(files, vec!["main.json".to_owned()]);
}

#[test]
fn purge_user_data_nao_e_comando_nem_permissao() {
    let all = format!(
        "{}{}{}",
        COMMAND_NAMES.join(" "),
        read("capabilities/main.json"),
        read("src/lib.rs")
            .split("generate_handler![")
            .nth(1)
            .unwrap()
            .split(']')
            .next()
            .unwrap()
    );
    assert!(
        !all.contains("purge"),
        "INV-02: --purge-user-data nunca e comando"
    );
}

#[test]
fn conf_do_tauri_nao_expoe_global_nem_cria_janela_por_config() {
    let conf: Value = serde_json::from_str(&read("tauri.conf.json")).unwrap();
    assert_eq!(conf["app"]["withGlobalTauri"], false, "SEC-05");
    assert_eq!(
        conf["app"]["windows"],
        serde_json::json!([]),
        "janela criada em Rust"
    );
    assert!(
        conf["app"]["security"].get("assetProtocol").is_none(),
        "SEC-08: sem assetProtocol"
    );
}

/// BE-37 (ADR-0006 1, Q-09, P-01): instalador NSIS por maquina, so NSIS (sem MSI), pasta ASCII
/// `Sofa`, bootstrapper do WebView2 embutido, icone da marca e os arquivos do template existentes.
#[test]
fn bundle_e_nsis_por_maquina_com_bootstrapper_embutido() {
    let conf: Value = serde_json::from_str(&read("tauri.conf.json")).unwrap();
    let bundle = &conf["bundle"];
    assert_eq!(bundle["active"], true);
    assert_eq!(bundle["targets"], serde_json::json!(["nsis"]), "sem MSI");
    // D-49, D-56, ADR-0007: muda o nome que o usuario le; os identificadores tecnicos ficam.
    assert_eq!(conf["productName"], "SofaLauncher", "nome visivel, ASCII");
    assert_eq!(
        conf["identifier"], "app.sofa.launcher",
        "pasta de dados, chave HKLM e mutex"
    );
    assert_eq!(
        conf["mainBinaryName"], "sofa",
        "sofa.exe, Run e tarefa do autostart"
    );
    let nsis = &bundle["windows"]["nsis"];
    assert_eq!(nsis["installMode"], "perMachine", "Q-09");
    assert_eq!(nsis["displayLanguageSelector"], false);
    assert_eq!(nsis["languages"], serde_json::json!(["PortugueseBR"]));
    assert_eq!(
        bundle["windows"]["webviewInstallMode"]["type"], "embedBootstrapper",
        "P-01"
    );
    assert_eq!(nsis["installerIcon"], "../docs/brand/sofa/assets/icon.ico");
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    for key in ["template", "installerHooks"] {
        let rel = nsis[key].as_str().unwrap_or_else(|| panic!("{key}"));
        assert!(root.join(rel).is_file(), "{key}: {rel} nao existe");
    }
    let lang = nsis["customLanguageFiles"]["PortugueseBR"]
        .as_str()
        .unwrap();
    assert!(root.join(lang).is_file(), "{lang} nao existe");
    assert!(root.join(nsis["installerIcon"].as_str().unwrap()).is_file());
    // licencas OFL das fontes vao junto do instalador
    let resources = bundle["resources"].as_object().unwrap();
    for (src, dst) in resources {
        assert!(root.join(src).is_file(), "{src} nao existe");
        assert!(dst.as_str().unwrap().starts_with("licenses/"), "{dst}");
    }
    assert_eq!(resources.len(), 2, "OFL-Figtree e OFL-Sora");
}

/// D-57, CA-53, ADR-0007: uma versao so em todos os manifestos, e maior que a 0.1.0 (com
/// `allowDowngrades: false` o instalador so atualiza por cima de uma versao menor).
#[test]
fn versao_unica_nos_manifestos_e_maior_que_a_0_1_0() {
    let conf: Value = serde_json::from_str(&read("tauri.conf.json")).unwrap();
    let version = conf["version"]
        .as_str()
        .expect("version do tauri.conf.json");
    assert_eq!(
        version,
        env!("CARGO_PKG_VERSION"),
        "Cargo.toml do workspace"
    );
    let pkg: Value = serde_json::from_str(&read("../ui/package.json")).unwrap();
    assert_eq!(pkg["version"], version, "ui/package.json");
    let lock: Value = serde_json::from_str(&read("../ui/package-lock.json")).unwrap();
    assert_eq!(lock["version"], version, "ui/package-lock.json");
    assert_eq!(
        lock["packages"][""]["version"], version,
        "ui/package-lock.json, raiz"
    );
    assert_eq!(
        conf["bundle"]["windows"]["allowDowngrades"], false,
        "a 0.2.0 so instala por cima de uma versao menor"
    );
    let parts: Vec<u32> = version.split('.').map(|p| p.parse().unwrap()).collect();
    assert!(
        parts[..] > [0, 1, 0][..],
        "{version} precisa ser maior que a 0.1.0"
    );
}

/// SEC-31, SEC-32: o template do Sofa nao traz de volta a remocao recursiva de dados do Tauri nem
/// escreve em HKCU (o instalador elevado pode ser de outra conta, E-19).
#[test]
fn template_do_instalador_nao_apaga_dados_nem_escreve_em_hkcu() {
    let tpl = read("windows/installer.nsi");
    for line in tpl.lines().filter(|l| !l.trim_start().starts_with(';')) {
        let l = line.to_ascii_lowercase();
        // `RMDir /r` (recursivo); `/REBOOTOK` do $INSTDIR e outra opcao
        let words: Vec<&str> = l.split_whitespace().collect();
        assert!(
            !(words.first() == Some(&"rmdir") && words.contains(&"/r")),
            "SEC-31: {line}"
        );
        assert!(!l.contains("deleteappdata"), "SEC-31: {line}");
        assert!(
            !l.contains("writereg") || !l.contains("hkcu"),
            "E-19: {line}"
        );
        assert!(
            !l.contains("deletereg") || !l.contains("hkcu"),
            "E-19: {line}"
        );
    }
    assert!(tpl.contains("RequestExecutionLevel admin"), "RN-26");
}

#[test]
fn tauri_sem_features_de_devtools_nem_plugins() {
    let manifest = read("Cargo.toml");
    let deps = manifest.split("[dependencies]").nth(1).unwrap();
    let tauri_line = deps
        .lines()
        .find(|l| l.trim_start().starts_with("tauri = "))
        .expect("dependencia tauri");
    assert!(tauri_line.contains("default-features = false"));
    assert!(
        !tauri_line.contains("devtools"),
        "SEC-02: sem a feature devtools"
    );
    for banned in [
        "tauri-plugin-shell",
        "tauri-plugin-fs",
        "tauri-plugin-http",
        "tauri-plugin-opener",
        "tauri-plugin-dialog",
        "tauri-plugin-process",
        "tauri-plugin-updater",
        "tauri-plugin-sql",
        "tauri-plugin-autostart",
    ] {
        assert!(!deps.contains(banned), "SEC-04: {banned}");
    }
}
