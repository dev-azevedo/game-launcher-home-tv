//! BE-41 / SEC-31 / INV-02: o modo `--purge-user-data` existe so como argumento do proprio
//! processo. Nao e comando IPC, nao esta na capability, e o `main` o trata antes de criar Tauri,
//! WebView2 e instancia unica. Nada aqui apaga a pasta de dados real: so a recusa por argumento
//! extra roda como processo.

use std::process::Command;

fn read(rel: &str) -> String {
    std::fs::read_to_string(format!("{}/{rel}", env!("CARGO_MANIFEST_DIR"))).expect(rel)
}

#[test]
fn identificador_do_dominio_e_o_do_tauri_conf() {
    let conf: serde_json::Value = serde_json::from_str(&read("tauri.conf.json")).unwrap();
    assert_eq!(
        conf["identifier"],
        launcher_core::domain::data_dir_policy::APP_IDENTIFIER,
        "SEC-25: o alvo da remocao usa o identificador fixo do build"
    );
}

#[test]
fn purge_nao_e_comando_nem_permissao_nem_handler() {
    for (name, text) in [
        ("command_list.rs", read("command_list.rs")),
        ("capabilities/main.json", read("capabilities/main.json")),
        ("src/lib.rs (handlers)", {
            let lib = read("src/lib.rs");
            lib.split("generate_handler![")
                .nth(1)
                .and_then(|s| s.split(']').next())
                .unwrap()
                .to_owned()
        }),
    ] {
        assert!(
            !text.to_lowercase().contains("purge"),
            "{name}: o modo de remocao nunca e exposto por IPC (INV-02)"
        );
    }
}

/// O `main` desvia para a remocao **antes** de qualquer coisa do Tauri e a recusa por argumento
/// extra nao passa pela instancia unica.
#[test]
fn main_trata_o_modo_antes_do_tauri() {
    let main = read("src/main.rs");
    let purge = main.find("purge::run_main").expect("main chama a remocao");
    let run = main.find("sofa_lib::run(").expect("main chama o app");
    assert!(purge < run);
    let first_call = main.find("set_default_dll_directories").unwrap();
    assert!(first_call < purge, "SEC-23: a trava de DLL vem primeiro");
}

/// `--purge-user-data` com qualquer outro argumento: sai com 2, sem janela e sem apagar nada. Roda
/// o binario de verdade, nas duas ordens.
#[test]
fn argumento_extra_sai_com_codigo_2_sem_abrir_nada() {
    for args in [
        &["--purge-user-data", "extra"][..],
        &["extra", "--purge-user-data"],
        &["--purge-user-data", "--autostart"],
        &["--purge-user-data", r"C:\Windows\System32\calc.exe"],
    ] {
        let out = Command::new(env!("CARGO_BIN_EXE_sofa"))
            .args(args)
            .output()
            .expect("rodar sofa.exe");
        assert_eq!(out.status.code(), Some(2), "{args:?}");
    }
}
