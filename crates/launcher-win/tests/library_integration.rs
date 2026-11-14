//! BE-29 e BE-30 com adapters reais no disco: cadastro manual pela `LibraryService` e o
//! inspetor de `.exe` de verdade (um teste por caso recusado) e a medida da varredura de 300
//! jogos (meta: < 500 ms em SSD).

use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU32, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;

use launcher_core::application::ports::{
    GameResolver, LibraryEvents, LibraryRepository, Loaded, StoreError, StoreScanner,
};
use launcher_core::application::{AppError, LibraryDeps, LibraryService};
use launcher_core::domain::library::{InstallPathPolicy, StoredLibrary};
use launcher_win::folder_browser::WinFolderBrowser;
use launcher_win::manual_exe::WinManualExeInspector;
use launcher_win::path_canon::{known, FsPathResolver};
use launcher_win::stores::registry::{Hive, RegistryReader};
use launcher_win::stores::{SteamScanner, StoreEnv};

struct TempDir(PathBuf);
impl TempDir {
    fn new(tag: &str) -> Self {
        static N: AtomicU32 = AtomicU32::new(0);
        let p = std::env::temp_dir().join(format!(
            "sofa-it-{tag}-{}-{}",
            std::process::id(),
            N.fetch_add(1, Ordering::SeqCst)
        ));
        fs::create_dir_all(&p).unwrap();
        Self(p)
    }
    fn text(&self) -> String {
        launcher_win::path_canon::canonicalize(self.0.to_str().unwrap()).unwrap()
    }
}
impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[derive(Default)]
struct Mem(Mutex<StoredLibrary>);
impl LibraryRepository for Mem {
    fn load(&self) -> Loaded<StoredLibrary> {
        Loaded {
            value: self.0.lock().unwrap().clone(),
            recovery: None,
            warnings: Vec::new(),
        }
    }
    fn save(&self, library: &StoredLibrary) -> Result<(), StoreError> {
        *self.0.lock().unwrap() = library.clone();
        Ok(())
    }
}

#[derive(Default)]
struct Events(AtomicUsize);
impl LibraryEvents for Events {
    fn changed(&self) {
        self.0.fetch_add(1, Ordering::SeqCst);
    }
}

fn service(scanners: Vec<Arc<dyn StoreScanner>>) -> (LibraryService, Arc<Mem>) {
    let mem = Arc::new(Mem::default());
    let svc = LibraryService::new(LibraryDeps {
        repo: mem.clone(),
        scanners,
        inspector: Arc::new(WinManualExeInspector),
        browser: Arc::new(WinFolderBrowser),
        events: Arc::new(Events::default()),
    });
    (svc, mem)
}

fn fake_pe() -> Vec<u8> {
    let mut b = vec![0u8; 0x100];
    b[0] = b'M';
    b[1] = b'Z';
    b[0x3C] = 0x80;
    b[0x80..0x84].copy_from_slice(b"PE\0\0");
    b
}

#[test]
fn cadastro_manual_no_disco_recusa_cada_caso_com_o_codigo_certo() {
    let t = TempDir::new("manual");
    let root = t.text();
    let w = |name: &str, bytes: &[u8]| -> String {
        let p = format!(r"{root}\{name}");
        fs::write(&p, bytes).unwrap();
        p
    };
    let (svc, mem) = service(Vec::new());

    // `.lnk`, `.bat`, `.cmd`, `.txt` e `.exe` forjado: recusados antes ou depois de ler o cabecalho.
    assert_eq!(
        svc.add_manual(&w("atalho.lnk", b"L")).unwrap_err(),
        AppError::NotAnExecutable
    );
    assert_eq!(
        svc.add_manual(&w("run.bat", b"@echo off")).unwrap_err(),
        AppError::NotAnExecutable
    );
    assert_eq!(
        svc.add_manual(&w("run.cmd", b"x")).unwrap_err(),
        AppError::NotAnExecutable
    );
    assert_eq!(
        svc.add_manual(&w("leia.txt", b"x")).unwrap_err(),
        AppError::NotAnExecutable
    );
    assert_eq!(
        svc.add_manual(&w("texto.exe", b"isto e um texto"))
            .unwrap_err(),
        AppError::NotAnExecutable
    );
    // ADS, UNC e prefixos especiais.
    let exe = w("jogo.exe", &fake_pe());
    assert_eq!(
        svc.add_manual(&format!("{exe}:ads")).unwrap_err(),
        AppError::NotAnExecutable
    );
    for p in [
        r"\\127.0.0.1\c$\Windows\notepad.exe",
        r"\\?\C:\Windows\notepad.exe",
        r"\\.\C:\x.exe",
    ] {
        assert_eq!(
            svc.add_manual(p).unwrap_err(),
            AppError::PathNotAllowed,
            "{p}"
        );
    }
    // Arquivo ausente e pasta com nome de `.exe`.
    assert_eq!(
        svc.add_manual(&format!(r"{root}\ausente.exe")).unwrap_err(),
        AppError::NotFound
    );
    fs::create_dir(format!(r"{root}\pasta.exe")).unwrap();
    assert_eq!(
        svc.add_manual(&format!(r"{root}\pasta.exe")).unwrap_err(),
        AppError::NotAnExecutable
    );
    assert!(mem.0.lock().unwrap().manual_games.is_empty());

    // Valido: nome do arquivo sem extensao, ID por hash do caminho canonico (CT-05).
    let e = svc.add_manual(&exe).unwrap();
    assert_eq!(e.display_name(), "jogo");
    // O mesmo arquivo em outra caixa e duplicata (CA-20, RN-03). Barra normal e recusada na
    // forma (o navegador so entrega a barra invertida).
    assert_eq!(
        svc.add_manual(&exe.to_uppercase()).unwrap_err(),
        AppError::Duplicate
    );
    assert_eq!(
        svc.add_manual(&exe.replace('\\', "/")).unwrap_err(),
        AppError::InvalidRequest
    );
    assert_eq!(mem.0.lock().unwrap().manual_games.len(), 1);
}

#[test]
fn manual_editado_a_mao_para_script_ou_rede_nao_inicia() {
    use launcher_core::domain::library::{GameId, ManualGameRecord};
    let mk = |path: &str| {
        let id = GameId::manual_from_canonical_path(path);
        let mem = Arc::new(Mem(Mutex::new(StoredLibrary {
            manual_games: vec![ManualGameRecord {
                id: id.clone(),
                name: "Jogo".into(),
                path: path.into(),
            }],
            ..StoredLibrary::default()
        })));
        let svc = LibraryService::new(LibraryDeps {
            repo: mem,
            scanners: Vec::new(),
            inspector: Arc::new(WinManualExeInspector),
            browser: Arc::new(WinFolderBrowser),
            events: Arc::new(Events::default()),
        });
        (svc, id)
    };
    let (svc, id) = mk(r"C:\Jogos\script.bat");
    assert!(matches!(
        svc.resolve(&id).unwrap_err(),
        AppError::GameFileMissing { .. }
    ));
    let (svc, id) = mk(r"\\192.168.0.50\share\jogo.exe");
    assert_eq!(svc.resolve(&id).unwrap_err(), AppError::NetworkPathBlocked);
    let (svc, id) = mk(r"C:\Jogos\jogo.exe:ads");
    assert!(matches!(
        svc.resolve(&id).unwrap_err(),
        AppError::GameFileMissing { .. }
    ));
}

struct OneKey(String);
impl RegistryReader for OneKey {
    fn read_string(&self, hive: Hive, subkey: &str, value: &str) -> Option<String> {
        (hive == Hive::CurrentUser && subkey == r"Software\Valve\Steam" && value == "SteamPath")
            .then(|| self.0.clone())
    }
    fn subkeys(&self, _: Hive, _: &str) -> Vec<String> {
        Vec::new()
    }
}

#[test]
fn varredura_de_300_jogos_steam_leva_menos_de_500_ms() {
    let t = TempDir::new("perf");
    let steam = format!(r"{}\Steam", t.text());
    let apps = format!(r"{steam}\steamapps");
    fs::create_dir_all(format!(r"{apps}\common")).unwrap();
    fs::write(
        format!(r"{apps}\libraryfolders.vdf"),
        format!(
            "\"libraryfolders\"\n{{\n\t\"0\"\n\t{{\n\t\t\"path\"\t\t\"{}\"\n\t}}\n}}\n",
            steam.replace('\\', "\\\\")
        ),
    )
    .unwrap();
    for i in 1..=300u32 {
        fs::create_dir_all(format!(r"{apps}\common\Jogo {i}")).unwrap();
        fs::write(
            format!(r"{apps}\appmanifest_{i}.acf"),
            format!(
                "\"AppState\"\n{{\n\t\"appid\"\t\t\"{i}\"\n\t\"name\"\t\t\"Jogo {i}\"\n\t\"StateFlags\"\t\t\"4\"\n\t\"installdir\"\t\t\"Jogo {i}\"\n}}\n"
            ),
        )
        .unwrap();
    }
    let root = known::windows_dir().unwrap();
    let env = Arc::new(StoreEnv::new(
        InstallPathPolicy::new(&root, Vec::new()),
        Arc::new(FsPathResolver),
    ));
    let scanner: Arc<dyn StoreScanner> = Arc::new(SteamScanner::new(
        Arc::new(OneKey(steam.to_lowercase().replace('\\', "/"))),
        env,
    ));
    let (svc, _) = service(vec![scanner]);
    let t0 = Instant::now();
    svc.refresh();
    let elapsed = t0.elapsed();
    println!("varredura de 300 jogos: {} ms", elapsed.as_millis());
    assert_eq!(svc.snapshot().visible().count(), 300);
    assert!(
        elapsed.as_millis() < 500,
        "varredura levou {} ms",
        elapsed.as_millis()
    );
}
