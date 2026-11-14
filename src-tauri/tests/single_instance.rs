//! BE-35 / SEC-19: a segunda instancia so traz a janela. O callback descarta `argv` e `cwd`
//! (conferido no codigo) e `bring_launcher_front` nao le nada de fora.

use tauri::test::{mock_builder, mock_context, noop_assets};
use tauri::{Manager, WebviewWindowBuilder};

fn app() -> tauri::App<tauri::test::MockRuntime> {
    sofa_lib::configure(mock_builder())
        .build(mock_context(noop_assets()))
        .expect("app de teste")
}

/// O callback do plugin nomeia `argv` e `cwd` com `_` e nenhuma linha de codigo (fora comentario)
/// usa o nome sem sublinhado: nao ha como a linha de comando de outro processo virar acao.
#[test]
fn callback_descarta_argv_e_cwd() {
    let src = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/single_instance.rs"
    ))
    .expect("single_instance.rs");
    assert!(
        src.contains(
            "tauri_plugin_single_instance::init(|app, _argv, _cwd| bring_launcher_front(app))"
        ),
        "callback fora do formato esperado"
    );
    for (n, line) in src.lines().enumerate() {
        let t = line.trim_start();
        if t.starts_with("//") {
            continue;
        }
        let sem_sublinhado = line.replace("_argv", "").replace("_cwd", "");
        assert!(
            !sem_sublinhado.contains("argv") && !sem_sublinhado.contains("cwd"),
            "linha {}: {line}",
            n + 1
        );
    }
    // Nenhum esquema de URI nem repasse de argumentos no resto do app.
    let lib = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/lib.rs")).unwrap();
    assert!(!lib.contains("deep-link") && !lib.contains("deep_link"));
}

/// O runtime de teste nao simula visibilidade (`is_visible` e sempre verdadeiro); o que se confere
/// aqui e que, com a janela `main` e sem sessao, trazer a janela roda sem panico e sem efeito
/// colateral. O comportamento real (janela oculta pelo jogo) fica no teste manual do BE-35.
#[test]
fn trazer_a_janela_sem_sessao_nao_tem_efeito_colateral() {
    let app = app();
    let window = WebviewWindowBuilder::new(&app, "main", Default::default())
        .build()
        .expect("janela de teste");
    sofa_lib::single_instance::bring_launcher_front(app.handle());
    sofa_lib::single_instance::bring_launcher_front(app.handle());
    assert!(window.is_visible().unwrap());
    assert!(app
        .state::<sofa_lib::state::AppState>()
        .session
        .get()
        .is_none());
}

#[test]
fn sem_janela_main_nao_faz_nada() {
    let app = app();
    sofa_lib::single_instance::bring_launcher_front(app.handle());
}
