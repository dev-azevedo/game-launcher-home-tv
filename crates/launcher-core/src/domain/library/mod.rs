//! Biblioteca: identidade dos jogos (BE-10) e formato persistido (BE-11). Fusao, ordenacao e
//! politicas de seguranca entram com BE-22 e BE-23.

mod id;
pub mod stored;

pub use id::{GameId, GameIdError, Origin};
pub use stored::{
    parse_library, serialize_library, LibraryFileError, ManualGameRecord, OverlayEntry,
    StoredLibrary,
};
