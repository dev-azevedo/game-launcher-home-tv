//! D-49, ADR-0007 (BE-70): o titulo da janela e o nome visivel, SofaLauncher, e nenhum texto de
//! `src-tauri/src` mostra o nome antigo ao usuario.

use std::fs;
use std::path::{Path, PathBuf};

use sofa_lib::window::MAIN_TITLE;

fn src_files(dir: &Path, out: &mut Vec<PathBuf>) {
    for e in fs::read_dir(dir).unwrap().flatten() {
        let p = e.path();
        if p.is_dir() {
            src_files(&p, out);
        } else if p.extension().is_some_and(|x| x == "rs") {
            out.push(p);
        }
    }
}

#[test]
fn titulo_da_janela_e_sofalauncher_e_vem_da_constante() {
    assert_eq!(MAIN_TITLE, "SofaLauncher");
    let src =
        fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("src/window.rs")).unwrap();
    assert!(
        src.contains(".title(MAIN_TITLE)"),
        "a janela usa MAIN_TITLE"
    );
}

/// Literais de texto com o nome antigo ("Sofa" ou "Sofa" com acento como palavra do produto) nao
/// existem em `src`; os identificadores tecnicos (`sofa`, `app.sofa.launcher`, `Sofa` do Run) ficam.
#[test]
fn nenhum_literal_de_src_mostra_o_nome_antigo() {
    let mut files = Vec::new();
    src_files(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("src"),
        &mut files,
    );
    assert!(!files.is_empty());
    for f in files {
        let text = fs::read_to_string(&f).unwrap();
        assert!(
            !text.contains("Sof\u{e1}"),
            "{}: nome antigo com acento em texto",
            f.display()
        );
        for (n, line) in text.lines().enumerate() {
            let code = line.split("//").next().unwrap_or("");
            assert!(
                !(code.contains(".title(\"Sofa\")") || code.contains("title: \"Sofa\"")),
                "{}:{}: titulo com o nome antigo",
                f.display(),
                n + 1
            );
        }
    }
}
