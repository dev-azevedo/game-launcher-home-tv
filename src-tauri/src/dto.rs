//! DTOs da IPC (contrato `docs/api/launcher-tv.md` secoes 1 e 3).
//!
//! Saidas: `Serialize` + `TS`. Entradas (`*Args`): `Deserialize` com `deny_unknown_fields`, campo
//! ausente = erro (inclusive nos `Option`), e cada valor validado na borda (SEC-03, SEC-06).
//! Os tipos TypeScript saem destes structs (`cargo test -p sofa` regenera
//! `ui/src/shared/ipc/generated/`); nada ali e editado a mao.

use launcher_core::domain::library::{GameId, Origin};
use serde::de::{self, Deserializer};
use serde::ser::{SerializeMap, Serializer};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

// ----------------------------------------------------------------------------------- enums

macro_rules! wire_enum {
    ($(#[$m:meta])* $name:ident { $($variant:ident),+ $(,)? }) => {
        $(#[$m])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
        pub enum $name { $($variant),+ }
    };
}

wire_enum!(SessionState {
    Idle,
    Launching,
    NeedsElevation,
    NotConfirmed,
    Running,
    Ending,
    Closing,
    CloseOverdue
});
wire_enum!(InputIntent {
    Up,
    Down,
    Left,
    Right,
    Confirm,
    Back,
    Menu,
    View,
    PagePrev,
    PageNext,
    VolumeDown,
    VolumeUp
});
wire_enum!(PowerAction {
    Shutdown,
    Restart,
    Suspend
});
wire_enum!(Availability {
    Available,
    Unavailable
});
wire_enum!(VolumeDirection { Up, Down });
wire_enum!(FrontOutcome {
    Front,
    FrontAfterMinimize,
    Failed
});
wire_enum!(AutostartMechanism { Task, Run, None });
wire_enum!(DirEntryKind { Drive, Dir, Exe });
wire_enum!(LaunchFailure { ElevationDenied });
wire_enum!(ErrorCode {
    Busy,
    InvalidRequest,
    Internal,
    Duplicate,
    NotAnExecutable,
    PathNotAllowed,
    NetworkPathBlocked,
    NotFound,
    AccessDenied,
    EmptyName,
    UnknownGame,
    SessionAlreadyActive,
    GameFileMissing,
    StoreNotInstalled,
    NeedsElevation,
    ElevationDenied,
    InvalidState,
    NoActiveSession,
    ForegroundDenied,
    NotOverdue,
    NoAudioDevice,
    Unavailable,
    TicketInvalid,
    Rejected,
    AutostartFailed,
    UnknownMonitor
});

/// `Origin` do contrato. O nome TS e `Origin`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(rename = "Origin")]
pub enum OriginDto {
    Steam,
    Epic,
    Gog,
    Manual,
}

impl From<Origin> for OriginDto {
    fn from(o: Origin) -> Self {
        match o {
            Origin::Steam => Self::Steam,
            Origin::Epic => Self::Epic,
            Origin::Gog => Self::Gog,
            Origin::Manual => Self::Manual,
        }
    }
}

// ---------------------------------------------------------------------------------- saidas

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct GameDto {
    #[ts(type = "string")]
    pub id: String,
    pub name: String,
    pub original_name: Option<String>,
    pub origin: OriginDto,
    pub cover: String,
    #[ts(type = "number | null")]
    pub last_played_at: Option<i64>,
    pub in_progress: bool,
}

#[derive(Debug, Clone, Default, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct LibrarySnapshot {
    pub games: Vec<GameDto>,
    #[ts(type = "string | null")]
    pub focus_game_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct SessionDto {
    pub state: SessionState,
    #[ts(type = "string | null")]
    pub game_id: Option<String>,
    #[ts(type = "number | null")]
    pub since: Option<i64>,
    pub dismissed: bool,
    pub elevated: bool,
    pub minimized: bool,
    pub launch_failure: Option<LaunchFailure>,
}

impl SessionDto {
    pub fn idle() -> Self {
        Self {
            state: SessionState::Idle,
            game_id: None,
            since: None,
            dismissed: false,
            elevated: false,
            minimized: false,
            launch_failure: None,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct VolumeState {
    pub level: u8,
    pub muted: bool,
    pub available: bool,
}

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct MonitorDto {
    #[ts(type = "string")]
    pub id: String,
    pub name: String,
    pub primary: bool,
    pub current: bool,
}

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct SettingsDto {
    pub autostart: bool,
    pub autostart_mechanism: AutostartMechanism,
    #[ts(type = "string | null")]
    pub monitor_id: Option<String>,
    pub log_dir: String,
}

#[derive(Debug, Clone, Serialize, TS)]
pub struct DirEntry {
    pub name: String,
    pub kind: DirEntryKind,
}

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct DirListing {
    pub path: Option<String>,
    pub parent: Option<String>,
    pub entries: Vec<DirEntry>,
    pub truncated: bool,
}

#[derive(Debug, Clone, Copy, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct PowerCapabilities {
    pub shutdown: Availability,
    pub restart: Availability,
    pub suspend: Availability,
}

#[derive(Debug, Clone, Copy, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct InputStatus {
    pub connected: u8,
}

// ---------------------------------------------------------------------------------- eventos

#[derive(Debug, Clone, Copy, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct InputIntentEvent {
    pub intent: InputIntent,
    pub repeat: bool,
}

#[derive(Debug, Clone, Copy, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ControllersEvent {
    pub connected: u8,
}

#[derive(Debug, Clone, Copy, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct SessionPanelEvent {
    pub open: bool,
    pub outcome: FrontOutcome,
}

// ------------------------------------------------------------------------------------ erro

#[derive(Debug, Clone, Default, PartialEq, Eq, TS)]
#[ts(rename_all = "camelCase")]
pub struct ErrorParams {
    #[ts(type = "string", optional)]
    pub game_id: Option<String>,
    #[ts(optional)]
    pub origin: Option<OriginDto>,
}

// Serializacao manual: omite campos ausentes (o ts-rs nao entende `skip_serializing_if`) e fixa
// os nomes `gameId` e `origin` do contrato.
impl Serialize for ErrorParams {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let len = usize::from(self.game_id.is_some()) + usize::from(self.origin.is_some());
        let mut map = serializer.serialize_map(Some(len))?;
        if let Some(id) = &self.game_id {
            map.serialize_entry("gameId", id)?;
        }
        if let Some(origin) = &self.origin {
            map.serialize_entry("origin", origin)?;
        }
        map.end()
    }
}

/// `ErrorDto { code, params? }`: unica forma de erro que cruza a IPC (SEC-06). `params` leva so
/// `gameId` e `origin`, nunca caminho nem texto do sistema.
#[derive(Debug, Clone, PartialEq, Eq, TS)]
pub struct ErrorDto {
    pub code: ErrorCode,
    #[ts(optional)]
    pub params: Option<ErrorParams>,
}

impl Serialize for ErrorDto {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let len = 1 + usize::from(self.params.is_some());
        let mut map = serializer.serialize_map(Some(len))?;
        map.serialize_entry("code", &self.code)?;
        if let Some(params) = &self.params {
            map.serialize_entry("params", params)?;
        }
        map.end()
    }
}

// -------------------------------------------------------------------------------- entradas

const MAX_PATH_CHARS: usize = 1024;
const MAX_NAME_CHARS: usize = 1024; // bruto; o nucleo normaliza e corta em 256 (SEC-07)
const MAX_MONITOR_ID: usize = 256;

fn de_game_id<'de, D: Deserializer<'de>>(d: D) -> Result<String, D::Error> {
    let raw = String::deserialize(d)?;
    GameId::parse(&raw)
        .map(|id| id.to_string())
        .map_err(de::Error::custom)
}

fn de_ticket<'de, D: Deserializer<'de>>(d: D) -> Result<String, D::Error> {
    let raw = String::deserialize(d)?;
    let ok = (22..=64).contains(&raw.len())
        && raw
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-');
    if ok {
        Ok(raw)
    } else {
        Err(de::Error::custom("ticket mal formado"))
    }
}

/// `null` ou texto; qualquer outra coisa e erro. Le como `Value` de proposito: com
/// `Option::deserialize` o serde trata campo ausente como `None`, e o contrato diz que ausente e
/// `InvalidRequest`.
fn null_or_string<'de, D: Deserializer<'de>>(d: D) -> Result<Option<String>, D::Error> {
    match serde_json::Value::deserialize(d)? {
        serde_json::Value::Null => Ok(None),
        serde_json::Value::String(s) => Ok(Some(s)),
        _ => Err(de::Error::custom("esperado texto ou null")),
    }
}

fn de_monitor_id<'de, D: Deserializer<'de>>(d: D) -> Result<Option<String>, D::Error> {
    match null_or_string(d)? {
        None => Ok(None),
        Some(s) if (1..=MAX_MONITOR_ID).contains(&s.chars().count()) => Ok(Some(s)),
        Some(_) => Err(de::Error::custom("monitorId fora do limite")),
    }
}

fn de_manual_path<'de, D: Deserializer<'de>>(d: D) -> Result<String, D::Error> {
    let s = String::deserialize(d)?;
    let n = s.chars().count();
    if (1..=MAX_PATH_CHARS).contains(&n) && !s.contains('\0') {
        Ok(s)
    } else {
        Err(de::Error::custom("caminho fora do limite"))
    }
}

fn de_list_path<'de, D: Deserializer<'de>>(d: D) -> Result<Option<String>, D::Error> {
    match null_or_string(d)? {
        None => Ok(None),
        Some(s) if s.chars().count() <= MAX_PATH_CHARS && !s.contains('\0') => Ok(Some(s)),
        Some(_) => Err(de::Error::custom("caminho fora do limite")),
    }
}

fn de_raw_name<'de, D: Deserializer<'de>>(d: D) -> Result<String, D::Error> {
    let s = String::deserialize(d)?;
    if s.chars().count() <= MAX_NAME_CHARS {
        Ok(s)
    } else {
        Err(de::Error::custom("nome grande demais"))
    }
}

macro_rules! wire_value {
    ($(#[$m:meta])* $name:ident($inner:ty), $func:ident) => {
        $(#[$m])*
        #[derive(Debug, Clone, PartialEq, Eq)]
        pub struct $name(pub $inner);

        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
                $func(d).map(Self)
            }
        }
    };
}

wire_value!(
    /// `GameId` bem formado, na forma canonica `origem:chave`.
    WireGameId(String),
    de_game_id
);
wire_value!(
    /// Ticket opaco `^[A-Za-z0-9_-]{22,64}$`.
    WireTicket(String),
    de_ticket
);
wire_value!(
    /// `MonitorId` 1..256 ou `null`; ausente e erro.
    WireMonitorId(Option<String>), de_monitor_id
);
wire_value!(
    /// Caminho de `library_add_manual`: 1..1024 caracteres.
    WireManualPath(String),
    de_manual_path
);
wire_value!(
    /// Caminho de `fs_list_dir`: `null` ou ate 1024 caracteres; ausente e erro.
    WireListPath(Option<String>), de_list_path
);
wire_value!(
    /// Nome bruto (o nucleo normaliza e corta em 256, SEC-07).
    WireRawName(String),
    de_raw_name
);

#[derive(Debug, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GameIdArgs {
    #[ts(type = "string")]
    pub game_id: WireGameId,
}

#[derive(Debug, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RenameArgs {
    #[ts(type = "string")]
    pub game_id: WireGameId,
    #[ts(type = "string")]
    pub name: WireRawName,
}

#[derive(Debug, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AddManualArgs {
    #[ts(type = "string")]
    pub path: WireManualPath,
}

#[derive(Debug, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ListDirArgs {
    #[ts(type = "string | null")]
    pub path: WireListPath,
}

#[derive(Debug, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct VolumeStepArgs {
    pub direction: VolumeDirection,
}

#[derive(Debug, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PowerPrepareArgs {
    pub action: PowerAction,
}

#[derive(Debug, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PowerExecuteArgs {
    #[ts(type = "string")]
    pub ticket: WireTicket,
}

#[derive(Debug, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SetAutostartArgs {
    pub enabled: bool,
}

#[derive(Debug, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SetMonitorArgs {
    #[ts(type = "string | null")]
    pub monitor_id: WireMonitorId,
}
