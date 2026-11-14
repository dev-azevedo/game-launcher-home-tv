//! Estado compartilhado do app (composition root). Cada ponta (caso de uso, adapter) entra aqui
//! com a sua tarefa BE-xx.

use std::any::Any;
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex, OnceLock};

use launcher_core::application::ports::{CoverCache, SettingsRepository};
use launcher_core::application::{LibraryService, SerialGate, SessionService, VolumeService};
use launcher_core::domain::settings::Settings;
use launcher_win::input_thread::InputControl;

/// Dados do usuario carregados no `setup` (pasta de dados, repositorio e configuracoes).
pub struct UserData {
    pub data_dir: PathBuf,
    pub settings_repo: Arc<dyn SettingsRepository>,
}

/// Portas de serializacao do contrato (secao 0): `Busy` em chamada concorrente.
#[derive(Default)]
pub struct AppState {
    /// `library_refresh`.
    pub library_gate: SerialGate,
    /// `session_launch`, `session_launch_with_uac`, `session_replace` e `session_retry`.
    pub session_gate: SerialGate,
    /// `TokenElevation` do processo no inicio (SEC-20). Verdadeiro nao e esperado.
    pub process_elevated: AtomicBool,
    /// Fecha o app (`app_exit`); definido no `setup`. Nao encerra processo de jogo.
    pub exit_hook: OnceLock<Box<dyn Fn() + Send + Sync>>,
    /// Definido no `setup`, depois de resolver a pasta de dados.
    pub user_data: OnceLock<UserData>,
    /// Configuracoes em memoria; `settings_set_*` gravam por `user_data.settings_repo`.
    pub settings: Mutex<Settings>,
    /// Casos de uso de sessao (BE-20, BE-21); definidos por `runtime::start` no `setup`.
    pub session: OnceLock<Arc<SessionService>>,
    /// Estado da entrada (modo e controles conectados, BE-14).
    pub input: OnceLock<Arc<InputControl>>,
    /// Threads vivas (entrada, eventos do sistema, temporizador); soltar encerra e junta.
    pub threads: OnceLock<Mutex<Vec<Box<dyn Any + Send>>>>,
    /// Biblioteca (BE-29): varredura, camada do usuario, cadastro manual e resolver de lancamento.
    /// Definida no `setup`, depois da pasta de dados.
    pub library: OnceLock<Arc<LibraryService>>,
    /// Capas e icones reencodados (BE-28), servidos por `cover://`.
    pub covers: OnceLock<Arc<dyn CoverCache>>,
    /// Volume do dispositivo padrao (BE-32); ausente se o Core Audio nao subiu.
    pub volume: OnceLock<Arc<VolumeService>>,
    /// A primeira leitura da biblioteca (primeiro paint da UI) dispara a varredura em segundo
    /// plano, uma vez so (BE-40).
    pub initial_refresh: AtomicBool,
}
