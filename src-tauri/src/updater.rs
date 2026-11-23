//! Atualizacao do app pelo proprio launcher (SEC-33, ADR-0008): adapter do `tauri-plugin-updater`.
//!
//! So o processo Rust fala com a rede: a webview continua sem origem externa na CSP (SEC-01) e a
//! capability `main` nao ganha permissao `updater:` (SEC-04). O endpoint e a chave publica minisign
//! ficam em `tauri.conf.json` (`plugins.updater`); o plugin recusa instalador sem assinatura valida
//! dessa chave e versao menor ou igual a atual.
//!
//! Instalar baixa o instalador NSIS, confere a assinatura e o executa com `/P /UPDATE /R`
//! (passivo, sem apagar dados, reabre o launcher sem elevacao via `RunAsUser`). O Windows pede UAC
//! (RN-26). Se o usuario recusar, o launcher continua aberto e a UI recebe `UpdateCancelled`.

use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use launcher_core::application::AppError;
use launcher_core::domain::session::SessionState;
use tauri::{AppHandle, Emitter, Manager, Runtime};
use tauri_plugin_updater::{Update, Updater, UpdaterExt};

use crate::dto::{UpdateDto, UpdateProgressEvent};
use crate::exit::LOG_FLUSH_TIMEOUT;
use crate::state::AppState;

pub const UPDATE_PROGRESS: &str = "update://progress";

/// Procurar versao nova: resposta pequena, falha rapido sem internet.
const CHECK_TIMEOUT: Duration = Duration::from_secs(15);
/// Baixar o instalador (o timeout do plugin vale para a requisicao inteira).
const DOWNLOAD_TIMEOUT: Duration = Duration::from_secs(15 * 60);
/// Intervalo minimo entre eventos de progresso, em bytes.
const PROGRESS_STEP: u64 = 256 * 1024;

/// Porta usada pelos comandos `update_check` e `update_install`. Bloqueante: chamar fora da
/// thread da janela.
pub trait AppUpdater: Send + Sync {
    /// `Ok(None)` quando a versao instalada ja e a mais nova.
    fn check(&self) -> Result<Option<UpdateDto>, AppError>;
    /// Baixa, confere a assinatura e executa o instalador. Em sucesso o processo sai (`exit(0)`) e
    /// nao retorna.
    fn install(&self) -> Result<(), AppError>;
}

pub struct TauriUpdater<R: Runtime> {
    app: AppHandle<R>,
    busy: AtomicBool,
}

impl<R: Runtime> TauriUpdater<R> {
    pub fn new(app: AppHandle<R>) -> Self {
        Self {
            app,
            busy: AtomicBool::new(false),
        }
    }

    fn updater(&self, timeout: Duration) -> Result<Updater, AppError> {
        self.app
            .updater_builder()
            .timeout(timeout)
            .on_before_exit(|| {
                // O plugin chama `exit(0)` logo depois de abrir o instalador: drena o log antes.
                tracing::info!("instalador da atualizacao aberto; fechando o launcher");
                launcher_win::logging::flush(LOG_FLUSH_TIMEOUT);
            })
            .build()
            .map_err(|error| {
                tracing::error!(%error, "atualizador nao configurado");
                AppError::UpdateFailed
            })
    }

    fn fetch(&self, timeout: Duration) -> Result<Option<Update>, AppError> {
        let updater = self.updater(timeout)?;
        tauri::async_runtime::block_on(updater.check()).map_err(|error| {
            tracing::warn!(%error, "falha ao procurar atualizacao");
            AppError::UpdateFailed
        })
    }

    fn session_idle(&self) -> bool {
        let state = self.app.state::<AppState>();
        state
            .session
            .get()
            .map_or(true, |s| s.get().snapshot.state == SessionState::Idle)
    }

    fn emit_progress(&self, downloaded: u64, total: Option<u64>) {
        let payload = UpdateProgressEvent { downloaded, total };
        if let Err(error) = self.app.emit(UPDATE_PROGRESS, payload) {
            tracing::warn!(%error, "evento de progresso nao emitido");
        }
    }

    fn download_and_run(&self) -> Result<(), AppError> {
        if !self.session_idle() {
            // O instalador fecha o launcher: so com nenhum jogo em sessao.
            return Err(AppError::InvalidState);
        }
        let Some(update) = self.fetch(DOWNLOAD_TIMEOUT)? else {
            return Err(AppError::UpdateFailed);
        };
        tracing::info!(versao = %update.version, "baixando atualizacao");
        let mut downloaded: u64 = 0;
        let mut last: u64 = 0;
        let bytes = tauri::async_runtime::block_on(update.download(
            |chunk, total| {
                downloaded += chunk as u64;
                if downloaded - last >= PROGRESS_STEP || total == Some(downloaded) {
                    last = downloaded;
                    self.emit_progress(downloaded, total);
                }
            },
            || {},
        ))
        .map_err(|error| {
            // Rede, servidor ou assinatura invalida: nada e executado.
            tracing::error!(%error, "download da atualizacao falhou");
            AppError::UpdateFailed
        })?;
        tracing::info!(versao = %update.version, "atualizacao baixada e assinatura conferida");
        update.install(bytes).map_err(|error| {
            // `ShellExecuteW` falhou: em geral o UAC recusado.
            tracing::warn!(%error, "instalador da atualizacao nao abriu");
            AppError::UpdateCancelled
        })
    }
}

impl<R: Runtime> AppUpdater for TauriUpdater<R> {
    fn check(&self) -> Result<Option<UpdateDto>, AppError> {
        Ok(self.fetch(CHECK_TIMEOUT)?.map(|u| UpdateDto {
            version: u.version.clone(),
            current_version: u.current_version.clone(),
            notes: u.body.clone().filter(|b| !b.trim().is_empty()),
        }))
    }

    fn install(&self) -> Result<(), AppError> {
        if self.busy.swap(true, Ordering::AcqRel) {
            return Err(AppError::Busy);
        }
        let result = self.download_and_run();
        self.busy.store(false, Ordering::Release);
        result
    }
}
