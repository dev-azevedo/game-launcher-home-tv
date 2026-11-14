//! Inicio do app: pasta de dados, logs e configuracoes (ADR-0004, SEC-18, SEC-28).

use std::path::PathBuf;
use std::sync::Arc;

use launcher_core::application::ports::{
    Clock, CoverCache, FolderBrowser, LibraryEvents, LibraryRepository, ManualExeInspector,
    ScanCache, SettingsRepository, StoreScanner,
};
use launcher_core::application::{LibraryDeps, LibraryService};
use launcher_core::domain::settings::{LogLevel, Step2Key};
use launcher_win::cover_cache::WinCoverCache;
use launcher_win::folder_browser::WinFolderBrowser;
use launcher_win::logging::{self, Level};
use launcher_win::manual_exe::WinManualExeInspector;
use launcher_win::path_canon::known;
use launcher_win::persistence::{JsonLibraryRepository, JsonScanCache, JsonSettingsRepository};
use launcher_win::stores::{EpicScanner, GogScanner, SteamScanner, StoreEnv, WinRegistry};
use launcher_win::system_clock::SystemClock;
use tauri::{AppHandle, Manager, Runtime};

use crate::events::TauriLibraryEvents;
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

/// Liga a biblioteca (BE-29): repositorio JSON, as tres lojas, cadastro manual, navegador de
/// pastas e o cache de capas. Precisa so da pasta de dados (nao da janela), por isso roda antes
/// dela e antes do runtime de sessao. A primeira varredura so comeca na primeira leitura da UI
/// (BE-40); ate la a biblioteca responde com o que e do usuario.
pub fn init_library<R: Runtime>(app: &AppHandle<R>) -> Result<(), String> {
    let state = app.state::<AppState>();
    let user_data = state
        .user_data
        .get()
        .ok_or("pasta de dados nao inicializada")?;
    let clock: Arc<dyn Clock> = Arc::new(SystemClock::new());
    let repo: Arc<dyn LibraryRepository> = Arc::new(JsonLibraryRepository::new(
        &user_data.data_dir,
        Arc::clone(&clock),
    ));
    let env = Arc::new(StoreEnv::system());
    let registry = Arc::new(WinRegistry);
    let scanners: Vec<Arc<dyn StoreScanner>> = vec![
        Arc::new(SteamScanner::new(registry.clone(), Arc::clone(&env))),
        Arc::new(EpicScanner::new(
            known::epic_manifests_dir(),
            Arc::clone(&env),
        )),
        Arc::new(GogScanner::new(registry, env)),
    ];
    let inspector: Arc<dyn ManualExeInspector> = Arc::new(WinManualExeInspector);
    let browser: Arc<dyn FolderBrowser> = Arc::new(WinFolderBrowser);
    let events: Arc<dyn LibraryEvents> = Arc::new(TauriLibraryEvents::new(app.clone()));
    let covers: Arc<dyn CoverCache> = Arc::new(WinCoverCache::new(&user_data.data_dir));
    let scan_cache: Arc<dyn ScanCache> = Arc::new(JsonScanCache::new(&user_data.data_dir));

    let library = Arc::new(LibraryService::new(LibraryDeps {
        repo,
        scanners,
        inspector,
        browser,
        events,
        scan_cache,
    }));
    let _ = state.covers.set(covers);
    let _ = state.library.set(library);
    tracing::info!("biblioteca ligada");
    Ok(())
}
