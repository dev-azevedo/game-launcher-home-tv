//! BE-41 no disco de verdade (`--features win-integration`), so em pastas temporarias: a pasta de
//! dados **real** do usuario nunca e tocada (o alvo vem de `run_in(<pasta temporaria>, ...)`).
//!
//! Cobre: junction estatica dentro da arvore, junction trocada em loop durante a remocao (a isca
//! tem que ficar intacta), alvo que e junction, pasta sem `library.json`/`settings.json`, arquivo em
//! uso (remocao parcial, codigo 1) e os codigos de saida.
#![cfg(feature = "win-integration")]

use std::fs::{self, OpenOptions};
use std::os::windows::fs::OpenOptionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

use launcher_core::domain::data_dir_policy::PurgeExit;
use launcher_win::purge::{purge_tree, run_in};

const ID: &str = "app.sofa.launcher";

struct Temp(PathBuf);
impl Temp {
    fn new(tag: &str) -> Self {
        static N: AtomicU32 = AtomicU32::new(0);
        let p = std::env::temp_dir().join(format!(
            "sofa-purge-it-{tag}-{}-{}",
            std::process::id(),
            N.fetch_add(1, Ordering::SeqCst)
        ));
        fs::create_dir_all(&p).unwrap();
        Self(p)
    }
}
impl Drop for Temp {
    fn drop(&mut self) {
        // Junctions que sobrarem sao removidas como link (remove_dir) antes da limpeza do resto.
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn junction(link: &Path, target: &Path) -> bool {
    Command::new("cmd")
        .args(["/c", "mklink", "/J"])
        .arg(link)
        .arg(target)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Isca com 50 arquivos fora da pasta de dados.
fn decoy(parent: &Path) -> PathBuf {
    let d = parent.join("isca-so-administradores");
    fs::create_dir_all(d.join("sub")).unwrap();
    for i in 0..50 {
        fs::write(d.join(format!("importante-{i}.txt")), b"nao apague").unwrap();
    }
    fs::write(d.join("sub").join("fundo.txt"), b"nao apague").unwrap();
    d
}

fn decoy_intact(d: &Path) -> bool {
    (0..50).all(|i| d.join(format!("importante-{i}.txt")).is_file())
        && d.join("sub/fundo.txt").is_file()
}

fn data_tree(root: &Path, covers: usize) {
    fs::create_dir_all(root.join("cache/covers")).unwrap();
    fs::create_dir_all(root.join("cache/icons")).unwrap();
    fs::create_dir_all(root.join("logs")).unwrap();
    fs::create_dir_all(root.join("EBWebView/Default/Cache")).unwrap();
    fs::write(root.join("library.json"), b"{}").unwrap();
    fs::write(root.join("settings.json"), b"{}").unwrap();
    for i in 0..covers {
        fs::write(root.join("cache/covers").join(format!("c{i}.jpg")), b"x").unwrap();
    }
    for i in 0..50 {
        fs::write(root.join("logs").join(format!("l{i}.log")), b"x").unwrap();
        fs::write(
            root.join("EBWebView/Default/Cache").join(format!("f{i}")),
            b"x",
        )
        .unwrap();
    }
}

#[test]
fn junction_dentro_da_arvore_e_removida_sem_descer() {
    let t = Temp::new("estatica");
    let d = decoy(&t.0);
    let root = t.0.join(ID);
    data_tree(&root, 20);
    assert!(
        junction(&root.join("cache").join("atalho"), &d),
        "mklink /J"
    );
    assert!(
        root.join("cache/atalho/importante-0.txt").exists(),
        "a junction funciona"
    );

    let code = run_in(&t.0.display().to_string(), ID, false);
    assert_eq!(code, PurgeExit::Done);
    assert!(!root.exists(), "pasta de dados removida");
    assert!(
        decoy_intact(&d),
        "isca intacta: a remocao nao desceu na junction"
    );
}

#[test]
fn contagem_de_links_no_relatorio() {
    let t = Temp::new("relatorio");
    let d = decoy(&t.0);
    let root = t.0.join("dados");
    data_tree(&root, 5);
    assert!(junction(&root.join("logs").join("j1"), &d));
    assert!(junction(&root.join("cache").join("j2"), &d));
    let r = purge_tree(&root);
    assert!(r.failures.is_empty(), "{r:?}");
    assert_eq!(r.links, 2);
    assert!(decoy_intact(&d));
    assert!(!root.exists());
}

#[test]
fn alvo_que_e_junction_e_recusado_e_a_isca_fica() {
    let t = Temp::new("alvo-link");
    let d = decoy(&t.0);
    fs::write(d.join("library.json"), b"{}").unwrap(); // a isca ate "parece" dados do Sofa
    assert!(junction(&t.0.join(ID), &d));
    let code = run_in(&t.0.display().to_string(), ID, false);
    assert_eq!(code, PurgeExit::InvalidTarget);
    assert!(decoy_intact(&d));
    assert!(d.join("library.json").exists());
}

#[test]
fn sem_marcador_nada_e_apagado_e_elevado_recusa() {
    let t = Temp::new("sem-marca");
    let root = t.0.join(ID);
    fs::create_dir_all(root.join("cache")).unwrap();
    fs::write(root.join("cache/x.bin"), b"x").unwrap();
    assert_eq!(
        run_in(&t.0.display().to_string(), ID, false),
        PurgeExit::InvalidTarget
    );
    assert!(root.join("cache/x.bin").exists());

    fs::write(root.join("library.json"), b"{}").unwrap();
    assert_eq!(
        run_in(&t.0.display().to_string(), ID, true),
        PurgeExit::Elevated
    );
    assert!(root.join("library.json").exists());
    assert_eq!(
        run_in(&t.0.display().to_string(), ID, false),
        PurgeExit::Done
    );
    assert!(!root.exists());
}

/// WebView2 aberto segura arquivos de `EBWebView\`: o resto sai, o arquivo em uso fica e o codigo
/// de saida e 1. Depois de soltar o arquivo, uma nova rodada conclui (codigo 0).
#[test]
fn arquivo_em_uso_da_remocao_parcial() {
    let t = Temp::new("parcial");
    let root = t.0.join(ID);
    data_tree(&root, 10);
    let locked = root.join("EBWebView/Default/Cache/travado.db");
    fs::write(&locked, b"em uso").unwrap();
    let hold = OpenOptions::new()
        .read(true)
        .write(true)
        .share_mode(0) // nenhum compartilhamento: ninguem apaga nem abre
        .open(&locked)
        .unwrap();

    assert_eq!(
        run_in(&t.0.display().to_string(), ID, false),
        PurgeExit::Partial
    );
    assert!(locked.exists(), "o arquivo em uso ficou");
    assert!(!root.join("cache").exists(), "o que nao estava em uso saiu");
    assert!(!root.join("logs").exists());
    let log = fs::read_to_string(std::env::temp_dir().join("sofa-purge.log")).unwrap();
    assert!(log.starts_with("parcial:"), "{log}");
    assert!(log.contains("travado.db"), "{log}");

    // So o arquivo em uso (e as pastas que o contem) ficou; nenhum outro arquivo sobrou.
    let mut left = Vec::new();
    fn walk(p: &Path, out: &mut Vec<String>) {
        for e in fs::read_dir(p).unwrap().flatten() {
            if e.path().is_dir() {
                walk(&e.path(), out);
            } else {
                out.push(e.file_name().to_string_lossy().into_owned());
            }
        }
    }
    walk(&root, &mut left);
    assert_eq!(left, vec!["travado.db".to_owned()]);

    // Sem os arquivos-marca (ja removidos), uma segunda rodada nao tem como provar que a pasta e
    // do Sofa: recusa (SEC-25) em vez de apagar o resto. O desinstalador trata o codigo 1 com o
    // plano de nomes conhecidos e informa o resto.
    drop(hold);
    assert_eq!(
        run_in(&t.0.display().to_string(), ID, false),
        PurgeExit::InvalidTarget
    );
    assert!(locked.exists());
}

/// A corrida do SEC-31: uma thread troca `cache\covers` por uma junction para a isca em loop
/// enquanto a remocao roda. Nenhum arquivo da isca pode se perder.
#[test]
fn junction_trocada_em_loop_durante_a_remocao_deixa_a_isca_intacta() {
    let swaps = Arc::new(AtomicUsize::new(0));
    let mut link_deleted = 0u64;
    for round in 0..8 {
        let t = Temp::new(&format!("corrida{round}"));
        let d = decoy(&t.0);
        let root = t.0.join(ID);
        data_tree(&root, 1500);

        let stop = Arc::new(AtomicBool::new(false));
        let swapper = {
            let (stop, swaps) = (Arc::clone(&stop), Arc::clone(&swaps));
            let covers = root.join("cache").join("covers");
            let away = root.join("cache").join("covers.away");
            let d = d.clone();
            std::thread::spawn(move || {
                while !stop.load(Ordering::Relaxed) {
                    if fs::rename(&covers, &away).is_ok() {
                        if junction(&covers, &d) {
                            swaps.fetch_add(1, Ordering::Relaxed);
                            std::thread::sleep(Duration::from_millis(2));
                            let _ = fs::remove_dir(&covers); // remove so o link
                        }
                        let _ = fs::rename(&away, &covers);
                    }
                    std::thread::sleep(Duration::from_millis(1));
                }
            })
        };
        std::thread::sleep(Duration::from_millis(30)); // deixa a troca comecar
        let report = purge_tree(&root);
        link_deleted += report.links;
        stop.store(true, Ordering::Relaxed);
        swapper.join().unwrap();

        assert!(
            decoy_intact(&d),
            "rodada {round}: arquivo da isca perdido! {report:?}"
        );
        // O resto da arvore: o que sobrou e so efeito da propria troca, nunca da isca.
        let _ = fs::remove_dir_all(&root);
    }
    eprintln!(
        "trocas por junction conseguidas: {}; links apagados como link: {link_deleted}",
        swaps.load(Ordering::Relaxed)
    );
}
