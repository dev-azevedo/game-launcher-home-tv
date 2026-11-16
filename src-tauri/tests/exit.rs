//! BE-71 (CA-82 itens 2, 3 e 5; RN-35; D-51; SEC-18; SEC-28; INV-02, INV-03): "Fechar o launcher"
//! reusa o `app_exit` (#30). O encerramento registra uma linha `info` com o estado da sessao e o
//! `GameId`, sem caminho, drena o log e so entao sai. Sessao, broker e autostart nao entram.

use std::cell::RefCell;
use std::fs;
use std::path::{Path, PathBuf};

use launcher_core::domain::library::GameId;
use launcher_core::domain::session::{SessionSnapshot, SessionState};
use launcher_core::domain::time::Millis;
use launcher_win::logging;
use sofa_lib::exit::{close_launcher, ExitEnv};

fn running(game: &str) -> SessionSnapshot {
    SessionSnapshot {
        state: SessionState::Running,
        game_id: Some(GameId::parse(game).unwrap()),
        since: Some(Millis(1)),
        dismissed: false,
        elevated: false,
        minimized: false,
        launch_failure: None,
    }
}

/// Env de teste: so anota a ordem das tres unicas capacidades do encerramento.
struct Fake {
    snapshot: Option<SessionSnapshot>,
    calls: RefCell<Vec<&'static str>>,
}

impl ExitEnv for Fake {
    fn session(&self) -> Option<SessionSnapshot> {
        self.calls.borrow_mut().push("session");
        self.snapshot.clone()
    }
    fn flush_log(&self) {
        self.calls.borrow_mut().push("flush");
    }
    fn exit(&self) {
        self.calls.borrow_mut().push("exit");
    }
}

#[test]
fn encerra_lendo_a_sessao_uma_vez_drenando_o_log_e_saindo_nessa_ordem() {
    let env = Fake {
        snapshot: Some(running("steam:111")),
        calls: RefCell::default(),
    };
    close_launcher(&env);
    assert_eq!(*env.calls.borrow(), ["session", "flush", "exit"]);
}

#[test]
fn sem_runtime_de_sessao_ainda_drena_o_log_e_sai() {
    let env = Fake {
        snapshot: None,
        calls: RefCell::default(),
    };
    close_launcher(&env);
    assert_eq!(*env.calls.borrow(), ["session", "flush", "exit"]);
}

struct TempDir(PathBuf);
impl TempDir {
    fn new() -> Self {
        let p = std::env::temp_dir().join(format!("sofa-exit-{}", std::process::id()));
        fs::create_dir_all(&p).unwrap();
        Self(p)
    }
}
impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn log_text(dir: &Path) -> String {
    fs::read_to_string(dir.join("sofa.log")).unwrap_or_default()
}

/// Env de producao do log: o `flush` e o de verdade; o `exit` le o arquivo NO MOMENTO da saida.
struct RealLog<'a> {
    dir: &'a Path,
    snapshot: SessionSnapshot,
    at_exit: RefCell<Option<String>>,
}

impl ExitEnv for RealLog<'_> {
    fn session(&self) -> Option<SessionSnapshot> {
        Some(self.snapshot.clone())
    }
    fn flush_log(&self) {
        assert!(logging::flush(sofa_lib::exit::LOG_FLUSH_TIMEOUT), "flush");
    }
    fn exit(&self) {
        *self.at_exit.borrow_mut() = Some(log_text(self.dir));
    }
}

/// Com sessao `Running`, a linha sai com o estado e o `GameId`, sem caminho, e ja esta no arquivo
/// quando o `exit(0)` e chamado. Unico teste deste binario que instala o log global.
#[test]
fn linha_com_estado_e_game_id_chega_ao_arquivo_antes_do_exit() {
    let tmp = TempDir::new();
    logging::init(&tmp.0, logging::Level::Info).expect("log global");
    let env = RealLog {
        dir: &tmp.0,
        snapshot: running("steam:620"),
        at_exit: RefCell::new(None),
    };
    close_launcher(&env);
    let seen = env.at_exit.borrow().clone().expect("exit foi chamado");
    let line = seen
        .lines()
        .find(|l| l.contains("fechar o launcher") && l.contains("steam:620"))
        .unwrap_or_else(|| panic!("linha ausente do arquivo antes do exit:\n{seen}"));
    assert!(line.contains("INFO"), "{line}");
    assert!(line.contains("Running"), "estado: {line}");
    assert!(line.contains("steam:620"), "GameId: {line}");
    assert!(
        !line.contains('\\') && !line.contains(":/") && !line.to_lowercase().contains(".exe"),
        "SEC-18: sem caminho: {line}"
    );
}

/// INV-03, CA-82 item 3 e 5: o caminho de saida nao chama fechamento, forca, primeiro plano nem
/// autostart. O `ExitEnv` so tem as tres capacidades e o hook so chama `close_launcher`.
#[test]
fn caminho_de_saida_nao_toca_sessao_primeiro_plano_nem_autostart() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let strip = |rel: &str| -> String {
        fs::read_to_string(root.join(rel))
            .unwrap()
            .lines()
            .filter(|l| !l.trim_start().starts_with("//"))
            .collect::<Vec<_>>()
            .join("\n")
    };
    let exit_rs = strip("src/exit.rs");
    for banned in [
        "force_close",
        ".close(",
        "session_close",
        "return_to_game",
        "ForegroundBroker",
        "foreground",
        "AutostartRegistry",
        "AutostartService",
        "autostart",
        "KILL_ON_JOB_CLOSE",
        "TerminateProcess",
        "TerminateJobObject",
    ] {
        assert!(!exit_rs.contains(banned), "exit.rs nao pode citar {banned}");
    }
    let lib = strip("src/lib.rs");
    let hook = lib
        .split(".exit_hook")
        .nth(1)
        .and_then(|s| s.split("}));").next())
        .expect("hook de saida em lib.rs");
    assert!(hook.contains("exit::close_launcher"), "{hook}");
    for banned in [
        "force_close",
        "session",
        "foreground",
        "autostart",
        "handle.exit",
    ] {
        assert!(
            !hook.replace("exit::TauriExit", "").contains(banned),
            "o hook de saida so chama close_launcher, nao {banned}"
        );
    }
}
