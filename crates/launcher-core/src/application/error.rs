//! `AppError`: erro de aplicacao com os 28 codigos do contrato (secao 5 de
//! `docs/api/launcher-tv.md`). A apresentacao mapeia para `ErrorDto` num lugar so (SEC-06).
//!
//! `Display` serve ao log: nunca leva caminho nem texto do sistema. `params` do contrato so
//! carregam `GameId` e `Origin`.

use crate::domain::library::{GameId, Origin};

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum AppError {
    #[error("acao serializada em andamento")]
    Busy,
    #[error("requisicao invalida")]
    InvalidRequest,
    #[error("falha interna")]
    Internal,
    #[error("jogo ja cadastrado")]
    Duplicate,
    #[error("arquivo nao e um executavel valido")]
    NotAnExecutable,
    #[error("local nao permitido")]
    PathNotAllowed,
    #[error("caminho de rede bloqueado ao iniciar")]
    NetworkPathBlocked,
    #[error("nao encontrado")]
    NotFound,
    #[error("acesso negado")]
    AccessDenied,
    #[error("nome vazio")]
    EmptyName,
    #[error("jogo desconhecido")]
    UnknownGame,
    #[error("ja ha sessao ativa: {game_id}")]
    SessionAlreadyActive { game_id: GameId },
    #[error("arquivo do jogo ausente: {game_id}")]
    GameFileMissing { game_id: GameId },
    #[error("loja nao instalada: {}", origin.prefix())]
    StoreNotInstalled { origin: Origin },
    #[error("jogo exige administrador: {game_id}")]
    NeedsElevation { game_id: GameId },
    #[error("elevacao recusada: {game_id}")]
    ElevationDenied { game_id: GameId },
    #[error("comando fora do estado permitido")]
    InvalidState,
    #[error("nao ha jogo aberto")]
    NoActiveSession,
    #[error("primeiro plano negado")]
    ForegroundDenied,
    #[error("fechamento ainda nao esta atrasado")]
    NotOverdue,
    #[error("sem dispositivo de audio")]
    NoAudioDevice,
    #[error("acao indisponivel")]
    Unavailable,
    #[error("ticket invalido")]
    TicketInvalid,
    #[error("pedido recusado pelo Windows")]
    Rejected,
    #[error("falha no autostart")]
    AutostartFailed,
    #[error("monitor desconhecido")]
    UnknownMonitor,
    #[error("atualizacao do app falhou")]
    UpdateFailed,
    #[error("instalador da atualizacao recusado")]
    UpdateCancelled,
}

/// Os 28 codigos do contrato, na ordem da secao 5.
pub const ALL_CODES: [&str; 28] = [
    "Busy",
    "InvalidRequest",
    "Internal",
    "Duplicate",
    "NotAnExecutable",
    "PathNotAllowed",
    "NetworkPathBlocked",
    "NotFound",
    "AccessDenied",
    "EmptyName",
    "UnknownGame",
    "SessionAlreadyActive",
    "GameFileMissing",
    "StoreNotInstalled",
    "NeedsElevation",
    "ElevationDenied",
    "InvalidState",
    "NoActiveSession",
    "ForegroundDenied",
    "NotOverdue",
    "NoAudioDevice",
    "Unavailable",
    "TicketInvalid",
    "Rejected",
    "AutostartFailed",
    "UnknownMonitor",
    "UpdateFailed",
    "UpdateCancelled",
];

impl AppError {
    /// Codigo estavel do contrato (`ErrorDto.code`).
    pub fn code(&self) -> &'static str {
        match self {
            AppError::Busy => "Busy",
            AppError::InvalidRequest => "InvalidRequest",
            AppError::Internal => "Internal",
            AppError::Duplicate => "Duplicate",
            AppError::NotAnExecutable => "NotAnExecutable",
            AppError::PathNotAllowed => "PathNotAllowed",
            AppError::NetworkPathBlocked => "NetworkPathBlocked",
            AppError::NotFound => "NotFound",
            AppError::AccessDenied => "AccessDenied",
            AppError::EmptyName => "EmptyName",
            AppError::UnknownGame => "UnknownGame",
            AppError::SessionAlreadyActive { .. } => "SessionAlreadyActive",
            AppError::GameFileMissing { .. } => "GameFileMissing",
            AppError::StoreNotInstalled { .. } => "StoreNotInstalled",
            AppError::NeedsElevation { .. } => "NeedsElevation",
            AppError::ElevationDenied { .. } => "ElevationDenied",
            AppError::InvalidState => "InvalidState",
            AppError::NoActiveSession => "NoActiveSession",
            AppError::ForegroundDenied => "ForegroundDenied",
            AppError::NotOverdue => "NotOverdue",
            AppError::NoAudioDevice => "NoAudioDevice",
            AppError::Unavailable => "Unavailable",
            AppError::TicketInvalid => "TicketInvalid",
            AppError::Rejected => "Rejected",
            AppError::AutostartFailed => "AutostartFailed",
            AppError::UnknownMonitor => "UnknownMonitor",
            AppError::UpdateFailed => "UpdateFailed",
            AppError::UpdateCancelled => "UpdateCancelled",
        }
    }

    /// `gameId` de `params`, quando o codigo carrega um.
    pub fn game_id(&self) -> Option<&GameId> {
        match self {
            AppError::SessionAlreadyActive { game_id }
            | AppError::GameFileMissing { game_id }
            | AppError::NeedsElevation { game_id }
            | AppError::ElevationDenied { game_id } => Some(game_id),
            _ => None,
        }
    }

    /// `origin` de `params`, quando o codigo carrega um.
    pub fn origin(&self) -> Option<Origin> {
        match self {
            AppError::StoreNotInstalled { origin } => Some(*origin),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;
    use std::fs;
    use std::path::PathBuf;

    fn gid() -> GameId {
        GameId::parse("steam:620").unwrap()
    }

    fn one_of_each() -> Vec<AppError> {
        vec![
            AppError::Busy,
            AppError::InvalidRequest,
            AppError::Internal,
            AppError::Duplicate,
            AppError::NotAnExecutable,
            AppError::PathNotAllowed,
            AppError::NetworkPathBlocked,
            AppError::NotFound,
            AppError::AccessDenied,
            AppError::EmptyName,
            AppError::UnknownGame,
            AppError::SessionAlreadyActive { game_id: gid() },
            AppError::GameFileMissing { game_id: gid() },
            AppError::StoreNotInstalled {
                origin: Origin::Epic,
            },
            AppError::NeedsElevation { game_id: gid() },
            AppError::ElevationDenied { game_id: gid() },
            AppError::InvalidState,
            AppError::NoActiveSession,
            AppError::ForegroundDenied,
            AppError::NotOverdue,
            AppError::NoAudioDevice,
            AppError::Unavailable,
            AppError::TicketInvalid,
            AppError::Rejected,
            AppError::AutostartFailed,
            AppError::UnknownMonitor,
            AppError::UpdateFailed,
            AppError::UpdateCancelled,
        ]
    }

    #[test]
    fn codigos_de_app_error_sao_os_28_do_contrato_em_ordem() {
        let codes: Vec<&str> = one_of_each().iter().map(AppError::code).collect();
        assert_eq!(codes, ALL_CODES);
    }

    /// Guard: a tabela da secao 5 do contrato e `ALL_CODES` dizem a mesma coisa.
    #[test]
    fn contrato_secao_5_lista_exatamente_os_codigos() {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../docs/api/launcher-tv.md");
        let doc = fs::read_to_string(path).expect("contrato de API");
        let section = doc
            .split("## 5. ")
            .nth(1)
            .and_then(|s| s.split("\n## 6.").next())
            .expect("secao 5");
        let in_doc: BTreeSet<String> = section
            .lines()
            .filter_map(|l| l.strip_prefix("| `"))
            .filter_map(|l| l.split('`').next())
            .map(str::to_owned)
            .collect();
        let in_code: BTreeSet<String> = ALL_CODES.iter().map(|s| (*s).to_owned()).collect();
        assert_eq!(in_doc, in_code);
    }

    #[test]
    fn params_so_carregam_game_id_e_origin() {
        let e = AppError::SessionAlreadyActive { game_id: gid() };
        assert_eq!(e.game_id().unwrap().to_string(), "steam:620");
        assert_eq!(e.origin(), None);
        let e = AppError::StoreNotInstalled {
            origin: Origin::Epic,
        };
        assert_eq!(e.origin(), Some(Origin::Epic));
        assert!(e.game_id().is_none());
        assert!(AppError::Busy.game_id().is_none());
    }

    #[test]
    fn display_nao_vaza_caminho() {
        for e in one_of_each() {
            let text = e.to_string();
            assert!(!text.contains('\\'), "{text}");
        }
    }
}
