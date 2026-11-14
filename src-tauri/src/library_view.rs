//! Da biblioteca do nucleo para os DTOs da IPC (contrato secao 1: `GameDto`, `LibrarySnapshot`,
//! `DirListing`). Sem regra de negocio: o nome exibido, a ordem e o filtro de ocultos ja vem do
//! dominio. `GameDto` nunca leva caminho (SEC-03).

use launcher_core::application::LibraryState;
use launcher_core::domain::library::{
    DirEntryKind as CoreKind, DirListing as CoreListing, GameId, LibraryEntry,
};

use crate::dto::{DirEntry, DirEntryKind, DirListing, GameDto, LibrarySnapshot, OriginDto};
use crate::state::AppState;

/// `GameDto` de um item. `in_progress`: e o jogo da sessao ativa (selo "Em andamento", CA-36).
pub fn game_dto(entry: &LibraryEntry, in_progress: bool) -> GameDto {
    GameDto {
        id: entry.id.to_string(),
        name: entry.display_name().to_owned(),
        // So ha `originalName` quando o usuario renomeou (habilita "Restaurar nome", CA-62).
        original_name: entry
            .custom_name
            .as_ref()
            .map(|_| entry.original_name.clone()),
        origin: OriginDto::from(entry.id.origin()),
        cover: format!("cover://{}", entry.id),
        last_played_at: entry.last_played_at,
        in_progress,
    }
}

/// O jogo da sessao ativa, se ha uma.
fn active_game(state: &AppState) -> Option<GameId> {
    state.session.get()?.get().snapshot.game_id
}

/// `GameDto` de um item com o selo "Em andamento" do estado atual da sessao.
pub fn game_dto_for(state: &AppState, entry: &LibraryEntry) -> GameDto {
    game_dto(entry, active_game(state).as_ref() == Some(&entry.id))
}

/// `LibrarySnapshot` sem itens ocultos, na ordem da RN-05. `focus`: o jogo em foco (ultimo jogado
/// na abertura, CA-22; o que acabou de fechar no fim da sessao, CA-30).
pub fn snapshot_dto(
    state: &AppState,
    lib: &LibraryState,
    focus: Option<&GameId>,
) -> LibrarySnapshot {
    let active = active_game(state);
    LibrarySnapshot {
        games: lib
            .visible()
            .map(|e| game_dto(e, active.as_ref() == Some(&e.id)))
            .collect(),
        focus_game_id: focus.map(ToString::to_string),
    }
}

/// Itens ocultos (CA-70).
pub fn hidden_dto(state: &AppState, lib: &LibraryState) -> Vec<GameDto> {
    let active = active_game(state);
    lib.hidden()
        .map(|e| game_dto(e, active.as_ref() == Some(&e.id)))
        .collect()
}

impl From<CoreListing> for DirListing {
    fn from(l: CoreListing) -> Self {
        Self {
            path: l.path,
            parent: l.parent,
            entries: l
                .entries
                .into_iter()
                .map(|e| DirEntry {
                    name: e.name,
                    kind: match e.kind {
                        CoreKind::Drive => DirEntryKind::Drive,
                        CoreKind::Dir => DirEntryKind::Dir,
                        CoreKind::Exe => DirEntryKind::Exe,
                    },
                })
                .collect(),
            truncated: l.truncated,
        }
    }
}
