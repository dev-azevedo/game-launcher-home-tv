//! Metadados de versao e manifesto so para o jogo sintetico dos testes (`sofa-test-game`, BE-52):
//! o Defender barra, como falso positivo de aprendizado de maquina, o executavel pequeno e sem
//! nenhuma informacao de versao. O `launcher-win` em si (lib) nao leva recurso nenhum.

fn main() {
    println!("cargo:rerun-if-changed=test-game.rc");
    println!("cargo:rerun-if-changed=test-game.manifest");
    println!("cargo:rerun-if-changed=build.rs");
    embed_resource::compile_for("test-game.rc", ["sofa-test-game"], embed_resource::NONE)
        .manifest_required()
        .expect("falha ao embutir a versao e o manifesto do jogo de teste");
}
