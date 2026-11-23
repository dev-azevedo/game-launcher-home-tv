//! BE-10: tipos TypeScript da IPC gerados a partir dos DTOs (plano B `ts-rs`, sem `tauri-specta`).
//!
//! `cargo test -p sofa --test bindings` regenera `ui/src/shared/ipc/generated/` por inteiro
//! (`*.ts` por tipo, `index.ts` e `commandNames.ts`). Nada ali e editado a mao; o `scripts/ci.ps1`
//! roda este teste e falha se `git diff` mostrar mudanca na pasta (bindings desatualizados).

use std::collections::BTreeSet;
use std::fmt::Write as _;
use std::fs;
use std::path::PathBuf;

use sofa_lib::commands::list::COMMAND_NAMES;
use sofa_lib::dto::*;
use ts_rs::TS;

fn out_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../ui/src/shared/ipc/generated")
}

/// Exporta cada tipo e devolve os nomes TypeScript, na ordem dada.
macro_rules! export_types {
    ($dir:expr; $($t:ty),+ $(,)?) => {{
        let mut names: Vec<String> = Vec::new();
        $(
            <$t as TS>::export_all_to($dir).unwrap_or_else(|e| panic!("{}: {e}", stringify!($t)));
            names.push(<$t as TS>::name());
        )+
        names
    }};
}

#[test]
fn gera_bindings_e_confere_o_contrato() {
    let dir = out_dir();
    // Pasta gerada: recria do zero para nao sobrar tipo removido.
    if dir.exists() {
        for entry in fs::read_dir(&dir).unwrap() {
            let path = entry.unwrap().path();
            if path.extension().is_some_and(|e| e == "ts") {
                fs::remove_file(path).unwrap();
            }
        }
    }
    fs::create_dir_all(&dir).unwrap();

    let names = export_types!(&dir;
        // enums
        OriginDto, SessionState, InputIntent, PowerAction, Availability, VolumeDirection,
        FrontOutcome, AutostartMechanism, DirEntryKind, LaunchFailure, ErrorCode,
        // saidas
        GameDto, LibrarySnapshot, SessionDto, VolumeState, MonitorDto, SettingsDto, DirEntry,
        DirListing, PowerCapabilities, InputStatus, UpdateDto, ErrorParams, ErrorDto,
        // eventos
        InputIntentEvent, ControllersEvent, SessionPanelEvent, UpdateProgressEvent,
        // entradas
        GameIdArgs, RenameArgs, AddManualArgs, ListDirArgs, VolumeStepArgs, PowerPrepareArgs,
        PowerExecuteArgs, SetAutostartArgs, SetMonitorArgs,
    );

    // index.ts: reexporta tudo e declara os aliases de texto do contrato.
    let mut index = String::from(
        "// Gerado por `cargo test -p sofa --test bindings`. Nao edite a mao.\n\n\
         export type GameId = string;\nexport type MonitorId = string;\nexport type PowerTicket = string;\n\n",
    );
    let unique: BTreeSet<&String> = names.iter().collect();
    for name in &unique {
        writeln!(index, "export type {{ {name} }} from './{name}';").unwrap();
    }
    fs::write(dir.join("index.ts"), index).unwrap();

    // commandNames.ts: a lista fechada, na ordem do contrato.
    let mut cmds = String::from(
        "// Gerado por `cargo test -p sofa --test bindings` a partir de src-tauri/command_list.rs.\n\
         // Nao edite a mao.\n\nexport const COMMAND_NAMES = [\n",
    );
    for n in COMMAND_NAMES {
        writeln!(cmds, "  '{n}',").unwrap();
    }
    cmds.push_str("] as const;\n\nexport type CommandName = (typeof COMMAND_NAMES)[number];\n");
    fs::write(dir.join("commandNames.ts"), cmds).unwrap();

    // --- conferencias do que foi gerado ---
    let read = |n: &str| fs::read_to_string(dir.join(format!("{n}.ts"))).unwrap();

    let error_code = read("ErrorCode");
    for code in launcher_core::application::error::ALL_CODES {
        assert!(
            error_code.contains(&format!("\"{code}\"")),
            "ErrorCode sem {code}"
        );
    }
    assert_eq!(
        error_code.matches('"').count(),
        28 * 2,
        "ErrorCode deve ter exatamente 28 codigos"
    );

    // Origin.ts (nome TS do OriginDto) e sem `bigint`/`any`/`unknown` em nenhum tipo.
    assert!(dir.join("Origin.ts").exists());
    for entry in fs::read_dir(&dir).unwrap() {
        let path = entry.unwrap().path();
        let text = fs::read_to_string(&path).unwrap();
        for bad in ["bigint", ": any", ": unknown"] {
            assert!(!text.contains(bad), "{}: {bad}", path.display());
        }
    }
    // Campos do contrato: tempos em `number`, `truncated` obrigatorio, params opcionais.
    assert!(read("GameDto").contains("lastPlayedAt: number | null"));
    assert!(read("SessionDto").contains("since: number | null"));
    assert!(read("DirListing").contains("truncated: boolean"));
    assert!(read("ErrorDto").contains("params?: ErrorParams"));
    assert!(read("ErrorParams").contains("gameId?: string"));
    // SEC-03: GameDto nao tem caminho de instalacao nem do executavel.
    let game = read("GameDto");
    for forbidden in ["path", "exe", "args", "pid", "uri"] {
        assert!(
            !game.to_lowercase().contains(&format!("{forbidden}:")),
            "GameDto.{forbidden}"
        );
    }
}
