//! Composition root do Sofa. Casos de uso e adapters sao ligados aqui (BE-08 em diante).

pub fn run() {
    tauri::Builder::default()
        .run(tauri::generate_context!())
        .expect("falha ao iniciar o Sofa");
}
