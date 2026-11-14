//! BE-29, BE-30 e BE-28 pelo caminho real de IPC do Tauri (runtime de teste): comandos de
//! biblioteca com a `LibraryService` de verdade e adapters de mentira, o resultado em
//! `library://changed` e o protocolo `cover://` (SEC-08, CT-06).

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use launcher_core::application::ports::{
    CoverCache, CoverImage, CoverSource, FolderBrowser, LibraryEvents, LibraryRepository, Loaded,
    ManualExeInspector, NullScanCache, ScanError, StoreError, StoreScanner,
};
use launcher_core::application::{AppError, LibraryDeps, LibraryService};
use launcher_core::domain::library::{
    dir_listing, BrowseTarget, DetectedGame, DirListing, GameId, LaunchSpec, Origin, RawEntry,
    StoredLibrary,
};
use serde_json::{json, Value};
use sofa_lib::state::AppState;
use tauri::http::Request;
use tauri::ipc::{CallbackFn, InvokeBody};
use tauri::test::MockRuntime;
use tauri::test::{get_ipc_response, mock_builder, mock_context, noop_assets, INVOKE_KEY};
use tauri::webview::InvokeRequest;
use tauri::{App, Manager, WebviewWindow, WebviewWindowBuilder};

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

struct Scanner(Origin, Result<Vec<DetectedGame>, ScanError>, bool);
impl StoreScanner for Scanner {
    fn origin(&self) -> Origin {
        self.0
    }
    fn scan(&self) -> Result<Vec<DetectedGame>, ScanError> {
        assert!(!self.2, "varredura com defeito");
        self.1.clone()
    }
}

#[derive(Default)]
struct Inspector(Mutex<Option<AppError>>);
impl ManualExeInspector for Inspector {
    fn inspect(&self, path: &str) -> Result<String, AppError> {
        match self.0.lock().unwrap().clone() {
            Some(e) => Err(e),
            None => Ok(path.to_owned()),
        }
    }
}

struct Browser;
impl FolderBrowser for Browser {
    fn list(&self, target: &BrowseTarget) -> Result<DirListing, AppError> {
        match target {
            BrowseTarget::Drives => Ok(dir_listing(r"C:\", Vec::new())),
            BrowseTarget::Dir(p) => Ok(dir_listing(
                p,
                vec![
                    RawEntry {
                        name: "Jogo".into(),
                        is_dir: true,
                    },
                    RawEntry {
                        name: "a.exe".into(),
                        is_dir: false,
                    },
                    RawEntry {
                        name: "x.txt".into(),
                        is_dir: false,
                    },
                ],
            )),
        }
    }
}

#[derive(Default)]
struct Events(AtomicUsize);
impl LibraryEvents for Events {
    fn changed(&self) {
        self.0.fetch_add(1, Ordering::SeqCst);
    }
}

/// So tem imagem para a capa `C:\capa.jpg`.
struct Covers;
impl CoverCache for Covers {
    fn image(&self, source: &CoverSource) -> Option<CoverImage> {
        (source.cover.as_deref() == Some(r"C:\capa.jpg")).then(|| CoverImage {
            mime: "image/jpeg",
            bytes: vec![0xFF, 0xD8, 0xFF, 0xD9],
        })
    }
}

fn detected(game: &str, name: &str, cover: Option<&str>) -> DetectedGame {
    let id = GameId::parse(game).unwrap();
    DetectedGame {
        launch: LaunchSpec::SteamUri {
            app_id: id.key().parse().unwrap_or(1),
        },
        id,
        name: name.into(),
        install_dir: Some(r"C:\Jogos\X".into()),
        cover_source: cover.map(str::to_owned),
        icon_source: None,
    }
}

struct Harness {
    app: App<MockRuntime>,
    webview: WebviewWindow<MockRuntime>,
    events: Arc<Events>,
    inspector: Arc<Inspector>,
}

fn harness(scanners: Vec<Scanner>) -> Harness {
    let app = sofa_lib::configure(mock_builder())
        .build(mock_context(noop_assets()))
        .expect("app de teste");
    let webview = WebviewWindowBuilder::new(&app, "main", Default::default())
        .build()
        .expect("janela de teste");
    let events = Arc::new(Events::default());
    let inspector = Arc::new(Inspector::default());
    let library = Arc::new(LibraryService::new(LibraryDeps {
        repo: Arc::new(Mem::default()),
        scanners: scanners
            .into_iter()
            .map(|s| Arc::new(s) as Arc<dyn StoreScanner>)
            .collect(),
        inspector: inspector.clone(),
        browser: Arc::new(Browser),
        events: events.clone(),
        scan_cache: Arc::new(NullScanCache),
    }));
    let state = app.state::<AppState>();
    assert!(state.library.set(library).is_ok());
    assert!(state.covers.set(Arc::new(Covers)).is_ok());
    Harness {
        app,
        webview,
        events,
        inspector,
    }
}

impl Harness {
    fn call(&self, cmd: &str, body: Value) -> Result<Value, Value> {
        get_ipc_response(
            &self.webview,
            InvokeRequest {
                cmd: cmd.into(),
                callback: CallbackFn(0),
                error: CallbackFn(1),
                url: "http://tauri.localhost".parse().unwrap(),
                body: InvokeBody::Json(body),
                headers: Default::default(),
                invoke_key: INVOKE_KEY.to_string(),
            },
        )
        .map(|b| b.deserialize::<Value>().expect("resposta JSON"))
    }

    fn code(&self, cmd: &str, body: Value) -> String {
        let err = self.call(cmd, body).expect_err("devia rejeitar");
        err["code"]
            .as_str()
            .unwrap_or_else(|| panic!("sem code: {err}"))
            .to_owned()
    }

    fn names(&self) -> Vec<String> {
        self.call("library_get", json!({})).unwrap()["games"]
            .as_array()
            .unwrap()
            .iter()
            .map(|g| g["name"].as_str().unwrap().to_owned())
            .collect()
    }

    fn wait_events(&self, n: usize) {
        let t0 = Instant::now();
        while self.events.0.load(Ordering::SeqCst) < n {
            assert!(
                t0.elapsed() < Duration::from_secs(5),
                "library://changed nao chegou"
            );
            std::thread::sleep(Duration::from_millis(10));
        }
    }
}

fn two_stores() -> Vec<Scanner> {
    vec![
        Scanner(
            Origin::Steam,
            Ok(vec![detected(
                "steam:620",
                "Portal 2",
                Some(r"C:\capa.jpg"),
            )]),
            false,
        ),
        Scanner(
            Origin::Epic,
            Ok(vec![detected("epic:Sample", "Sample", None)]),
            false,
        ),
        Scanner(Origin::Gog, Err(ScanError::NotInstalled), false),
    ]
}

#[test]
fn primeira_leitura_entrega_o_cache_e_dispara_a_varredura_em_segundo_plano() {
    let h = harness(two_stores());
    // A primeira leitura nao espera a varredura: volta com o que e do usuario (vazio aqui).
    assert_eq!(
        h.call("library_get", json!({})).unwrap(),
        json!({ "games": [], "focusGameId": null })
    );
    h.wait_events(1);
    assert_eq!(h.names(), ["Portal 2", "Sample"]);
    // So a primeira leitura varre: as seguintes nao disparam nada.
    for _ in 0..3 {
        h.call("library_get", json!({})).unwrap();
    }
    std::thread::sleep(Duration::from_millis(100));
    assert_eq!(h.events.0.load(Ordering::SeqCst), 1);
    let g = &h.call("library_get", json!({})).unwrap()["games"][0];
    assert_eq!(g["cover"], "cover://steam:620");
    assert_eq!(g["inProgress"], false);
    assert!(g.get("path").is_none() && !g.to_string().contains("Jogos"));
}

#[test]
fn refresh_e_serializado_loja_com_defeito_nao_derruba_as_outras() {
    let mut s = two_stores();
    s[1] = Scanner(Origin::Epic, Ok(Vec::new()), true); // este entra em panico
    let h = harness(s);
    let state = h.app.state::<AppState>();
    let permit = state.library_gate.try_enter().unwrap();
    assert_eq!(h.code("library_refresh", json!({})), "Busy");
    drop(permit);
    assert_eq!(h.call("library_refresh", json!({})).unwrap(), Value::Null);
    h.wait_events(1);
    assert_eq!(h.names(), ["Portal 2"]);
}

#[test]
fn renomear_ocultar_restaurar_pela_ipc() {
    let h = harness(two_stores());
    h.call("library_refresh", json!({})).unwrap();
    h.wait_events(1);
    let r = h
        .call(
            "game_rename",
            json!({ "gameId": "steam:620", "name": "  Portal\u{202E} 2 (coop) " }),
        )
        .unwrap();
    assert_eq!(
        r,
        json!({ "id": "steam:620", "name": "Portal 2 (coop)", "originalName": "Portal 2", "origin": "Steam",
                "cover": "cover://steam:620", "lastPlayedAt": null, "inProgress": false })
    );
    assert_eq!(
        h.code(
            "game_rename",
            json!({ "gameId": "steam:620", "name": "\u{202E} " })
        ),
        "EmptyName"
    );
    assert_eq!(
        h.code("game_rename", json!({ "gameId": "steam:1", "name": "x" })),
        "UnknownGame"
    );
    assert_eq!(
        h.code("game_restore_name", json!({ "gameId": "steam:1" })),
        "UnknownGame"
    );
    let r = h
        .call("game_restore_name", json!({ "gameId": "steam:620" }))
        .unwrap();
    assert_eq!(r["name"], "Portal 2");
    assert_eq!(r["originalName"], Value::Null);

    assert_eq!(
        h.call("game_hide", json!({ "gameId": "steam:620" }))
            .unwrap(),
        Value::Null
    );
    assert_eq!(h.names(), ["Sample"]);
    let hidden = h.call("library_hidden", json!({})).unwrap();
    assert_eq!(hidden[0]["id"], "steam:620");
    assert_eq!(
        h.code("game_hide", json!({ "gameId": "steam:99" })),
        "UnknownGame"
    );
    h.call("game_unhide", json!({ "gameId": "steam:620" }))
        .unwrap();
    assert_eq!(h.names(), ["Portal 2", "Sample"]);
    assert_eq!(
        h.code("game_unhide", json!({ "gameId": "steam:99" })),
        "UnknownGame"
    );
}

#[test]
fn cadastro_manual_recusa_cada_caso_com_o_codigo_certo() {
    let h = harness(Vec::new());
    for (path, code) in [
        (r"C:\Jogos\atalho.lnk", "NotAnExecutable"),
        (r"C:\Jogos\run.bat", "NotAnExecutable"),
        (r"C:\Jogos\leia.txt", "NotAnExecutable"),
        (r"C:\jogo.exe:ads", "NotAnExecutable"),
        (r"\\127.0.0.1\c$\Jogos\x.exe", "PathNotAllowed"),
        (r"\\?\C:\Jogos\x.exe", "PathNotAllowed"),
        (r"jogo.exe", "InvalidRequest"),
    ] {
        assert_eq!(
            h.code("library_add_manual", json!({ "path": path })),
            code,
            "{path}"
        );
    }
    // O disco recusa: arquivo ausente, rede mapeada, `.txt` renomeado para `.exe`.
    for (erro, code) in [
        (AppError::NotFound, "NotFound"),
        (AppError::PathNotAllowed, "PathNotAllowed"),
        (AppError::NotAnExecutable, "NotAnExecutable"),
    ] {
        *h.inspector.0.lock().unwrap() = Some(erro);
        assert_eq!(
            h.code("library_add_manual", json!({ "path": r"C:\Jogos\x.exe" })),
            code
        );
    }
    *h.inspector.0.lock().unwrap() = None;
    let ok = h
        .call(
            "library_add_manual",
            json!({ "path": r"C:\Jogos\Foo\foo.exe" }),
        )
        .unwrap();
    assert_eq!(ok["name"], "foo");
    assert_eq!(ok["origin"], "Manual");
    assert!(ok["id"].as_str().unwrap().starts_with("manual:"));
    assert_eq!(
        h.code(
            "library_add_manual",
            json!({ "path": r"c:\JOGOS\foo\FOO.exe" })
        ),
        "Duplicate"
    );
    assert_eq!(h.names(), ["foo"]);
}

#[test]
fn navegador_de_pastas_rejeita_rede_e_prefixos_e_traz_so_pastas_e_exe() {
    let h = harness(Vec::new());
    for p in [
        r"\\127.0.0.1\c$",
        r"\\?\C:\Windows",
        r"\\.\C:\",
        r"C:\a\..\b",
        r"C:\x:ads",
    ] {
        assert_eq!(
            h.code("fs_list_dir", json!({ "path": p })),
            "PathNotAllowed",
            "{p}"
        );
    }
    assert_eq!(
        h.code("fs_list_dir", json!({ "path": "relativo" })),
        "InvalidRequest"
    );
    let l = h
        .call("fs_list_dir", json!({ "path": r"D:\Jogos" }))
        .unwrap();
    assert_eq!(
        l,
        json!({ "path": r"D:\Jogos", "parent": r"D:\", "truncated": false,
                "entries": [ { "name": "Jogo", "kind": "Dir" }, { "name": "a.exe", "kind": "Exe" } ] })
    );
}

// ----------------------------------------------------------------------------- cover://

fn cover_request(uri: &str) -> Request<Vec<u8>> {
    Request::builder()
        .method("GET")
        .uri(uri)
        .body(Vec::new())
        .unwrap()
}

fn cover_status(h: &Harness, uri: &str) -> u16 {
    let state = h.app.state::<AppState>();
    sofa_lib::cover_protocol::respond(&state, &cover_request(uri))
        .status()
        .as_u16()
}

#[test]
fn cover_so_serve_game_id_existente_e_nunca_arquivo() {
    let h = harness(two_stores());
    h.call("library_refresh", json!({})).unwrap();
    h.wait_events(1);
    let state = h.app.state::<AppState>();
    // Existe e tem imagem: 200 com os bytes do encoder do nucleo.
    for uri in [
        "http://cover.localhost/steam%3A620",
        "http://cover.localhost/steam:620",
        "cover://localhost/steam%3A620",
    ] {
        let r = sofa_lib::cover_protocol::respond(&state, &cover_request(uri));
        assert_eq!(r.status().as_u16(), 200, "{uri}");
        assert_eq!(r.headers()["content-type"], "image/jpeg");
        assert_eq!(r.headers()["x-content-type-options"], "nosniff");
        assert_eq!(r.body(), &vec![0xFF, 0xD8, 0xFF, 0xD9]);
    }
    // Existe e nao tem imagem: 404 (placeholder, CA-24).
    assert_eq!(
        cover_status(&h, "http://cover.localhost/epic%3ASample"),
        404
    );
    // Fora da biblioteca, malformado, traversal e `epic:..` (CT-06): 404, sem tocar em disco.
    for uri in [
        "http://cover.localhost/steam%3A999",
        "cover://../../Windows/win.ini",
        "http://cover.localhost/..%2F..%2FWindows%2Fwin.ini",
        "http://cover.localhost/..\\..\\Windows\\win.ini",
        "http://cover.localhost/C%3A%5CWindows%5Cwin.ini",
        "http://cover.localhost/epic%3A..",
        "http://cover.localhost/steam%3A620%00",
        "http://cover.localhost/steam%3A620/extra",
        "http://cover.localhost/%ZZ",
        "http://cover.localhost/",
        "http://cover.localhost",
    ] {
        assert_eq!(cover_status(&h, uri), 404, "{uri}");
    }
    // Metodo que nao e GET.
    let post = Request::builder()
        .method("POST")
        .uri("http://cover.localhost/steam%3A620")
        .body(Vec::new())
        .unwrap();
    assert_eq!(
        sofa_lib::cover_protocol::respond(&state, &post)
            .status()
            .as_u16(),
        404
    );
}

#[test]
fn cover_sem_biblioteca_ligada_e_404() {
    let app = sofa_lib::configure(mock_builder())
        .build(mock_context(noop_assets()))
        .unwrap();
    let state = app.state::<AppState>();
    let r = sofa_lib::cover_protocol::respond(
        &state,
        &cover_request("http://cover.localhost/steam%3A620"),
    );
    assert_eq!(r.status().as_u16(), 404);
}

#[test]
fn nenhum_asset_protocol_habilitado() {
    let conf: Value = serde_json::from_str(include_str!("../tauri.conf.json")).unwrap();
    let asset = &conf["app"]["security"]["assetProtocol"];
    assert!(
        asset.is_null() || asset["enable"] == json!(false),
        "{asset}"
    );
    // As capas so podem vir do protocolo proprio.
    let csp = conf["app"]["security"]["csp"].as_str().unwrap();
    assert!(
        csp.contains("img-src 'self' cover: http://cover.localhost data:"),
        "{csp}"
    );
    assert!(
        !csp.contains("asset:") && !csp.contains("asset.localhost"),
        "{csp}"
    );
}
