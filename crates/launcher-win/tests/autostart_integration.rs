//! BE-36 no Agendador de Tarefas e no registro de verdade (`--features win-integration`), como
//! usuario comum, sem UAC. Tudo usa nomes **proprios de teste e unicos por teste** (tarefa
//! `Sofa QA Autostart <pid>x<n> (<SID>)` e valor `SofaQaTest<pid>x<n>` em `HKCU\...\Run`), nunca a
//! tarefa nem o valor reais do Sofa, e cada teste apaga o que criou (guarda no `Drop`),
//! conferindo no fim que nada sobrou. Como os nomes nao se repetem, os testes rodam em paralelo
//! (QA-BUG-03).
//!
//! A recusa fora de `Program Files` (DR-04) usa `current_exe` falso; ligar com o binario real de
//! teste (em `target\debug`) tem que recusar.
#![cfg(feature = "win-integration")]

use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex};

use launcher_core::application::ports::{
    InstallEnv, RunEntry, RunState, SchedulerError, SettingsStore, TaskScheduler, TaskState,
};
use launcher_core::application::{AppError, AutostartService, AutostartStatus};
use launcher_core::domain::autostart::Mechanism;
use launcher_win::autostart::{WinInstallEnv, WinRunEntry, WinTaskScheduler};

const QA_TASK: &str = "Sofa QA Autostart";
const QA_RUN: &str = "SofaQaTest";
const APPROVED_KEY: &str =
    r"HKCU\Software\Microsoft\Windows\CurrentVersion\Explorer\StartupApproved\Run";

/// Pasta que finge ser `Program Files` e o exe que finge estar nela (nada disso e executado).
fn fake_install() -> (String, String) {
    let root = std::env::temp_dir().join("sofa-qa-fake-programfiles");
    let exe = root.join("Sofa").join("sofa.exe");
    (root.display().to_string(), exe.display().to_string())
}

struct FakeEnv {
    exe: String,
    pf: String,
    default: Option<u32>,
}
impl InstallEnv for FakeEnv {
    fn current_exe(&self) -> Option<String> {
        Some(self.exe.clone())
    }
    fn program_files(&self) -> Vec<String> {
        vec![self.pf.clone()]
    }
    fn install_default(&self) -> Option<u32> {
        self.default
    }
}

#[derive(Default)]
struct MemStore(Mutex<bool>);
impl SettingsStore for MemStore {
    fn monitor_id(&self) -> Option<String> {
        None
    }
    fn set_monitor_id(&self, _: Option<String>) -> Result<(), AppError> {
        Ok(())
    }
    fn install_defaults_applied(&self) -> bool {
        *self.0.lock().unwrap()
    }
    fn mark_install_defaults_applied(&self) -> Result<(), AppError> {
        *self.0.lock().unwrap() = true;
        Ok(())
    }
}

/// Agendador real, mas com a criacao forcada a falhar (E_ACCESSDENIED), como conta sem direito.
struct RegisterDenied(WinTaskScheduler);
impl TaskScheduler for RegisterDenied {
    fn state(&self, e: Option<&str>) -> Result<TaskState, SchedulerError> {
        self.0.state(e)
    }
    fn register(&self, _exe: &str) -> Result<(), SchedulerError> {
        Err(SchedulerError(0x8007_0005_u32 as i32))
    }
    fn delete(&self) -> Result<(), SchedulerError> {
        self.0.delete()
    }
}

/// Limpa antes e depois e confere que nada ficou.
struct Cleanup {
    sched: WinTaskScheduler,
    run: WinRunEntry,
    /// Nome do valor em `Run` e em `StartupApproved`, unico deste teste.
    run_name: String,
}
impl Cleanup {
    fn new() -> Self {
        static N: AtomicU32 = AtomicU32::new(0);
        let tag = format!(
            "{}x{}",
            std::process::id(),
            N.fetch_add(1, Ordering::SeqCst)
        );
        let run_name = format!("{QA_RUN}{tag}");
        let c = Self {
            sched: WinTaskScheduler::with_test_prefix(&format!("{QA_TASK} {tag}"))
                .expect("SID do usuario"),
            run: WinRunEntry::with_value_name(&run_name),
            run_name,
        };
        c.wipe();
        c
    }
    fn wipe(&self) {
        let _ = self.sched.delete();
        let _ = self.run.remove();
        let _ = Command::new("reg")
            .args(["delete", APPROVED_KEY, "/v", self.run_name.as_str(), "/f"])
            .output();
    }
}
impl Drop for Cleanup {
    fn drop(&mut self) {
        self.wipe();
        if !std::thread::panicking() {
            assert_eq!(
                self.sched.state(None),
                Ok(TaskState::Absent),
                "tarefa de teste sobrou"
            );
            assert_eq!(
                self.run.state(None),
                Ok(RunState::Absent),
                "Run de teste sobrou"
            );
        }
    }
}

fn service(
    c: &Cleanup,
    exe: &str,
    pf: &str,
    default: Option<u32>,
    sched: Arc<dyn TaskScheduler>,
) -> (AutostartService, Arc<MemStore>) {
    let store = Arc::new(MemStore::default());
    let svc = AutostartService::new(
        sched,
        Arc::new(c.run.clone()),
        Arc::new(FakeEnv {
            exe: exe.into(),
            pf: pf.into(),
            default,
        }),
        store.clone(),
    );
    (svc, store)
}

fn real_exe_path() -> String {
    // Qualquer arquivo real serve: a tarefa nunca roda neste teste.
    let p: PathBuf = std::env::current_exe().unwrap();
    p.display().to_string()
}

/// Valor de `Run` pelo `reg query` (independente do adapter).
fn reg_run_value(c: &Cleanup) -> Option<String> {
    let out = Command::new("reg")
        .args([
            "query",
            r"HKCU\Software\Microsoft\Windows\CurrentVersion\Run",
            "/v",
            c.run_name.as_str(),
        ])
        .output()
        .unwrap();
    out.status
        .success()
        .then(|| String::from_utf8_lossy(&out.stdout).into_owned())
}

#[test]
fn tarefa_criada_campo_a_campo_tem_o_xml_do_sec22() {
    let c = Cleanup::new();
    let exe = real_exe_path();
    c.sched
        .register(&exe)
        .expect("criar a tarefa como usuario comum, sem UAC");

    let st = c.sched.state(Some(&exe)).unwrap();
    // `SCHED_S_TASK_HAS_NOT_RUN` (0x41303): ainda nao rodou, nao e falha.
    assert_eq!(
        st,
        TaskState::Mine {
            last_result: 0x0004_1303
        },
        "{st:?}"
    );

    let xml = c.sched.exported_xml().unwrap();
    eprintln!("{xml}");
    assert!(
        xml.contains("<LogonType>InteractiveToken</LogonType>"),
        "{xml}"
    );
    assert!(
        !xml.contains("HighestAvailable"),
        "SEC-22: nunca RunLevel elevado"
    );
    assert!(
        xml.contains(&format!("<UserId>{}</UserId>", c.sched.sid())),
        "UserId = SID"
    );
    assert!(xml.contains("<LogonTrigger>"), "{xml}");
    assert_eq!(
        xml.matches("<UserId>").count(),
        2,
        "principal e gatilho, so do mesmo SID"
    );
    assert!(xml.contains("<ExecutionTimeLimit>PT0S</ExecutionTimeLimit>"));
    assert!(xml.contains("<MultipleInstancesPolicy>IgnoreNew</MultipleInstancesPolicy>"));
    assert!(xml.contains("<Arguments>--autostart</Arguments>"));
    assert!(
        xml.contains(&format!("<Command>\"{exe}\"</Command>")),
        "caminho entre aspas: {xml}"
    );
    assert!(
        !xml.contains("<WorkingDirectory>"),
        "sem diretorio de trabalho gravavel"
    );
    assert_eq!(xml.matches("<Exec>").count(), 1);
    assert!(!xml.contains("<Password>") && !xml.contains("S4U"));

    // Caminho diferente do esperado = estranha; sem esperado, o exe de teste nao se chama sofa.exe.
    assert_eq!(
        c.sched
            .state(Some(r"C:\Program Files\Sofa\sofa.exe"))
            .unwrap(),
        TaskState::Foreign
    );
    assert_eq!(c.sched.state(None).unwrap(), TaskState::Foreign);

    c.sched.delete().unwrap();
    assert_eq!(c.sched.state(Some(&exe)).unwrap(), TaskState::Absent);
    c.sched.delete().unwrap(); // apagar o que nao existe nao e erro
}

/// Tarefa forjada com o mesmo nome apontando para `calc.exe`: nao vale, e e recriada.
#[test]
fn tarefa_forjada_para_calc_nao_e_valida_e_o_app_a_recria() {
    let c = Cleanup::new();
    let (pf, exe) = fake_install();
    let (svc, _store) = service(&c, &exe, &pf, None, Arc::new(c.sched.clone()));

    c.sched
        .register(r"C:\Windows\System32\calc.exe")
        .expect("forjar a tarefa de teste");
    assert_eq!(c.sched.state(Some(&exe)).unwrap(), TaskState::Foreign);
    assert_eq!(
        svc.status(),
        AutostartStatus::OFF,
        "forjada nao conta como ligada"
    );

    // O app a recria com o conteudo certo.
    let on = svc.enable().expect("ligar");
    assert_eq!(on.mechanism, Mechanism::Task);
    let xml = c.sched.exported_xml().unwrap();
    assert!(
        xml.contains(&format!("<Command>\"{exe}\"</Command>")),
        "{xml}"
    );
    assert!(!xml.contains("calc.exe"));
    assert_eq!(svc.status(), on);
}

#[test]
fn ligar_cria_um_unico_mecanismo_e_desligar_remove_os_dois() {
    let c = Cleanup::new();
    let (pf, exe) = fake_install();
    let (svc, _store) = service(&c, &exe, &pf, None, Arc::new(c.sched.clone()));

    // Um Run antigo (de uma versao anterior) e removido ao ligar a tarefa.
    c.run.set(&exe).unwrap();
    assert!(reg_run_value(&c).is_some());
    let on = svc.enable().unwrap();
    assert_eq!(
        on,
        AutostartStatus {
            enabled: true,
            mechanism: Mechanism::Task
        }
    );
    assert!(reg_run_value(&c).is_none(), "um mecanismo por vez");
    assert!(matches!(
        c.sched.state(Some(&exe)).unwrap(),
        TaskState::Mine { .. }
    ));

    assert_eq!(svc.disable().unwrap(), AutostartStatus::OFF);
    assert_eq!(c.sched.state(Some(&exe)).unwrap(), TaskState::Absent);
    assert!(reg_run_value(&c).is_none());
    assert_eq!(svc.status(), AutostartStatus::OFF);
}

/// A criacao da tarefa falha (E_ACCESSDENIED forcado): o Run nasce sozinho, com o caminho entre
/// aspas e `--autostart`, e a tarefa nao existe.
#[test]
fn falha_da_tarefa_cria_o_run_sozinho() {
    let c = Cleanup::new();
    let (pf, exe) = fake_install();
    let denied = Arc::new(RegisterDenied(c.sched.clone()));
    let (svc, _store) = service(&c, &exe, &pf, None, denied);

    let on = svc.enable().expect("fallback");
    assert_eq!(
        on,
        AutostartStatus {
            enabled: true,
            mechanism: Mechanism::Run
        }
    );
    let value = reg_run_value(&c).expect("valor Run criado");
    eprintln!("{value}");
    assert!(
        value.contains(&format!("\"{exe}\" --autostart")),
        "caminho entre aspas + --autostart: {value}"
    );
    assert_eq!(c.sched.state(Some(&exe)).unwrap(), TaskState::Absent);
    assert_eq!(svc.status(), on);

    // O usuario desliga o item no Gerenciador de Tarefas (StartupApproved): vira "desligado".
    let ok = Command::new("reg")
        .args([
            "add",
            APPROVED_KEY,
            "/v",
            c.run_name.as_str(),
            "/t",
            "REG_BINARY",
            "/d",
            "030000000000000000000000",
            "/f",
        ])
        .output()
        .unwrap()
        .status
        .success();
    assert!(ok);
    assert_eq!(c.run.state(Some(&exe)).unwrap(), RunState::DisabledByUser);
    assert_eq!(
        svc.status(),
        AutostartStatus {
            enabled: false,
            mechanism: Mechanism::Run
        }
    );

    // Religar na opcao do app tira o "desligado".
    let again = svc.enable().unwrap();
    assert_eq!(again.mechanism, Mechanism::Run);
    assert_eq!(c.run.state(Some(&exe)).unwrap(), RunState::Enabled);

    assert_eq!(svc.disable().unwrap(), AutostartStatus::OFF);
    assert!(reg_run_value(&c).is_none());
}

/// Run com valor de outro programa no mesmo nome: nao e do Sofa.
#[test]
fn run_com_outro_conteudo_e_estranho() {
    let c = Cleanup::new();
    let (_pf, exe) = fake_install();
    let status = Command::new("reg")
        .args([
            "add",
            r"HKCU\Software\Microsoft\Windows\CurrentVersion\Run",
            "/v",
            c.run_name.as_str(),
            "/t",
            "REG_SZ",
            "/d",
            r#""C:\Windows\System32\calc.exe" --autostart"#,
            "/f",
        ])
        .output()
        .unwrap()
        .status;
    assert!(status.success());
    assert_eq!(c.run.state(Some(&exe)).unwrap(), RunState::Foreign);
}

/// DR-04: o binario real de teste roda de `target\debug`, nao de `Program Files`: recusa ligar e
/// nao cria nada.
#[test]
fn binario_fora_de_program_files_recusa_ligar() {
    let c = Cleanup::new();
    let svc = AutostartService::new(
        Arc::new(c.sched.clone()),
        Arc::new(c.run.clone()),
        Arc::new(WinInstallEnv),
        Arc::new(MemStore::default()),
    );
    assert_eq!(svc.enable(), Err(AppError::AutostartFailed));
    assert_eq!(c.sched.state(None).unwrap(), TaskState::Absent);
    assert!(reg_run_value(&c).is_none());
}

/// SEC-21 / BE-36: `InstallDefaults` DWORD 1 liga uma vez; 0 e 7 deixam desligado.
#[test]
fn padrao_do_instalador_com_adaptadores_reais() {
    for (valor, liga) in [
        (Some(1), true),
        (Some(0), false),
        (Some(7), false),
        (None, false),
    ] {
        let c = Cleanup::new();
        let (pf, exe) = fake_install();
        let (svc, store) = service(&c, &exe, &pf, valor, Arc::new(c.sched.clone()));
        svc.apply_install_default_once();
        assert!(*store.0.lock().unwrap(), "{valor:?}");
        assert_eq!(svc.status().enabled, liga, "{valor:?}");
        svc.disable().unwrap();
    }
}
