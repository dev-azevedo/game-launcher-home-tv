//! SEC-03 / SEC-06: os comandos pelo caminho real de IPC do Tauri (runtime de teste). Campo extra,
//! tipo errado, campo ausente e `GameId` malformado devolvem `ErrorDto { code: "InvalidRequest" }`,
//! e as acoes serializadas respondem `Busy`.

use serde_json::{json, Value};
use sofa_lib::commands::list::COMMAND_NAMES;
use tauri::ipc::{CallbackFn, InvokeBody};
use tauri::test::MockRuntime;
use tauri::test::{get_ipc_response, mock_builder, mock_context, noop_assets, INVOKE_KEY};
use tauri::webview::InvokeRequest;
use tauri::{App, Manager, WebviewWindow, WebviewWindowBuilder};

struct Harness {
    _app: App<MockRuntime>,
    webview: WebviewWindow<MockRuntime>,
}

fn harness() -> Harness {
    let app = sofa_lib::configure(mock_builder())
        .build(mock_context(noop_assets()))
        .expect("app de teste");
    let webview = WebviewWindowBuilder::new(&app, "main", Default::default())
        .build()
        .expect("janela de teste");
    Harness { _app: app, webview }
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
            .unwrap_or_else(|| panic!("erro sem code: {err}"))
            .to_owned()
    }
}

/// Corpo valido minimo de cada comando com argumentos (os sem argumentos recebem `{}`).
fn valid_body(cmd: &str) -> Value {
    match cmd {
        "library_add_manual" => json!({ "path": "C:\\Jogos\\a.exe" }),
        "game_rename" => json!({ "gameId": "steam:620", "name": "Ok" }),
        "game_restore_name"
        | "game_hide"
        | "game_unhide"
        | "session_launch"
        | "session_launch_with_uac"
        | "session_replace" => json!({ "gameId": "steam:620" }),
        "fs_list_dir" => json!({ "path": null }),
        "volume_step" => json!({ "direction": "Up" }),
        "power_prepare" => json!({ "action": "Shutdown" }),
        "power_execute" => json!({ "ticket": "A".repeat(22) }),
        "settings_set_autostart" => json!({ "enabled": true }),
        "settings_set_monitor" => json!({ "monitorId": null }),
        _ => json!({}),
    }
}

#[test]
fn todo_comando_rejeita_campo_extra_com_invalid_request() {
    let h = harness();
    for cmd in COMMAND_NAMES {
        let mut body = valid_body(cmd);
        body.as_object_mut()
            .unwrap()
            .insert("campoExtra".into(), json!(1));
        assert_eq!(h.code(cmd, body), "InvalidRequest", "{cmd}");
    }
}

#[test]
fn comandos_com_argumentos_rejeitam_tipo_errado_ausente_e_game_id_malformado() {
    let h = harness();
    let with_game_id = [
        "game_restore_name",
        "game_hide",
        "game_unhide",
        "session_launch",
        "session_launch_with_uac",
        "session_replace",
        "game_rename",
    ];
    for cmd in with_game_id {
        for bad in [
            json!(620),
            json!("steam:10//-console"),
            json!("epic:a b"),
            json!("Steam:620"),
            json!(""),
        ] {
            let mut body = valid_body(cmd);
            body["gameId"] = bad.clone();
            assert_eq!(h.code(cmd, body), "InvalidRequest", "{cmd} {bad}");
        }
        let mut body = valid_body(cmd);
        body.as_object_mut().unwrap().remove("gameId");
        assert_eq!(h.code(cmd, body), "InvalidRequest", "{cmd} sem gameId");
    }
    assert_eq!(
        h.code("game_rename", json!({ "gameId": "steam:620", "name": 7 })),
        "InvalidRequest"
    );
    assert_eq!(
        h.code("library_add_manual", json!({ "path": 7 })),
        "InvalidRequest"
    );
    assert_eq!(h.code("library_add_manual", json!({})), "InvalidRequest");
    assert_eq!(h.code("fs_list_dir", json!({})), "InvalidRequest");
    assert_eq!(
        h.code("volume_step", json!({ "direction": "Sideways" })),
        "InvalidRequest"
    );
    assert_eq!(
        h.code("power_prepare", json!({ "action": "Hibernate" })),
        "InvalidRequest"
    );
    assert_eq!(
        h.code("power_execute", json!({ "ticket": "curto" })),
        "InvalidRequest"
    );
    assert_eq!(
        h.code("settings_set_autostart", json!({ "enabled": "sim" })),
        "InvalidRequest"
    );
    assert_eq!(h.code("settings_set_monitor", json!({})), "InvalidRequest");
}

#[test]
fn leituras_sem_backend_respondem_estado_neutro() {
    let h = harness();
    assert_eq!(
        h.call("library_get", json!({})).unwrap(),
        json!({ "games": [], "focusGameId": null })
    );
    assert_eq!(h.call("library_hidden", json!({})).unwrap(), json!([]));
    assert_eq!(h.call("session_get", json!({})).unwrap()["state"], "Idle");
    assert_eq!(h.call("volume_get", json!({})).unwrap()["available"], false);
    assert_eq!(h.call("display_monitors", json!({})).unwrap(), json!([]));
    assert_eq!(
        h.call("input_status", json!({})).unwrap(),
        json!({ "connected": 0 })
    );
    assert_eq!(
        h.call("power_capabilities", json!({})).unwrap()["shutdown"],
        "Unavailable"
    );
    assert_eq!(h.call("session_dismiss", json!({})).unwrap(), json!(null));
    // Sem pasta de dados carregada (so o `setup` real a define), `logDir` vem vazio.
    assert_eq!(
        h.call("settings_get", json!({})).unwrap(),
        json!({ "autostart": false, "autostartMechanism": "None", "monitorId": null, "logDir": "" })
    );
}

#[test]
fn comando_nao_ligado_devolve_internal_sem_detalhe() {
    let h = harness();
    let err = h
        .call("game_hide", json!({ "gameId": "steam:620" }))
        .unwrap_err();
    assert_eq!(
        err,
        json!({ "code": "Internal" }),
        "nada alem do codigo cruza a IPC (SEC-06)"
    );
}

#[test]
fn sem_servico_de_energia_nada_e_emitido_nem_executado() {
    let h = harness();
    assert_eq!(
        h.code("power_prepare", json!({ "action": "Shutdown" })),
        "Unavailable"
    );
    assert_eq!(
        h.code("power_execute", json!({ "ticket": "A".repeat(22) })),
        "Internal"
    );
}

#[test]
fn comando_desconhecido_nao_existe() {
    let h = harness();
    // Comando fora da lista (ex.: purge) nunca e roteado.
    let err = h.call("purge_user_data", json!({})).unwrap_err();
    assert!(err.is_string(), "{err}"); // mensagem do Tauri, nao um ErrorDto
}

#[test]
fn acoes_serializadas_respondem_busy_enquanto_outra_roda() {
    let h = harness();
    let state = h._app.state::<sofa_lib::state::AppState>();

    let permit = state.session_gate.try_enter().unwrap();
    for (cmd, body) in [
        ("session_launch", json!({ "gameId": "steam:620" })),
        ("session_launch_with_uac", json!({ "gameId": "steam:620" })),
        ("session_replace", json!({ "gameId": "steam:620" })),
        ("session_retry", json!({})),
    ] {
        assert_eq!(h.code(cmd, body), "Busy", "{cmd}");
    }
    // A porta da biblioteca e outra: nao e afetada.
    assert_eq!(h.code("library_refresh", json!({})), "Internal");
    drop(permit);
    assert_eq!(
        h.code("session_launch", json!({ "gameId": "steam:620" })),
        "Internal"
    );

    let permit = state.library_gate.try_enter().unwrap();
    assert_eq!(h.code("library_refresh", json!({})), "Busy");
    // Leitura nunca da Busy.
    assert!(h.call("library_get", json!({})).is_ok());
    drop(permit);
}

mod power_fake {
    use std::sync::{Arc, Mutex};

    use launcher_core::application::ports::{Clock, ManualClock, PowerControl, RandomSource};
    use launcher_core::application::{AppError, PowerCapabilities, PowerService};
    use launcher_core::domain::power::PowerAction;

    #[derive(Default)]
    pub struct FakeControl {
        pub suspend: bool,
        pub executed: Mutex<Vec<PowerAction>>,
    }

    impl PowerControl for FakeControl {
        fn capabilities(&self) -> PowerCapabilities {
            PowerCapabilities {
                shutdown: true,
                restart: true,
                suspend: self.suspend,
            }
        }
        fn execute(&self, action: PowerAction) -> Result<(), AppError> {
            self.executed.lock().unwrap().push(action);
            Ok(())
        }
    }

    pub struct Counter(pub Mutex<u8>);
    impl RandomSource for Counter {
        fn fill(&self, buf: &mut [u8]) -> Result<(), AppError> {
            let mut n = self.0.lock().unwrap();
            *n += 1;
            buf.iter_mut().for_each(|b| *b = *n);
            Ok(())
        }
    }

    pub fn service(suspend: bool) -> (Arc<FakeControl>, Arc<ManualClock>, PowerService) {
        let control = Arc::new(FakeControl {
            suspend,
            ..Default::default()
        });
        let clock = Arc::new(ManualClock::new());
        let dyn_clock: Arc<dyn Clock> = clock.clone();
        let svc = PowerService::new(control.clone(), Arc::new(Counter(Mutex::new(0))), dyn_clock);
        (control, clock, svc)
    }
}

/// CT-07 e SEC-27 pelo caminho real da IPC, com o Windows trocado por um fake: nada aqui
/// desliga, reinicia ou suspende o PC.
#[test]
fn energia_ticket_de_ponta_a_ponta_pela_ipc() {
    use launcher_core::domain::power::PowerAction;

    let h = harness();
    let (control, clock, svc) = power_fake::service(false);
    h._app
        .state::<sofa_lib::state::AppState>()
        .power
        .set(std::sync::Arc::new(svc))
        .ok()
        .unwrap();

    assert_eq!(
        h.call("power_capabilities", json!({})).unwrap(),
        json!({ "shutdown": "Available", "restart": "Available", "suspend": "Unavailable" })
    );
    // CA-47: suspensao indisponivel nao emite ticket.
    assert_eq!(
        h.code("power_prepare", json!({ "action": "Suspend" })),
        "Unavailable"
    );

    let ticket = h
        .call("power_prepare", json!({ "action": "Restart" }))
        .unwrap();
    let ticket = ticket.as_str().expect("ticket em texto").to_owned();
    assert!((22..=64).contains(&ticket.len()), "{ticket}");
    // Um prepare novo invalida o anterior (CT-07).
    let novo = h
        .call("power_prepare", json!({ "action": "Shutdown" }))
        .unwrap();
    let novo = novo.as_str().unwrap().to_owned();
    assert_eq!(
        h.code("power_execute", json!({ "ticket": ticket })),
        "TicketInvalid"
    );
    assert!(control.executed.lock().unwrap().is_empty());

    assert_eq!(
        h.call("power_execute", json!({ "ticket": novo.clone() }))
            .unwrap(),
        json!(null)
    );
    assert_eq!(
        *control.executed.lock().unwrap(),
        vec![PowerAction::Shutdown]
    );
    // Uso unico.
    assert_eq!(
        h.code("power_execute", json!({ "ticket": novo })),
        "TicketInvalid"
    );
    // Expirado.
    let t = h
        .call("power_prepare", json!({ "action": "Restart" }))
        .unwrap();
    clock.advance_ms(30_001);
    assert_eq!(
        h.code("power_execute", json!({ "ticket": t.as_str().unwrap() })),
        "TicketInvalid"
    );
    assert_eq!(control.executed.lock().unwrap().len(), 1);
    // Acao fora do enum nunca chega ao servico (SEC-27).
    assert_eq!(
        h.code("power_prepare", json!({ "action": "Hibernate" })),
        "InvalidRequest"
    );
}

/// BE-34 pela IPC com monitores falsos (nenhuma janela real se mexe): `display_monitors`,
/// `settings_set_monitor` (UnknownMonitor, grava, move, `display://changed`) e a preferencia
/// que continua gravada em `settings_get`.
#[test]
fn monitor_pela_ipc_com_topologia_falsa() {
    use std::sync::{Arc, Mutex};

    use launcher_core::application::ports::{
        DisplayTopology, LauncherPlacement, Loaded, SettingsRepository, StoreError,
    };
    use launcher_core::application::DisplayService;
    use launcher_core::domain::display::{Monitor, Rect};
    use launcher_core::domain::settings::Settings;
    use sofa_lib::events::TauriDisplayEvents;
    use sofa_lib::settings_store::AppSettingsStore;
    use sofa_lib::state::{AppState, UserData};
    use tauri::Listener;

    struct Topology;
    impl DisplayTopology for Topology {
        fn monitors(&self) -> Vec<Monitor> {
            let rect = |x| Rect {
                x,
                y: 0,
                width: 1920,
                height: 1080,
            };
            vec![
                Monitor {
                    id: "PC".into(),
                    name: "Monitor do PC\u{202e}".into(), // controle bidi: o DTO normaliza
                    primary: true,
                    rect: rect(0),
                },
                Monitor {
                    id: "TV".into(),
                    name: "\u{7}".into(), // vazio depois de normalizar
                    primary: false,
                    rect: rect(1920),
                },
            ]
        }
    }
    #[derive(Default)]
    struct Placement(Mutex<Vec<String>>);
    impl LauncherPlacement for Placement {
        fn place(&self, m: &Monitor) -> bool {
            self.0.lock().unwrap().push(m.id.clone());
            true
        }
    }
    #[derive(Default)]
    struct MemRepo(Mutex<Option<Settings>>);
    impl SettingsRepository for MemRepo {
        fn load(&self) -> Loaded<Settings> {
            Loaded {
                value: Settings::default(),
                recovery: None,
                warnings: Vec::new(),
            }
        }
        fn save(&self, s: &Settings) -> Result<(), StoreError> {
            *self.0.lock().unwrap() = Some(s.clone());
            Ok(())
        }
    }

    let h = harness();
    let handle = h._app.handle().clone();
    let state = h._app.state::<AppState>();
    let repo = Arc::new(MemRepo::default());
    state
        .user_data
        .set(UserData {
            data_dir: std::env::temp_dir(),
            settings_repo: repo.clone(),
        })
        .ok()
        .unwrap();
    let placement = Arc::new(Placement::default());
    let display = DisplayService::new(
        Arc::new(Topology),
        placement.clone(),
        Arc::new(AppSettingsStore::new(handle.clone())),
        Arc::new(TauriDisplayEvents::new(handle.clone())),
    );
    display.start();
    state.display.set(Arc::new(display)).ok().unwrap();

    let seen: Arc<Mutex<Vec<String>>> = Arc::default();
    let sink = seen.clone();
    h._app.listen("display://changed", move |e| {
        sink.lock().unwrap().push(e.payload().to_owned());
    });

    assert_eq!(
        h.call("display_monitors", json!({})).unwrap(),
        json!([
            { "id": "PC", "name": "Monitor do PC", "primary": true, "current": true },
            { "id": "TV", "name": "Monitor 2", "primary": false, "current": false }
        ])
    );
    assert_eq!(
        h.code("settings_set_monitor", json!({ "monitorId": "FANTASMA" })),
        "UnknownMonitor"
    );
    assert!(repo.0.lock().unwrap().is_none(), "nada gravado");

    let dto = h
        .call("settings_set_monitor", json!({ "monitorId": "TV" }))
        .unwrap();
    assert_eq!(dto["monitorId"], "TV");
    assert_eq!(*placement.0.lock().unwrap(), vec!["TV"]);
    assert_eq!(
        repo.0
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .monitor_id
            .as_deref(),
        Some("TV")
    );
    assert_eq!(
        h.call("settings_get", json!({})).unwrap()["monitorId"],
        "TV"
    );
    // `display://changed` leva a lista inteira, com o monitor atual marcado.
    std::thread::sleep(std::time::Duration::from_millis(50));
    let payloads = seen.lock().unwrap().clone();
    assert_eq!(payloads.len(), 1, "{payloads:?}");
    let list: Value = serde_json::from_str(&payloads[0]).unwrap();
    assert_eq!(list[1]["current"], true);
    assert_eq!(list[0]["current"], false);

    // Voltar ao principal.
    let dto = h
        .call("settings_set_monitor", json!({ "monitorId": null }))
        .unwrap();
    assert_eq!(dto["monitorId"], json!(null));
    assert_eq!(placement.0.lock().unwrap().last().unwrap(), "PC");
}
