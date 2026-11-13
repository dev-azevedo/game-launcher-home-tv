//! Guards de BE-06: INV-01 (nucleo sem Tauri/Windows/I-O) e SEC-24 (piso do toolchain).
//! Leem so os Cargo.toml do repo; nenhum I/O de producao.

use std::fs;
use std::path::PathBuf;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
}

/// Nomes de dependencias declaradas numa secao `[dependencies]`-like.
fn dependency_names(manifest: &str) -> Vec<String> {
    let mut in_deps = false;
    let mut names = Vec::new();
    for line in manifest.lines() {
        let t = line.trim();
        if t.starts_with('[') {
            in_deps = t.contains("dependencies");
            continue;
        }
        if in_deps && !t.is_empty() && !t.starts_with('#') {
            if let Some((name, _)) = t.split_once('=') {
                names.push(name.trim().trim_end_matches(".workspace").to_string());
            }
        }
    }
    names
}

#[test]
fn inv01_launcher_core_nao_depende_de_tauri_nem_windows() {
    let manifest =
        fs::read_to_string(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml")).unwrap();
    for dep in dependency_names(&manifest) {
        let d = dep.to_lowercase();
        assert!(
            !d.starts_with("tauri") && !d.starts_with("windows") && d != "winapi",
            "INV-01: launcher-core nao pode depender de `{dep}`"
        );
    }
}

#[test]
fn inv01_launcher_win_depende_do_core_e_nao_o_contrario() {
    let win = fs::read_to_string(repo_root().join("crates/launcher-win/Cargo.toml")).unwrap();
    assert!(dependency_names(&win).iter().any(|d| d == "launcher-core"));
    let core = fs::read_to_string(repo_root().join("crates/launcher-core/Cargo.toml")).unwrap();
    assert!(!dependency_names(&core)
        .iter()
        .any(|d| d.starts_with("launcher-")));
}

#[test]
fn sec24_rust_version_e_toolchain_no_piso_1_77_2() {
    fn parse(v: &str) -> (u32, u32, u32) {
        let mut it = v
            .trim()
            .trim_matches('"')
            .split('.')
            .map(|p| p.parse::<u32>().unwrap());
        (
            it.next().unwrap(),
            it.next().unwrap_or(0),
            it.next().unwrap_or(0),
        )
    }
    let floor = (1, 77, 2);
    let ws = fs::read_to_string(repo_root().join("Cargo.toml")).unwrap();
    let rv = ws
        .lines()
        .find_map(|l| l.trim().strip_prefix("rust-version"))
        .and_then(|l| l.split('=').nth(1))
        .map(|l| l.split('#').next().unwrap().to_string())
        .expect("rust-version no workspace");
    assert!(parse(&rv) >= floor, "rust-version {rv} abaixo de 1.77.2");

    let tc = fs::read_to_string(repo_root().join("rust-toolchain.toml")).unwrap();
    let ch = tc
        .lines()
        .find_map(|l| l.trim().strip_prefix("channel"))
        .and_then(|l| l.split('=').nth(1))
        .expect("channel no rust-toolchain.toml");
    assert!(parse(ch) >= floor, "toolchain {ch} abaixo de 1.77.2");
}
