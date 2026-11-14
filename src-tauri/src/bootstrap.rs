//! Inicio do app: pasta de dados, logs e configuracoes (ADR-0004, SEC-18, SEC-28).

use std::path::PathBuf;
use std::sync::Arc;

use launcher_core::application::ports::{Clock, SettingsRepository};
use launcher_core::domain::settings::{LogLevel, Step2Key};
use launcher_win::logging::{self, Level};
use launcher_win::persistence::JsonSettingsRepository;
use launcher_win::system_clock::SystemClock;
use tauri::{Manager, Runtime};

use crate::state::{AppState, UserData};

/// Resolve `%LOCALAPPDATA%\app.sofa.launcher`, liga o log em `logs\`, carrega `settings.json`
/// e guarda tudo no `AppState`. Falha de log nao derruba o launcher.
pub fn init_user_data<R: Runtime>(app: &impl Manager<R>) -> tauri::Result<()> {
    let data_dir: PathBuf = app.path().app_local_data_dir()?;
    let clock: Arc<dyn Clock> = Arc::new(SystemClock::new());
    let repo: Arc<dyn SettingsRepository> =
        Arc::new(JsonSettingsRepository::new(&data_dir, Arc::clone(&clock)));

    let loaded = repo.load();
    let advanced = loaded.value.advanced();
    let level = match advanced.settings.log_level {
        LogLevel::Info => Level::Info,
        LogLevel::Debug => Level::Debug,
    };
    if let Err(e) = logging::init(&data_dir.join("logs"), level) {
        // Sem arquivo de log o app segue; o erro vai para o stderr de quem iniciou.
        eprintln!("Sofa: log em arquivo indisponivel: {e}");
    }

    tracing::info!(version = env!("CARGO_PKG_VERSION"), "Sofa iniciado");
    if let Some(r) = &loaded.recovery {
        tracing::warn!(reason = ?r.reason, backup = ?r.backup_name, "settings.json recriado com os padroes");
    }
    for w in &advanced.warnings {
        tracing::warn!(aviso = %w, "configuracao avancada corrigida para o padrao");
    }
    // SEC-28: degraus 2 e 3 ligados aparecem no log INFO a cada abertura, com a tecla.
    let fg = &advanced.settings.foreground;
    if fg.step2 || fg.step3 {
        let key = match fg.step2_key {
            Step2Key::F24 => "F24",
            Step2Key::Alt => "Alt",
        };
        tracing::info!(step2 = fg.step2, step3 = fg.step3, step2_key = key, order = ?fg.order, "escada de primeiro plano: degraus 2/3 ativos");
    }

    let state = app.state::<AppState>();
    *state
        .settings
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner) = loaded.value;
    let _ = state.user_data.set(UserData {
        data_dir,
        settings_repo: repo,
    });
    Ok(())
}
