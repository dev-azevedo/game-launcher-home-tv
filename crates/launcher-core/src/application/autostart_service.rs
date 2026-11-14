//! "Iniciar com o Windows" (BE-36, RN-17, CA-04, CA-51, CA-74, SEC-22, DR-04, ADR-0006): um unico
//! mecanismo ativo por vez, sempre do proprio usuario e sem elevacao.
//!
//! - **Tarefa** de logon (`ITaskService`, `LeastPrivilege`) e o mecanismo preferido;
//! - **Fallback automatico** para `HKCU\...\Run` quando criar a tarefa falha (ex.:
//!   `E_ACCESSDENIED`) ou quando a ultima execucao dela registra falha real. Destino unico: o Run.
//!   Falha nunca tenta outra forma de tarefa, `schtasks.exe` nem elevacao. A tarefa e removida
//!   antes de gravar o Run. Sem volta automatica Run -> tarefa: so quando o usuario religa.
//! - Ligar so e permitido se o exe em execucao estiver em `Program Files` (DR-04).
//! - Tarefa com o nome do Sofa e conteudo diferente nunca e tratada como valida.

use std::sync::{Arc, Mutex, PoisonError};

use super::error::AppError;
use super::ports::{
    InstallEnv, RunEntry, RunState, SchedulerError, SettingsStore, TaskScheduler, TaskState,
};
use crate::domain::autostart::{
    install_default_enabled, is_task_failure, is_under_program_files, Mechanism,
};

/// O que `settings_get` mostra: ligado ou nao, e por qual mecanismo.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AutostartStatus {
    pub enabled: bool,
    pub mechanism: Mechanism,
}

impl AutostartStatus {
    pub const OFF: AutostartStatus = AutostartStatus {
        enabled: false,
        mechanism: Mechanism::None,
    };
}

pub struct AutostartService {
    tasks: Arc<dyn TaskScheduler>,
    run: Arc<dyn RunEntry>,
    env: Arc<dyn InstallEnv>,
    store: Arc<dyn SettingsStore>,
    /// Serializa ligar, desligar e reconciliar (um mecanismo por vez).
    op: Mutex<()>,
}

impl AutostartService {
    pub fn new(
        tasks: Arc<dyn TaskScheduler>,
        run: Arc<dyn RunEntry>,
        env: Arc<dyn InstallEnv>,
        store: Arc<dyn SettingsStore>,
    ) -> Self {
        Self {
            tasks,
            run,
            env,
            store,
            op: Mutex::new(()),
        }
    }

    /// O exe em execucao, se (e so se) estiver em `Program Files` (DR-04).
    fn installed_exe(&self) -> Option<String> {
        let exe = self.env.current_exe()?;
        is_under_program_files(&exe, &self.env.program_files()).then_some(exe)
    }

    /// Estado real, lido do Windows. Nunca falha: erro de leitura vira "desligado" e vai ao log.
    /// Fora de `Program Files` o `Path` so precisa terminar em `sofa.exe` (a copia de
    /// desenvolvimento mostra o que o instalado fez, sem mexer em nada).
    pub fn status(&self) -> AutostartStatus {
        let _g = self.op.lock().unwrap_or_else(PoisonError::into_inner);
        self.read_status(self.installed_exe().as_deref())
    }

    fn read_status(&self, expected: Option<&str>) -> AutostartStatus {
        match self.tasks.state(expected) {
            Ok(TaskState::Mine { .. }) => {
                return AutostartStatus {
                    enabled: true,
                    mechanism: Mechanism::Task,
                }
            }
            Ok(TaskState::Foreign) => {
                tracing::warn!(
                    "autostart: ha uma tarefa com o nome do Sofa e conteudo diferente; ignorada"
                );
            }
            Ok(TaskState::Absent) => {}
            Err(SchedulerError(code)) => {
                tracing::warn!(
                    hresult = format_args!("{code:#x}"),
                    "autostart: nao leu a tarefa"
                );
            }
        }
        match self.run.state(expected) {
            Ok(RunState::Enabled) => AutostartStatus {
                enabled: true,
                mechanism: Mechanism::Run,
            },
            // Desligada no Gerenciador de Tarefas: o mecanismo existe, a opcao esta desligada.
            Ok(RunState::DisabledByUser) => AutostartStatus {
                enabled: false,
                mechanism: Mechanism::Run,
            },
            Ok(RunState::Absent | RunState::Foreign) => AutostartStatus::OFF,
            Err(SchedulerError(code)) => {
                tracing::warn!(
                    hresult = format_args!("{code:#x}"),
                    "autostart: nao leu o Run"
                );
                AutostartStatus::OFF
            }
        }
    }

    /// `settings_set_autostart(true)`: tarefa primeiro; se falhar, Run. Recusa fora de
    /// `Program Files` (DR-04): `AutostartFailed`.
    pub fn enable(&self) -> Result<AutostartStatus, AppError> {
        let _g = self.op.lock().unwrap_or_else(PoisonError::into_inner);
        let Some(exe) = self.installed_exe() else {
            tracing::warn!("autostart: recusado, o Sofa nao esta rodando de Program Files (DR-04)");
            return Err(AppError::AutostartFailed);
        };

        match self.try_task(&exe) {
            Ok(()) => {
                // Um mecanismo por vez: tira o Run que uma versao anterior ou um fallback deixou.
                if let Err(SchedulerError(code)) = self.run.remove() {
                    tracing::warn!(
                        hresult = format_args!("{code:#x}"),
                        "autostart: nao removeu o Run antigo"
                    );
                    return Err(AppError::AutostartFailed);
                }
                tracing::info!(mecanismo = "tarefa", "autostart ligado");
                Ok(AutostartStatus {
                    enabled: true,
                    mechanism: Mechanism::Task,
                })
            }
            Err(reason) => self.fall_back_to_run(&exe, &reason),
        }
    }

    /// Cria a tarefa e confere o que ficou gravado (conteudo nosso, SEC-22 (e)).
    fn try_task(&self, exe: &str) -> Result<(), String> {
        if let Ok(TaskState::Foreign) = self.tasks.state(Some(exe)) {
            tracing::warn!(
                "autostart: tarefa estranha com o nome do Sofa removida antes de criar a nossa"
            );
            let _ = self.tasks.delete();
        }
        self.tasks
            .register(exe)
            .map_err(|SchedulerError(code)| format!("registro da tarefa falhou ({code:#x})"))?;
        match self.tasks.state(Some(exe)) {
            Ok(TaskState::Mine { .. }) => Ok(()),
            Ok(other) => Err(format!("tarefa criada nao confere ({other:?})")),
            Err(SchedulerError(code)) => Err(format!("nao releu a tarefa criada ({code:#x})")),
        }
    }

    /// Unico destino do fallback (SEC-22 rev. 4 (a), (b)): remove a tarefa e grava o Run.
    fn fall_back_to_run(&self, exe: &str, reason: &str) -> Result<AutostartStatus, AppError> {
        tracing::warn!(
            motivo = reason,
            "autostart: tarefa indisponivel, usando HKCU Run"
        );
        if let Err(SchedulerError(code)) = self.tasks.delete() {
            tracing::warn!(
                hresult = format_args!("{code:#x}"),
                "autostart: nao removeu a tarefa antes do Run"
            );
        }
        match self.run.set(exe) {
            Ok(()) => {
                tracing::info!(mecanismo = "run", "autostart ligado");
                Ok(AutostartStatus {
                    enabled: true,
                    mechanism: Mechanism::Run,
                })
            }
            Err(SchedulerError(code)) => {
                tracing::error!(
                    hresult = format_args!("{code:#x}"),
                    "autostart: o Run tambem falhou"
                );
                Err(AppError::AutostartFailed)
            }
        }
    }

    /// `settings_set_autostart(false)`: remove os dois mecanismos (CA-04). Pode ser chamado de
    /// qualquer pasta (nao depende de `Program Files`).
    pub fn disable(&self) -> Result<AutostartStatus, AppError> {
        let _g = self.op.lock().unwrap_or_else(PoisonError::into_inner);
        let task = self.tasks.delete();
        let run = self.run.remove();
        for (what, r) in [("tarefa", task), ("Run", run)] {
            if let Err(SchedulerError(code)) = r {
                tracing::error!(
                    mecanismo = what,
                    hresult = format_args!("{code:#x}"),
                    "autostart: nao desligou"
                );
                return Err(AppError::AutostartFailed);
            }
        }
        tracing::info!("autostart desligado (tarefa e Run removidos)");
        Ok(AutostartStatus::OFF)
    }

    /// Na abertura (so instalado, DR-04): tarefa estranha e removida; tarefa nossa com falha real
    /// na ultima execucao troca para o Run (sem volta automatica); tarefa valida com Run sobrando
    /// fica so com a tarefa. Falhas aqui so vao para o log.
    pub fn reconcile_on_open(&self) {
        let _g = self.op.lock().unwrap_or_else(PoisonError::into_inner);
        let Some(exe) = self.installed_exe() else {
            return;
        };
        match self.tasks.state(Some(&exe)) {
            Ok(TaskState::Foreign) => {
                tracing::warn!("autostart: tarefa estranha com o nome do Sofa removida");
                let _ = self.tasks.delete();
            }
            Ok(TaskState::Mine { last_result }) if is_task_failure(last_result) => {
                let reason = format!("a ultima execucao da tarefa falhou ({last_result:#x})");
                if let Err(e) = self.fall_back_to_run(&exe, &reason) {
                    tracing::error!(%e, "autostart: o fallback para o Run falhou");
                }
            }
            Ok(TaskState::Mine { .. }) => {
                if matches!(self.run.state(Some(&exe)), Ok(RunState::Enabled)) {
                    tracing::info!("autostart: Run sobrando ao lado da tarefa; removido");
                    let _ = self.run.remove();
                }
            }
            Ok(TaskState::Absent) | Err(_) => {}
        }
    }

    /// Primeira abertura do perfil com o Sofa instalado: aplica o padrao do instalador
    /// (`InstallDefaults`, DWORD 1 liga; qualquer outra coisa deixa desligado) e marca como
    /// aplicado. Se ligar falhar, nao marca: tenta de novo na proxima abertura. Fora de
    /// `Program Files` (copia de desenvolvimento) nao faz nada nem marca.
    pub fn apply_install_default_once(&self) {
        if self.store.install_defaults_applied() {
            return;
        }
        if self.installed_exe().is_none() {
            return;
        }
        if install_default_enabled(self.env.install_default()) {
            if let Err(e) = self.enable() {
                tracing::warn!(%e, "autostart: padrao do instalador nao aplicado; tenta na proxima abertura");
                return;
            }
            tracing::info!("autostart: padrao do instalador aplicado (ligado)");
        } else {
            tracing::info!("autostart: padrao do instalador e desligado");
        }
        if let Err(e) = self.store.mark_install_defaults_applied() {
            tracing::warn!(%e, "autostart: nao marcou o padrao como aplicado");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const EXE: &str = r"C:\Program Files\Sofa\sofa.exe";
    const DEV_EXE: &str = r"D:\GIT\STD\gamelaucher\target\debug\sofa.exe";

    #[derive(Default)]
    struct Sched {
        state: Mutex<Option<TaskState>>,
        registered: Mutex<Vec<String>>,
        deletes: Mutex<u32>,
        register_error: Mutex<Option<i32>>,
        delete_error: Mutex<Option<i32>>,
        /// Depois de criar, o que o Windows devolve ao reler (para simular conteudo adulterado).
        after_register: Mutex<Option<TaskState>>,
    }

    impl TaskScheduler for Sched {
        fn state(&self, _expected: Option<&str>) -> Result<TaskState, SchedulerError> {
            Ok(self.state.lock().unwrap().unwrap_or(TaskState::Absent))
        }
        fn register(&self, exe: &str) -> Result<(), SchedulerError> {
            if let Some(code) = *self.register_error.lock().unwrap() {
                return Err(SchedulerError(code));
            }
            self.registered.lock().unwrap().push(exe.to_owned());
            let next = self
                .after_register
                .lock()
                .unwrap()
                .unwrap_or(TaskState::Mine {
                    last_result: 0x41303,
                });
            *self.state.lock().unwrap() = Some(next);
            Ok(())
        }
        fn delete(&self) -> Result<(), SchedulerError> {
            *self.deletes.lock().unwrap() += 1;
            if let Some(code) = *self.delete_error.lock().unwrap() {
                return Err(SchedulerError(code));
            }
            *self.state.lock().unwrap() = Some(TaskState::Absent);
            Ok(())
        }
    }

    #[derive(Default)]
    struct Run {
        state: Mutex<Option<RunState>>,
        set_error: Mutex<Option<i32>>,
        removes: Mutex<u32>,
    }

    impl RunEntry for Run {
        fn state(&self, _expected: Option<&str>) -> Result<RunState, SchedulerError> {
            Ok(self.state.lock().unwrap().unwrap_or(RunState::Absent))
        }
        fn set(&self, _exe: &str) -> Result<(), SchedulerError> {
            if let Some(code) = *self.set_error.lock().unwrap() {
                return Err(SchedulerError(code));
            }
            *self.state.lock().unwrap() = Some(RunState::Enabled);
            Ok(())
        }
        fn remove(&self) -> Result<(), SchedulerError> {
            *self.removes.lock().unwrap() += 1;
            *self.state.lock().unwrap() = Some(RunState::Absent);
            Ok(())
        }
    }

    struct Env {
        exe: String,
        default: Option<u32>,
    }
    impl InstallEnv for Env {
        fn current_exe(&self) -> Option<String> {
            Some(self.exe.clone())
        }
        fn program_files(&self) -> Vec<String> {
            vec![r"C:\Program Files".into()]
        }
        fn install_default(&self) -> Option<u32> {
            self.default
        }
    }

    #[derive(Default)]
    struct Store {
        applied: Mutex<bool>,
    }
    impl SettingsStore for Store {
        fn monitor_id(&self) -> Option<String> {
            None
        }
        fn set_monitor_id(&self, _id: Option<String>) -> Result<(), AppError> {
            Ok(())
        }
        fn install_defaults_applied(&self) -> bool {
            *self.applied.lock().unwrap()
        }
        fn mark_install_defaults_applied(&self) -> Result<(), AppError> {
            *self.applied.lock().unwrap() = true;
            Ok(())
        }
    }

    struct Rig {
        sched: Arc<Sched>,
        run: Arc<Run>,
        store: Arc<Store>,
        svc: AutostartService,
    }

    fn rig(exe: &str, default: Option<u32>) -> Rig {
        let sched = Arc::new(Sched::default());
        let run = Arc::new(Run::default());
        let store = Arc::new(Store::default());
        let svc = AutostartService::new(
            sched.clone(),
            run.clone(),
            Arc::new(Env {
                exe: exe.into(),
                default,
            }),
            store.clone(),
        );
        Rig {
            sched,
            run,
            store,
            svc,
        }
    }

    const TASK: AutostartStatus = AutostartStatus {
        enabled: true,
        mechanism: Mechanism::Task,
    };
    const RUN: AutostartStatus = AutostartStatus {
        enabled: true,
        mechanism: Mechanism::Run,
    };

    #[test]
    fn ligar_cria_so_a_tarefa_e_confere_o_conteudo() {
        let r = rig(EXE, None);
        assert_eq!(r.svc.enable(), Ok(TASK));
        assert_eq!(*r.sched.registered.lock().unwrap(), vec![EXE]);
        assert_eq!(*r.run.state.lock().unwrap(), Some(RunState::Absent));
        assert_eq!(r.svc.status(), TASK);
    }

    /// DR-04: copia fora de `Program Files` (build de desenvolvimento, pasta gravavel) recusa.
    #[test]
    fn exe_fora_de_program_files_recusa_ligar() {
        for exe in [
            DEV_EXE,
            r"C:\Program Files Evil\sofa.exe",
            r"C:\Users\x\Downloads\sofa.exe",
            r"C:\Program Files\..\Users\x\sofa.exe",
        ] {
            let r = rig(exe, None);
            assert_eq!(r.svc.enable(), Err(AppError::AutostartFailed), "{exe}");
            assert!(r.sched.registered.lock().unwrap().is_empty(), "{exe}");
            assert_eq!(*r.run.state.lock().unwrap(), None, "{exe}");
        }
    }

    /// SEC-22 rev. 4: a criacao da tarefa falha (ex.: `E_ACCESSDENIED`) e o Run nasce sozinho,
    /// com a tarefa removida antes.
    #[test]
    fn falha_ao_criar_a_tarefa_cai_no_run_sozinho() {
        let r = rig(EXE, None);
        *r.sched.register_error.lock().unwrap() = Some(0x8007_0005_u32 as i32);
        assert_eq!(r.svc.enable(), Ok(RUN));
        assert!(
            *r.sched.deletes.lock().unwrap() >= 1,
            "tarefa removida antes do Run"
        );
        assert_eq!(*r.run.state.lock().unwrap(), Some(RunState::Enabled));
        assert_eq!(r.svc.status(), RUN);
    }

    #[test]
    fn tarefa_criada_com_conteudo_diferente_do_esperado_cai_no_run() {
        let r = rig(EXE, None);
        *r.sched.after_register.lock().unwrap() = Some(TaskState::Foreign);
        assert_eq!(r.svc.enable(), Ok(RUN));
        assert_eq!(*r.sched.state.lock().unwrap(), Some(TaskState::Absent));
    }

    #[test]
    fn run_tambem_falhando_e_autostart_failed() {
        let r = rig(EXE, None);
        *r.sched.register_error.lock().unwrap() = Some(0x8007_0005_u32 as i32);
        *r.run.set_error.lock().unwrap() = Some(5);
        assert_eq!(r.svc.enable(), Err(AppError::AutostartFailed));
        assert_eq!(r.svc.status(), AutostartStatus::OFF);
    }

    /// Ligar a tarefa tira o Run (um mecanismo por vez).
    #[test]
    fn ligar_a_tarefa_remove_o_run_que_existia() {
        let r = rig(EXE, None);
        *r.run.state.lock().unwrap() = Some(RunState::Enabled);
        assert_eq!(r.svc.enable(), Ok(TASK));
        assert_eq!(*r.run.state.lock().unwrap(), Some(RunState::Absent));
    }

    #[test]
    fn desligar_remove_os_dois() {
        let r = rig(EXE, None);
        *r.sched.state.lock().unwrap() = Some(TaskState::Mine { last_result: 0 });
        *r.run.state.lock().unwrap() = Some(RunState::Enabled);
        assert_eq!(r.svc.disable(), Ok(AutostartStatus::OFF));
        assert_eq!(r.svc.status(), AutostartStatus::OFF);
        // Tambem da pasta de desenvolvimento.
        let dev = rig(DEV_EXE, None);
        *dev.run.state.lock().unwrap() = Some(RunState::Enabled);
        assert_eq!(dev.svc.disable(), Ok(AutostartStatus::OFF));
        assert_eq!(*dev.run.state.lock().unwrap(), Some(RunState::Absent));
    }

    #[test]
    fn desligar_com_erro_do_agendador_e_autostart_failed() {
        let r = rig(EXE, None);
        *r.sched.delete_error.lock().unwrap() = Some(0x8007_0005_u32 as i32);
        assert_eq!(r.svc.disable(), Err(AppError::AutostartFailed));
        // O Run foi removido mesmo assim (as duas remocoes sao tentadas).
        assert_eq!(*r.run.removes.lock().unwrap(), 1);
    }

    /// Tarefa forjada com o mesmo nome (apontando para `calc.exe`): nao vale como autostart.
    #[test]
    fn tarefa_forjada_nao_e_tratada_como_valida() {
        let r = rig(EXE, None);
        *r.sched.state.lock().unwrap() = Some(TaskState::Foreign);
        assert_eq!(r.svc.status(), AutostartStatus::OFF);
        // Ligar a recria.
        assert_eq!(r.svc.enable(), Ok(TASK));
        assert!(*r.sched.deletes.lock().unwrap() >= 1);
    }

    #[test]
    fn reconciliar_remove_a_tarefa_estranha_sem_ligar_nada() {
        let r = rig(EXE, None);
        *r.sched.state.lock().unwrap() = Some(TaskState::Foreign);
        r.svc.reconcile_on_open();
        assert_eq!(*r.sched.state.lock().unwrap(), Some(TaskState::Absent));
        assert_eq!(*r.run.state.lock().unwrap(), None);
    }

    /// SEC-22 rev. 4 (c): os codigos "nao e falha" nao trocam de mecanismo.
    #[test]
    fn codigos_de_nao_falha_nao_disparam_o_fallback() {
        for code in [0, 0x41301, 0x41303, 0x41325, 0x41300, 0x41306] {
            let r = rig(EXE, None);
            *r.sched.state.lock().unwrap() = Some(TaskState::Mine { last_result: code });
            r.svc.reconcile_on_open();
            assert_eq!(r.svc.status(), TASK, "{code:#x}");
            assert_eq!(*r.run.state.lock().unwrap(), None, "{code:#x}");
        }
    }

    #[test]
    fn falha_real_na_ultima_execucao_troca_para_o_run_e_nao_volta_sozinha() {
        let r = rig(EXE, None);
        *r.sched.state.lock().unwrap() = Some(TaskState::Mine {
            last_result: 0x8007_0002_u32 as i32,
        });
        r.svc.reconcile_on_open();
        assert_eq!(r.svc.status(), RUN);
        assert_eq!(*r.sched.state.lock().unwrap(), Some(TaskState::Absent));
        // Abrir de novo: continua no Run, sem recriar a tarefa (SEC-22 (d)).
        let registrados = r.sched.registered.lock().unwrap().len();
        r.svc.reconcile_on_open();
        r.svc.reconcile_on_open();
        assert_eq!(r.svc.status(), RUN);
        assert_eq!(r.sched.registered.lock().unwrap().len(), registrados);
        // So o usuario religando volta a tentar a tarefa.
        assert_eq!(r.svc.enable(), Ok(TASK));
    }

    #[test]
    fn reconciliar_fora_de_program_files_nao_mexe_em_nada() {
        let r = rig(DEV_EXE, None);
        *r.sched.state.lock().unwrap() = Some(TaskState::Foreign);
        r.svc.reconcile_on_open();
        assert_eq!(*r.sched.deletes.lock().unwrap(), 0);
    }

    #[test]
    fn run_sobrando_ao_lado_da_tarefa_valida_e_removido() {
        let r = rig(EXE, None);
        *r.sched.state.lock().unwrap() = Some(TaskState::Mine { last_result: 0 });
        *r.run.state.lock().unwrap() = Some(RunState::Enabled);
        r.svc.reconcile_on_open();
        assert_eq!(*r.run.state.lock().unwrap(), Some(RunState::Absent));
    }

    #[test]
    fn run_desligado_no_gerenciador_de_tarefas_aparece_como_desligado() {
        let r = rig(EXE, None);
        *r.run.state.lock().unwrap() = Some(RunState::DisabledByUser);
        assert_eq!(
            r.svc.status(),
            AutostartStatus {
                enabled: false,
                mechanism: Mechanism::Run
            }
        );
    }

    #[test]
    fn run_estranho_nao_conta_como_ligado() {
        let r = rig(EXE, None);
        *r.run.state.lock().unwrap() = Some(RunState::Foreign);
        assert_eq!(r.svc.status(), AutostartStatus::OFF);
    }

    /// SEC-21: `InstallDefaults` DWORD 1 liga; 0 e 7 (e ausente) deixam desligado; marca aplicado.
    #[test]
    fn padrao_do_instalador_por_tabela() {
        for (valor, liga) in [
            (Some(1), true),
            (Some(0), false),
            (Some(7), false),
            (Some(2), false),
            (None, false),
        ] {
            let r = rig(EXE, valor);
            r.svc.apply_install_default_once();
            assert!(*r.store.applied.lock().unwrap(), "{valor:?}");
            assert_eq!(r.svc.status().enabled, liga, "{valor:?}");
            assert_eq!(r.sched.registered.lock().unwrap().len(), usize::from(liga));
        }
    }

    #[test]
    fn padrao_do_instalador_so_roda_uma_vez() {
        let r = rig(EXE, Some(1));
        r.svc.apply_install_default_once();
        r.svc.disable().unwrap(); // o usuario desligou depois
        r.svc.apply_install_default_once();
        assert_eq!(r.svc.status(), AutostartStatus::OFF);
        assert_eq!(r.sched.registered.lock().unwrap().len(), 1);
    }

    #[test]
    fn padrao_do_instalador_que_falha_ao_ligar_nao_e_marcado() {
        let r = rig(EXE, Some(1));
        *r.sched.register_error.lock().unwrap() = Some(1);
        *r.run.set_error.lock().unwrap() = Some(1);
        r.svc.apply_install_default_once();
        assert!(!*r.store.applied.lock().unwrap());
    }

    /// A copia de desenvolvimento compartilha `settings.json` com a instalada: ela nao pode
    /// gastar o "uma vez".
    #[test]
    fn padrao_do_instalador_fora_de_program_files_nao_faz_nem_marca() {
        let r = rig(DEV_EXE, Some(1));
        r.svc.apply_install_default_once();
        assert!(!*r.store.applied.lock().unwrap());
        assert!(r.sched.registered.lock().unwrap().is_empty());
    }
}
