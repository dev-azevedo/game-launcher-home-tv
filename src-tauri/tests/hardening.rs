//! BE-09: politica de navegacao (SEC-02), manifesto embutido `asInvoker` (SEC-20, INV-06) e
//! `SetDefaultDllDirectories` como 1a instrucao do `main` (SEC-23).

use std::fs;
use std::path::PathBuf;

use sofa_lib::webview_hardening::NavigationPolicy;
use tauri::Url;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn url(s: &str) -> Url {
    Url::parse(s).unwrap_or_else(|e| panic!("{s}: {e}"))
}

#[test]
fn release_so_navega_para_a_origem_do_app() {
    let p = NavigationPolicy::new(&[NavigationPolicy::APP_ORIGIN]);
    for ok in [
        "http://tauri.localhost/",
        "http://tauri.localhost/index.html",
        "http://tauri.localhost/index.html#/biblioteca?x=1",
    ] {
        assert!(p.allows(&url(ok)), "{ok}");
    }
    for bad in [
        "https://example.com/",
        "http://example.com/",
        "https://tauri.localhost/",         // esquema diferente
        "http://tauri.localhost:8080/",     // porta diferente
        "http://tauri.localhost.evil.com/", // subdominio de terceiros
        "http://evil.com/tauri.localhost",
        "http://tauri.localhost@evil.com/", // userinfo: host real e evil.com
        "http://evil.com@tauri.localhost/", // userinfo com host certo: tambem recusado
        "http://127.0.0.1:5173/",           // dev server nao vale no release
        "file:///C:/Windows/win.ini",
        "javascript:alert(1)",
        "data:text/html,<script>alert(1)</script>",
        "about:blank",
        "blob:http://tauri.localhost/abc",
        "ftp://tauri.localhost/",
        "cover://steam:620",
    ] {
        assert!(!p.allows(&url(bad)), "devia recusar {bad}");
    }
}

#[test]
fn debug_acrescenta_so_o_dev_server() {
    let p = NavigationPolicy::new(&[NavigationPolicy::APP_ORIGIN, NavigationPolicy::DEV_ORIGIN]);
    assert!(p.allows(&url("http://127.0.0.1:5173/")));
    assert!(!p.allows(&url("http://127.0.0.1:5174/")));
    assert!(!p.allows(&url("http://localhost:5173/")));
    assert!(!p.allows(&url("https://example.com/")));
}

#[test]
fn politica_da_build_atual_nao_aceita_externo() {
    let p = NavigationPolicy::for_build();
    assert!(p.allows(&url(NavigationPolicy::APP_ORIGIN)));
    assert!(!p.allows(&url("https://example.com/")));
}

#[test]
fn manifesto_do_repo_e_as_invoker_sem_ui_access() {
    let m = fs::read_to_string(root().join("sofa.exe.manifest")).unwrap();
    assert!(m.contains(r#"requestedExecutionLevel level="asInvoker" uiAccess="false""#));
    for bad in [
        "requireAdministrator",
        "highestAvailable",
        r#"uiAccess="true""#,
    ] {
        assert!(!m.contains(bad), "SEC-20: {bad}");
    }
    assert!(
        m.contains("Microsoft.Windows.Common-Controls"),
        "dialogos do Windows precisam da v6"
    );
}

/// O manifesto esta de fato dentro do `sofa.exe` (recurso `RT_MANIFEST` em texto UTF-8).
#[test]
fn manifesto_embutido_no_exe_e_as_invoker() {
    let exe = fs::read(env!("CARGO_BIN_EXE_sofa")).unwrap();
    // Busca de substring do `str` (rapida); o recurso do manifesto e ASCII/UTF-8.
    let text = String::from_utf8_lossy(&exe);
    let find = |needle: &str| text.contains(needle);
    assert!(
        find(r#"requestedExecutionLevel level="asInvoker" uiAccess="false""#),
        "manifesto asInvoker nao encontrado no sofa.exe (SEC-20, INV-06)"
    );
    assert!(!find("requireAdministrator"));
    assert!(!find(r#"uiAccess="true""#));
}

#[test]
fn set_default_dll_directories_e_a_primeira_instrucao_do_main() {
    let main = fs::read_to_string(root().join("src/main.rs")).unwrap();
    let body = main.split("fn main()").nth(1).expect("fn main");
    let first = body
        .lines()
        .map(str::trim)
        .find(|l| !l.is_empty() && !l.starts_with("//") && *l != "{")
        .expect("corpo do main");
    assert_eq!(
        first, "launcher_win::hardening::set_default_dll_directories();",
        "SEC-23: a 1a instrucao do main trava a busca de DLL"
    );
}

#[test]
fn release_sem_console_e_sem_devtools_no_tauri() {
    let main = fs::read_to_string(root().join("src/main.rs")).unwrap();
    assert!(main.contains(r#"windows_subsystem = "windows""#));
    let cargo = fs::read_to_string(root().join("Cargo.toml")).unwrap();
    assert!(
        !cargo.contains("devtools"),
        "SEC-02: sem a feature devtools do Tauri"
    );
}

#[test]
fn nenhum_codigo_do_app_chama_load_library_relativo() {
    for entry in fs::read_dir(root().join("src")).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().is_some_and(|e| e == "rs") {
            let text = fs::read_to_string(&path).unwrap();
            assert!(!text.contains("LoadLibrary"), "{}", path.display());
        }
    }
}
