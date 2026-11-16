//! "Fechar o launcher" (RN-35, CA-82, D-51, D-57): o encerramento que o `app_exit` (#30) dispara.
//!
//! Fecha so o launcher. A sessao de jogo, se houver, continua como esta: o `KILL_ON_JOB_CLOSE` nao
//! esta ligado (INV-05), entao o kernel solta o Job e o jogo segue rodando. Por isso o caminho de
//! saida so tem tres capacidades (`ExitEnv`): ler o estado da sessao para o log, drenar o log e
//! sair. Nao ha como pedir fechamento ao jogo, forcar, trazer janela a frente (`ForegroundBroker`,
//! INV-03) nem mexer no "Iniciar com o Windows" (`AutostartRegistry`) por aqui.

use std::time::Duration;

use launcher_core::domain::session::SessionSnapshot;

/// Folga curta para o log assincrono gravar o que ficou na fila antes de sair.
pub const LOG_FLUSH_TIMEOUT: Duration = Duration::from_millis(300);

/// O que o encerramento pode fazer. Deliberadamente sem `force_close`, `close`, primeiro plano nem autostart.
pub trait ExitEnv {
    /// Estado atual da sessao (so leitura); `None` se o runtime de sessao nao subiu.
    fn session(&self) -> Option<SessionSnapshot>;
    /// Espera o log assincrono gravar a fila (ate `LOG_FLUSH_TIMEOUT`).
    fn flush_log(&self);
    /// Encerra o processo (`exit(0)`).
    fn exit(&self);
}

/// Registra a linha `info` "fechar o launcher" com o estado da sessao e o `GameId` (SEC-18: sem
/// caminho), drena o log e sai, nessa ordem.
pub fn close_launcher(env: &dyn ExitEnv) {
    match env.session() {
        Some(s) => {
            let jogo = s.game_id.as_ref().map(ToString::to_string);
            tracing::info!(
                estado = ?s.state,
                jogo = jogo.as_deref().unwrap_or("-"),
                "fechar o launcher"
            );
        }
        None => tracing::info!(estado = "sem-sessao", jogo = "-", "fechar o launcher"),
    }
    env.flush_log();
    env.exit();
}

/// O `ExitEnv` de producao: sessao do `AppState`, `logging::flush` e `AppHandle::exit`.
pub struct TauriExit<R: tauri::Runtime> {
    pub handle: tauri::AppHandle<R>,
}

impl<R: tauri::Runtime> ExitEnv for TauriExit<R> {
    fn session(&self) -> Option<SessionSnapshot> {
        use tauri::Manager;
        let state = self.handle.state::<crate::state::AppState>();
        state.session.get().map(|s| s.get().snapshot)
    }

    fn flush_log(&self) {
        launcher_win::logging::flush(LOG_FLUSH_TIMEOUT);
    }

    fn exit(&self) {
        self.handle.exit(0);
    }
}
