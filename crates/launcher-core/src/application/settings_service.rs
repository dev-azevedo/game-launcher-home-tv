//! Configuracoes do usuario em memoria (BE-45, ARQ-BUG-13): a regra de "alterar uma copia, gravar
//! e so entao trocar a memoria" mora aqui, no nucleo, e nao no composition root. Grava sempre por
//! `SettingsRepository` (dono unico de `settings.json`, INV-03) e nunca toca na secao `advanced`
//! (SEC-28 rev. 5: `advanced_raw` volta intacto ao arquivo).
//!
//! Implementa `SettingsStore`, o port que `DisplayService` e `AutostartService` consomem.

use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use crate::domain::settings::Settings;

use super::error::AppError;
use super::ports::{SettingsRepository, SettingsStore};

pub struct SettingsService {
    repo: Arc<dyn SettingsRepository>,
    current: Mutex<Settings>,
}

impl SettingsService {
    /// `initial` e o que o repositorio devolveu na carga.
    pub fn new(repo: Arc<dyn SettingsRepository>, initial: Settings) -> Self {
        Self {
            repo,
            current: Mutex::new(initial),
        }
    }

    fn lock(&self) -> MutexGuard<'_, Settings> {
        // Mutex envenenado: um panico no meio de uma troca de copia nao deixa a memoria a frente
        // do arquivo (a troca so acontece depois de gravar), entao segue com o que ha.
        self.current.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Copia das configuracoes de agora (inclui `advanced_raw`).
    pub fn snapshot(&self) -> Settings {
        self.lock().clone()
    }

    /// Aplica `change` numa copia, grava e so entao troca a copia em memoria: falha de disco nao
    /// deixa a memoria a frente do arquivo. A gravacao roda com o `Mutex` preso de proposito: duas
    /// alteracoes concorrentes nao podem gravar uma sobre a outra com copias velhas.
    fn update(&self, change: impl FnOnce(&mut Settings)) -> Result<(), AppError> {
        let mut guard = self.lock();
        let mut next = guard.clone();
        change(&mut next);
        self.repo.save(&next).map_err(|error| {
            tracing::warn!(%error, "settings.json nao gravado");
            AppError::Internal
        })?;
        *guard = next;
        Ok(())
    }
}

impl SettingsStore for SettingsService {
    fn monitor_id(&self) -> Option<String> {
        self.lock().monitor_id.clone()
    }

    fn set_monitor_id(&self, id: Option<String>) -> Result<(), AppError> {
        self.update(|s| s.monitor_id = id)
    }

    fn install_defaults_applied(&self) -> bool {
        self.lock().install_defaults_applied
    }

    fn mark_install_defaults_applied(&self) -> Result<(), AppError> {
        self.update(|s| s.install_defaults_applied = true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::ports::{Loaded, StoreError};
    use serde_json::json;

    /// Repositorio em memoria que conta gravacoes e pode falhar.
    #[derive(Default)]
    struct Repo {
        saved: Mutex<Vec<Settings>>,
        fail: Mutex<bool>,
    }

    impl SettingsRepository for Repo {
        fn load(&self) -> Loaded<Settings> {
            Loaded {
                value: Settings::default(),
                recovery: None,
                warnings: Vec::new(),
            }
        }
        fn save(&self, settings: &Settings) -> Result<(), StoreError> {
            if *self.fail.lock().unwrap() {
                return Err(StoreError::Io(std::io::Error::other("disco cheio")));
            }
            self.saved.lock().unwrap().push(settings.clone());
            Ok(())
        }
    }

    fn service(initial: Settings) -> (SettingsService, Arc<Repo>) {
        let repo = Arc::new(Repo::default());
        (SettingsService::new(repo.clone(), initial), repo)
    }

    #[test]
    fn grava_a_copia_alterada_e_so_entao_a_memoria_muda() {
        let (svc, repo) = service(Settings::default());
        assert_eq!(svc.monitor_id(), None);
        svc.set_monitor_id(Some("TV".into())).unwrap();
        assert_eq!(svc.monitor_id().as_deref(), Some("TV"));
        let saved = repo.saved.lock().unwrap();
        assert_eq!(saved.len(), 1);
        assert_eq!(saved[0].monitor_id.as_deref(), Some("TV"));
    }

    #[test]
    fn falha_de_disco_devolve_internal_e_a_memoria_fica_como_estava() {
        let (svc, repo) = service(Settings {
            monitor_id: Some("A".into()),
            ..Settings::default()
        });
        *repo.fail.lock().unwrap() = true;
        assert_eq!(
            svc.set_monitor_id(Some("B".into())),
            Err(AppError::Internal)
        );
        assert_eq!(svc.monitor_id().as_deref(), Some("A"));
        assert_eq!(svc.mark_install_defaults_applied(), Err(AppError::Internal));
        assert!(!svc.install_defaults_applied());
        assert!(repo.saved.lock().unwrap().is_empty());

        // O disco volta: a mesma alteracao passa.
        *repo.fail.lock().unwrap() = false;
        svc.set_monitor_id(Some("B".into())).unwrap();
        assert_eq!(svc.monitor_id().as_deref(), Some("B"));
    }

    #[test]
    fn a_secao_advanced_volta_intacta_ao_gravar() {
        let advanced = json!({ "session": { "endGraceSeconds": 7 }, "desconhecida": [1, 2, 3] });
        let (svc, repo) = service(Settings {
            advanced_raw: advanced.clone(),
            ..Settings::default()
        });
        svc.set_monitor_id(Some("TV".into())).unwrap();
        svc.mark_install_defaults_applied().unwrap();
        for saved in repo.saved.lock().unwrap().iter() {
            assert_eq!(saved.advanced_raw, advanced);
        }
        assert_eq!(svc.snapshot().advanced_raw, advanced);
    }

    #[test]
    fn marcar_o_padrao_do_instalador_e_gravado_e_lembrado() {
        let (svc, repo) = service(Settings::default());
        assert!(!svc.install_defaults_applied());
        svc.mark_install_defaults_applied().unwrap();
        assert!(svc.install_defaults_applied());
        assert!(repo.saved.lock().unwrap()[0].install_defaults_applied);
    }

    #[test]
    fn snapshot_e_uma_copia_e_nao_acompanha_alteracoes_depois() {
        let (svc, _) = service(Settings::default());
        let antes = svc.snapshot();
        svc.set_monitor_id(Some("TV".into())).unwrap();
        assert_eq!(antes.monitor_id, None);
        assert_eq!(svc.snapshot().monitor_id.as_deref(), Some("TV"));
    }
}
