//! SEC-01 / BE-12: guard de CI da CSP. A CSP de release do `tauri.conf.json` nao pode ter
//! `unsafe-inline`, `unsafe-eval` nem origem externa; a de dev (`devCsp`) so pode ter o servidor
//! local do Vite e nunca `unsafe-eval`. O verificador tem testes proprios com CSPs ruins, para
//! provar que o guard falha quando deve.

use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

use serde_json::Value;

/// Hosts de rede permitidos na CSP de release (protocolos customizados do Tauri no Windows).
const RELEASE_HOSTS: &[&str] = &["http://ipc.localhost", "http://cover.localhost"];
/// Hosts permitidos so na `devCsp` (HMR do Vite, `host: 127.0.0.1`, `strictPort`).
const DEV_HOSTS: &[&str] = &["http://127.0.0.1:5173", "ws://127.0.0.1:5173"];

fn parse(csp: &str) -> BTreeMap<String, Vec<String>> {
    csp.split(';')
        .map(str::trim)
        .filter(|d| !d.is_empty())
        .map(|d| {
            let mut parts = d.split_whitespace();
            let name = parts.next().unwrap().to_owned();
            (name, parts.map(str::to_owned).collect())
        })
        .collect()
}

/// Problemas encontrados. `dev = true` aplica as regras da `devCsp`.
fn check(csp: &str, dev: bool) -> Vec<String> {
    let mut problems = Vec::new();
    let directives = parse(csp);
    let allowed_hosts: Vec<&str> = if dev {
        RELEASE_HOSTS.iter().chain(DEV_HOSTS).copied().collect()
    } else {
        RELEASE_HOSTS.to_vec()
    };

    for (name, sources) in &directives {
        for s in sources {
            let lower = s.to_ascii_lowercase();
            match lower.as_str() {
                "'unsafe-eval'" | "'wasm-unsafe-eval'" | "'unsafe-hashes'" => {
                    problems.push(format!("{name}: {s} e proibido (release e dev)"));
                }
                "'unsafe-inline'" => {
                    // Dev: so style-src (HMR do Vite injeta <style>). Release: nunca.
                    if !(dev && name == "style-src") {
                        problems.push(format!("{name}: {s} fora do que a devCsp permite"));
                    }
                }
                "*" => problems.push(format!("{name}: curinga `*` e proibido")),
                "blob:" | "filesystem:" | "http:" | "https:" | "ws:" | "wss:" => {
                    problems.push(format!("{name}: esquema inteiro {s} e proibido"));
                }
                "data:" => {
                    if name != "img-src" {
                        problems.push(format!("{name}: data: so e permitido em img-src"));
                    }
                }
                _ => {
                    let is_url = ["http://", "https://", "ws://", "wss://"]
                        .iter()
                        .any(|p| lower.starts_with(p));
                    if is_url && !allowed_hosts.iter().any(|h| lower == *h) {
                        problems.push(format!("{name}: origem externa {s}"));
                    }
                }
            }
        }
    }

    let expect_none = |dir: &str, problems: &mut Vec<String>| {
        if directives.get(dir).map(Vec::as_slice) != Some(&["'none'".to_owned()][..]) {
            problems.push(format!("{dir} deveria ser 'none'"));
        }
    };
    for d in ["object-src", "frame-src", "base-uri", "form-action"] {
        expect_none(d, &mut problems);
    }
    if directives.get("default-src").map(Vec::as_slice) != Some(&["'self'".to_owned()][..]) {
        problems.push("default-src deveria ser 'self'".into());
    }
    if directives.get("script-src").map(Vec::as_slice) != Some(&["'self'".to_owned()][..]) {
        problems.push("script-src deveria ser so 'self' (hash so com revisao da seguranca)".into());
    }
    problems
}

fn conf() -> Value {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tauri.conf.json");
    serde_json::from_str(&fs::read_to_string(path).unwrap()).unwrap()
}

#[test]
fn csp_de_release_do_projeto_esta_limpa() {
    let csp = conf()["app"]["security"]["csp"]
        .as_str()
        .expect("csp de release")
        .to_owned();
    let problems = check(&csp, false);
    assert!(problems.is_empty(), "CSP de release: {problems:#?}");
    for bad in ["unsafe-inline", "unsafe-eval"] {
        assert!(!csp.contains(bad), "SEC-01: {bad} na CSP de release");
    }
    assert!(!csp.contains("https://") && !csp.contains("ws://") && !csp.contains("wss://"));
}

#[test]
fn dev_csp_so_acrescenta_o_servidor_local_e_style_inline() {
    let c = conf();
    let dev = c["app"]["security"]["devCsp"]
        .as_str()
        .expect("devCsp")
        .to_owned();
    let problems = check(&dev, true);
    assert!(problems.is_empty(), "devCsp: {problems:#?}");
    assert!(!dev.contains("unsafe-eval"));
    // `devCsp` nunca vale como release: a de release e a outra chave e passa nas regras estritas.
    assert_ne!(dev, c["app"]["security"]["csp"].as_str().unwrap());
}

#[test]
fn verificador_reprova_csps_ruins() {
    let release = conf()["app"]["security"]["csp"]
        .as_str()
        .unwrap()
        .to_owned();
    let ruins: Vec<(String, &str)> = vec![
        (
            release.replace("script-src 'self'", "script-src 'self' 'unsafe-inline'"),
            "unsafe-inline em script-src",
        ),
        (
            release.replace("style-src 'self'", "style-src 'self' 'unsafe-inline'"),
            "unsafe-inline em style-src (so dev)",
        ),
        (
            release.replace("script-src 'self'", "script-src 'self' 'unsafe-eval'"),
            "unsafe-eval",
        ),
        (
            release.replace("script-src 'self'", "script-src 'self' 'wasm-unsafe-eval'"),
            "wasm-unsafe-eval",
        ),
        (
            release.replace(
                "connect-src ipc: http://ipc.localhost",
                "connect-src ipc: http://ipc.localhost https://example.com",
            ),
            "origem https externa",
        ),
        (
            release.replace(
                "connect-src ipc: http://ipc.localhost",
                "connect-src ipc: http://ipc.localhost ws://127.0.0.1:5173",
            ),
            "dev server na release",
        ),
        (
            release.replace(
                "connect-src ipc: http://ipc.localhost",
                "connect-src ipc: http://ipc.localhost wss://x.test",
            ),
            "wss externo",
        ),
        (
            release.replace("img-src 'self'", "img-src 'self' *"),
            "curinga",
        ),
        (
            release.replace("img-src 'self'", "img-src 'self' https:"),
            "esquema https inteiro",
        ),
        (
            release.replace("font-src 'self'", "font-src 'self' data:"),
            "data: fora de img-src",
        ),
        (
            release.replace("object-src 'none'", "object-src 'self'"),
            "object-src aberto",
        ),
        (
            release.replace("frame-src 'none'", "frame-src 'self'"),
            "frame-src aberto",
        ),
        (release.replace("base-uri 'none'", ""), "sem base-uri"),
        (
            release.replace("default-src 'self'", "default-src *"),
            "default-src aberto",
        ),
        (
            release.replace("script-src 'self'", "script-src 'self' 'sha256-abc='"),
            "hash sem revisao",
        ),
    ];
    for (csp, why) in &ruins {
        assert!(
            !check(csp, false).is_empty(),
            "o guard deveria reprovar: {why}\n{csp}"
        );
    }
    // Na dev, unsafe-eval e origem externa continuam proibidos.
    let dev = conf()["app"]["security"]["devCsp"]
        .as_str()
        .unwrap()
        .to_owned();
    assert!(!check(
        &dev.replace("script-src 'self'", "script-src 'self' 'unsafe-eval'"),
        true
    )
    .is_empty());
    assert!(!check(
        &dev.replace(
            "connect-src ipc:",
            "connect-src https://cdn.example.com ipc:"
        ),
        true
    )
    .is_empty());
    assert!(!check(
        &dev.replace("script-src 'self'", "script-src 'self' 'unsafe-inline'"),
        true
    )
    .is_empty());
}
