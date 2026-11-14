//! SEC-03 / SEC-06: leitura tipada dos argumentos (`ipc::parse_json`), `catch_unwind` por comando
//! (`ipc::guard`) e `ErrorDto` (`errors.rs`). Integration test: o crate `sofa_lib` nao tem testes
//! unitarios porque o binario de teste da lib nao leva o manifesto Common Controls v6 do Tauri.

use launcher_core::application::AppError;
use serde_json::{json, Value};
use sofa_lib::dto::*;
use sofa_lib::ipc::{guard, parse_json, require_no_args};
use tauri::ipc::InvokeBody;

fn body(v: Value) -> InvokeBody {
    InvokeBody::Json(v)
}

fn code<T: std::fmt::Debug>(r: Result<T, AppError>) -> &'static str {
    r.unwrap_err().code()
}

#[test]
fn game_id_args_aceita_o_formato_certo() {
    let a: GameIdArgs = parse_json(&body(json!({ "gameId": "steam:620" }))).unwrap();
    assert_eq!(a.game_id.0, "steam:620");
}

#[test]
fn campo_extra_tipo_errado_ausente_e_id_malformado_sao_invalid_request() {
    let casos = [
        json!({ "gameId": "steam:620", "extra": 1 }),
        json!({ "gameId": 620 }),
        json!({}),
        json!({ "gameId": "steam:10//-console" }),
        json!({ "gameId": "epic:a b" }),
        json!(null),
        json!([]),
    ];
    for c in casos {
        assert_eq!(
            code(parse_json::<GameIdArgs>(&body(c.clone()))),
            "InvalidRequest",
            "{c}"
        );
    }
    assert_eq!(
        code(parse_json::<GameIdArgs>(&InvokeBody::Raw(vec![1, 2]))),
        "InvalidRequest"
    );
}

#[test]
fn rename_exige_os_dois_campos_e_limita_o_nome_bruto() {
    assert!(parse_json::<RenameArgs>(&body(json!({ "gameId": "gog:1", "name": "Ok" }))).is_ok());
    let grande = "x".repeat(1025);
    for c in [
        json!({ "gameId": "gog:1" }),
        json!({ "name": "Ok" }),
        json!({ "gameId": "gog:1", "name": grande }),
        json!({ "gameId": "gog:1", "name": "Ok", "path": "C:\\x.exe" }),
    ] {
        assert_eq!(code(parse_json::<RenameArgs>(&body(c))), "InvalidRequest");
    }
}

#[test]
fn caminhos_so_entram_nos_dois_comandos_com_limites() {
    assert!(parse_json::<AddManualArgs>(&body(json!({ "path": "C:\\Jogos\\a.exe" }))).is_ok());
    for c in [
        json!({ "path": "" }),
        json!({ "path": "a".repeat(1025) }),
        json!({ "path": "C:\\a\u{0}.exe" }),
        json!({}),
        json!({ "path": "C:\\a.exe", "args": "-x" }),
    ] {
        assert_eq!(
            code(parse_json::<AddManualArgs>(&body(c))),
            "InvalidRequest"
        );
    }
    // fs_list_dir: null e vazio sao validos (unidades locais); ausente nao.
    let n: ListDirArgs = parse_json(&body(json!({ "path": null }))).unwrap();
    assert_eq!(n.path.0, None);
    assert!(parse_json::<ListDirArgs>(&body(json!({ "path": "" }))).is_ok());
    assert_eq!(
        code(parse_json::<ListDirArgs>(&body(json!({})))),
        "InvalidRequest"
    );
    assert_eq!(
        code(parse_json::<ListDirArgs>(&body(
            json!({ "path": "b".repeat(1025) })
        ))),
        "InvalidRequest"
    );
}

#[test]
fn enums_ticket_e_monitor_sao_fechados() {
    assert!(parse_json::<VolumeStepArgs>(&body(json!({ "direction": "Up" }))).is_ok());
    assert_eq!(
        code(parse_json::<VolumeStepArgs>(&body(
            json!({ "direction": "up" })
        ))),
        "InvalidRequest"
    );
    assert!(parse_json::<PowerPrepareArgs>(&body(json!({ "action": "Suspend" }))).is_ok());
    assert_eq!(
        code(parse_json::<PowerPrepareArgs>(&body(
            json!({ "action": "Hibernate" })
        ))),
        "InvalidRequest"
    );

    let ticket = "A".repeat(22);
    assert!(parse_json::<PowerExecuteArgs>(&body(json!({ "ticket": ticket }))).is_ok());
    for t in [
        "short".to_string(),
        "A".repeat(65),
        format!("{}!", "A".repeat(22)),
    ] {
        assert_eq!(
            code(parse_json::<PowerExecuteArgs>(&body(
                json!({ "ticket": t })
            ))),
            "InvalidRequest"
        );
    }

    assert_eq!(
        parse_json::<SetMonitorArgs>(&body(json!({ "monitorId": null })))
            .unwrap()
            .monitor_id
            .0,
        None
    );
    assert!(parse_json::<SetMonitorArgs>(&body(json!({ "monitorId": "DISPLAY1" }))).is_ok());
    for c in [
        json!({}),
        json!({ "monitorId": "" }),
        json!({ "monitorId": "m".repeat(257) }),
    ] {
        assert_eq!(
            code(parse_json::<SetMonitorArgs>(&body(c))),
            "InvalidRequest"
        );
    }
    assert_eq!(
        code(parse_json::<SetAutostartArgs>(&body(
            json!({ "enabled": "yes" })
        ))),
        "InvalidRequest"
    );
}

#[test]
fn comando_sem_argumentos_recusa_campo_extra() {
    assert!(require_no_args(&body(json!({}))).is_ok());
    assert!(require_no_args(&body(json!(null))).is_ok());
    assert_eq!(
        code(require_no_args(&body(json!({ "x": 1 })))),
        "InvalidRequest"
    );
    assert_eq!(
        code(require_no_args(&InvokeBody::Raw(vec![]))),
        "InvalidRequest"
    );
}

#[test]
fn guard_converte_erro_e_panico() {
    let ok: Result<u8, ErrorDto> = guard("t", || Ok(7));
    assert_eq!(ok.unwrap(), 7);

    let err: Result<u8, ErrorDto> = guard("t", || Err(AppError::Busy));
    assert_eq!(err.unwrap_err().code, ErrorCode::Busy);

    let boom: Result<u8, ErrorDto> = guard("t", || panic!("falha simulada"));
    let dto = boom.unwrap_err();
    assert_eq!(dto.code, ErrorCode::Internal);
    assert!(dto.params.is_none());
}

#[test]
fn error_dto_serializa_como_o_contrato() {
    use launcher_core::domain::library::{GameId, Origin};
    let e = ErrorDto::from(AppError::SessionAlreadyActive {
        game_id: GameId::parse("steam:620").unwrap(),
    });
    assert_eq!(
        serde_json::to_value(&e).unwrap(),
        json!({ "code": "SessionAlreadyActive", "params": { "gameId": "steam:620" } })
    );
    let e = ErrorDto::from(AppError::StoreNotInstalled {
        origin: Origin::Epic,
    });
    assert_eq!(
        serde_json::to_value(&e).unwrap(),
        json!({ "code": "StoreNotInstalled", "params": { "origin": "Epic" } })
    );
    assert_eq!(
        serde_json::to_value(ErrorDto::from(AppError::InvalidRequest)).unwrap(),
        json!({ "code": "InvalidRequest" })
    );
}

#[test]
fn todo_app_error_tem_o_mesmo_codigo_no_dto() {
    use launcher_core::application::error::ALL_CODES;
    // O enum `ErrorCode` serializa os mesmos 26 nomes do contrato.
    let json = serde_json::to_value(ErrorCode::Busy).unwrap();
    assert_eq!(json, json!("Busy"));
    for code in ALL_CODES {
        let parsed: ErrorCode =
            serde_json::from_value(json!(code)).unwrap_or_else(|_| panic!("{code}"));
        assert_eq!(serde_json::to_value(parsed).unwrap(), json!(code));
    }
}
