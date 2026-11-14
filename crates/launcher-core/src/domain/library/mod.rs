//! Biblioteca: identidade dos jogos (BE-10), formato persistido (BE-11), fusao, ordem e nome
//! exibido (BE-22), politicas de seguranca (BE-23) e validacao do que as lojas detectam.

pub mod detect;
pub mod dir_listing;
pub mod display_name;
pub mod exe_policy;
mod id;
pub mod id_formats;
pub mod install_path_policy;
pub mod launch;
pub mod model;
pub mod steam_non_games;
pub mod stored;

pub use detect::{
    epic_game, gog_game, split_arguments, steam_game, EpicManifestRaw, GogGameRaw, GogInfoRaw,
    GogTaskRaw, Reject, SteamManifestRaw,
};
pub use dir_listing::{
    dir_listing, drives_listing, parent_dir, DirEntry, DirEntryKind, DirListing, RawEntry,
    MAX_DIR_ENTRIES,
};
pub use display_name::{normalize_display_name, MAX_DISPLAY_NAME_CHARS};
pub use exe_policy::{
    check_manual_exe_path, check_store_exe, parse_browse_path, BrowseError, BrowseTarget,
    ManualPathError, StoreExeError,
};
pub use id::{GameId, GameIdError, Origin};
pub use install_path_policy::{
    local_components, normalize_local, resolve_confined, InstallPathPolicy, PathPolicyError,
};
pub use launch::{
    check_launch_exe_path, epic_uri, percent_encode, steam_uri, LaunchPathError, LaunchSpec,
    LaunchTarget,
};
pub use model::{merge_library, sort_entries, sort_key, DetectedGame, LibraryEntry};
pub use steam_non_games::is_steam_non_game;
pub use stored::{
    parse_library, serialize_library, LibraryFileError, ManualGameRecord, OverlayEntry,
    StoredLibrary,
};
