//! Acesso minimo da sessao a biblioteca persistida: resolver um jogo manual no `LaunchTarget` e
//! gravar `lastPlayedAt`. A fusao com as lojas e a varredura sao do BE-29; ate la, so os jogos
//! manuais de `library.json` resolvem e os ids de loja respondem `UnknownGame`.

use std::sync::{Arc, Mutex, PoisonError};

use crate::application::ports::{GameResolver, LibraryRepository, PlayedRecorder};
use crate::application::AppError;
use crate::domain::library::{
    check_launch_exe_path, GameId, LaunchPathError, LaunchSpec, LaunchTarget, Origin,
};

pub struct StoredLibraryAccess {
    repo: Arc<dyn LibraryRepository>,
    /// Serializa ler-alterar-gravar (um unico escritor de `library.json`, INV-09).
    write: Mutex<()>,
}

impl StoredLibraryAccess {
    pub fn new(repo: Arc<dyn LibraryRepository>) -> Self {
        Self {
            repo,
            write: Mutex::new(()),
        }
    }
}

/// Pasta que contem o arquivo (separador `\`). `None` se nao ha pasta.
fn parent_dir(path: &str) -> Option<String> {
    path.rsplit_once('\\')
        .map(|(dir, _)| dir.to_owned())
        .filter(|d| !d.is_empty())
}

impl GameResolver for StoredLibraryAccess {
    fn resolve(&self, id: &GameId) -> Result<LaunchTarget, AppError> {
        if id.origin() != Origin::Manual {
            return Err(AppError::UnknownGame); // lojas entram com BE-29
        }
        let lib = self.repo.load().value;
        let record = lib
            .manual_games
            .iter()
            .find(|g| &g.id == id)
            .ok_or(AppError::UnknownGame)?;
        // Revalidacao no lancamento (SEC-18): o arquivo pode ter sido editado a mao.
        match check_launch_exe_path(&record.path) {
            Ok(()) => {}
            Err(LaunchPathError::Network) => return Err(AppError::NetworkPathBlocked),
            Err(LaunchPathError::Invalid) => {
                tracing::warn!(game = %id, "jogo manual com caminho invalido em library.json");
                return Err(AppError::GameFileMissing {
                    game_id: id.clone(),
                });
            }
        }
        let dir = parent_dir(&record.path);
        Ok(LaunchTarget {
            game: id.clone(),
            spec: LaunchSpec::Exe {
                path: record.path.clone(),
                args: Vec::new(), // AF-03: jogo manual nunca leva argumento
                working_dir: dir.clone(),
            },
            install_dir: dir,
        })
    }
}

impl PlayedRecorder for StoredLibraryAccess {
    fn mark_played(&self, id: &GameId, at_unix_ms: i64) {
        let _guard = self.write.lock().unwrap_or_else(PoisonError::into_inner);
        let mut lib = self.repo.load().value;
        lib.overlay.entry(id.clone()).or_default().last_played_at = Some(at_unix_ms);
        if let Err(e) = self.repo.save(&lib) {
            tracing::warn!(game = %id, erro = %e, "nao gravou lastPlayedAt");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::ports::{Loaded, StoreError};
    use crate::domain::library::{ManualGameRecord, StoredLibrary};

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

    fn access(path: &str) -> (StoredLibraryAccess, GameId, Arc<Mem>) {
        let id = GameId::manual_from_canonical_path(path);
        let lib = StoredLibrary {
            manual_games: vec![ManualGameRecord {
                id: id.clone(),
                name: "Jogo".into(),
                path: path.into(),
            }],
            ..StoredLibrary::default()
        };
        let mem = Arc::new(Mem(Mutex::new(lib)));
        (StoredLibraryAccess::new(mem.clone()), id, mem)
    }

    #[test]
    fn manual_resolve_sem_argumentos() {
        let (a, id, _) = access(r"C:\Jogos\A & B\game.exe");
        let t = a.resolve(&id).unwrap();
        assert_eq!(t.install_dir.as_deref(), Some(r"C:\Jogos\A & B"));
        match t.spec {
            LaunchSpec::Exe { path, args, .. } => {
                assert_eq!(path, r"C:\Jogos\A & B\game.exe");
                assert!(args.is_empty());
            }
            _ => panic!("esperava Exe"),
        }
    }

    #[test]
    fn caminho_de_rede_editado_a_mao_vira_network_path_blocked() {
        let (a, id, _) = access(r"\\servidor\share\game.exe");
        assert_eq!(a.resolve(&id).unwrap_err(), AppError::NetworkPathBlocked);
    }

    #[test]
    fn script_editado_a_mao_nao_inicia() {
        let (a, id, _) = access(r"C:\Jogos\script.bat");
        assert!(matches!(
            a.resolve(&id).unwrap_err(),
            AppError::GameFileMissing { .. }
        ));
    }

    #[test]
    fn id_de_loja_e_desconhecido_ate_o_be_29() {
        let (a, _, _) = access(r"C:\Jogos\game.exe");
        let steam = GameId::parse("steam:620").unwrap();
        assert_eq!(a.resolve(&steam).unwrap_err(), AppError::UnknownGame);
    }

    #[test]
    fn mark_played_grava_no_overlay() {
        let (a, id, mem) = access(r"C:\Jogos\game.exe");
        a.mark_played(&id, 1_794_610_000_000);
        let lib = mem.0.lock().unwrap();
        assert_eq!(lib.overlay[&id].last_played_at, Some(1_794_610_000_000));
    }
}
