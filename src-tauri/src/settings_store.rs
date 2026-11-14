//! `SettingsStore` sobre o `settings.json` carregado no `setup` (ADR-0004): grava a preferencia de
//! monitor e a marca de `InstallDefaults` aplicado, sempre por `SettingsRepository` (dono unico do
//! arquivo, INV-03) e sem tocar na secao `advanced` (SEC-28 rev. 5, `advanced_raw` intacto).

use launcher_core::application::ports::SettingsStore;
use launcher_core::application::AppError;
use launcher_core::domain::settings::Settings;
use tauri::{AppHandle, Manager, Runtime};

use crate::state::AppState;

pub struct AppSettingsStore<R: Runtime> {
    app: AppHandle<R>,
}

impl<R: Runtime> AppSettingsStore<R> {
    pub fn new(app: AppHandle<R>) -> Self {
        Self { app }
    }

    /// Aplica `change` numa copia, grava e so entao troca a copia em memoria: falha de disco nao
    /// deixa a memoria a frente do arquivo.
    fn update(&self, change: impl FnOnce(&mut Settings)) -> Result<(), AppError> {
        let state = self.app.state::<AppState>();
        let user_data = state.user_data.get().ok_or(AppError::Internal)?;
        let mut guard = state
            .settings
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let mut next = guard.clone();
        change(&mut next);
        user_data.settings_repo.save(&next).map_err(|error| {
            tracing::warn!(%error, "settings.json nao gravado");
            AppError::Internal
        })?;
        *guard = next;
        Ok(())
    }

    fn read<T>(&self, get: impl FnOnce(&Settings) -> T) -> T {
        let state = self.app.state::<AppState>();
        let guard = state
            .settings
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        get(&guard)
    }
}

impl<R: Runtime> SettingsStore for AppSettingsStore<R> {
    fn monitor_id(&self) -> Option<String> {
        self.read(|s| s.monitor_id.clone())
    }

    fn set_monitor_id(&self, id: Option<String>) -> Result<(), AppError> {
        self.update(|s| s.monitor_id = id)
    }

    fn install_defaults_applied(&self) -> bool {
        self.read(|s| s.install_defaults_applied)
    }

    fn mark_install_defaults_applied(&self) -> Result<(), AppError> {
        self.update(|s| s.install_defaults_applied = true)
    }
}
