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
fn power_prepare_nao_emite_ticket_antes_do_be33() {
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
