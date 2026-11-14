// Fonte unica da lista de comandos: `command_list.rs` (tambem usada pelo crate e pelos testes).
include!("command_list.rs");

fn main() {
    // CT-08: sem o AppManifest com os comandos, o Tauri 2 libera comandos da app sem permissao
    // explicita e a capability `main` perde efeito (SEC-05). Cada nome gera `allow-<nome>`.
    let attributes = tauri_build::Attributes::new()
        .app_manifest(tauri_build::AppManifest::new().commands(COMMAND_NAMES));
    tauri_build::try_build(attributes).expect("falha ao gerar o manifesto de permissoes do Sofa");

    // Os binarios de teste (runtime de teste do Tauri) importam `TaskDialogIndirect`, que so
    // existe na Common Controls v6: sem manifesto o teste nem carrega (0xC0000139). So testes.
    println!("cargo:rerun-if-changed=test-manifest.rc");
    println!("cargo:rerun-if-changed=test.manifest");
    embed_resource::compile_for_tests("test-manifest.rc", embed_resource::NONE)
        .manifest_required()
        .expect("falha ao embutir o manifesto dos testes");
}
