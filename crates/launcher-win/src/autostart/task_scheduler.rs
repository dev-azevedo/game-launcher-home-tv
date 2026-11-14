//! Tarefa de logon do proprio usuario por `ITaskService` (BE-36, SEC-22, ADR-0006, SP-07).
//!
//! A `ITaskDefinition` e montada **campo a campo**: nada de `schtasks.exe`, shell ou XML
//! concatenado. Travas obrigatorias: `Principal.UserId` = SID do usuario atual, `LogonType =
//! INTERACTIVE_TOKEN`, `RunLevel = LUA` (LeastPrivilege; nunca HIGHEST, senha, S4U ou outra
//! conta), um unico `LogonTrigger` do mesmo SID, uma unica `IExecAction` (`"<exe>"` +
//! `--autostart`, sem `WorkingDirectory`), `ExecutionTimeLimit = PT0S` e `IgnoreNew`. O nome e
//! `Sofa Autostart (<SID>)` na raiz do Agendador. Unico arquivo que registra tarefa (INV-03).
//!
//! `state()` so devolve `Mine` quando **tudo** acima bate com o que o Sofa gravaria (SP-07: o
//! `UserId` pela API nao volta como SID literal, entao a conta e resolvida e comparada por SID; o
//! `Path` e comparado sem aspas). Uma tarefa com o mesmo nome e outro conteudo e `Foreign`.
//!
//! Cada operacao roda numa thread MTA propria e curta (COM inicializado e desfeito ali), para nao
//! depender do modelo de apartamento de quem chama.

use launcher_core::application::ports::{SchedulerError, TaskScheduler, TaskState};
use launcher_core::domain::autostart::{
    exe_matches, is_valid_sid, quoted_exe, task_name, AUTOSTART_ARG,
};
use windows::core::VARIANT;
use windows::core::{Interface, BSTR, PCWSTR, PWSTR};
use windows::Win32::Foundation::{
    CloseHandle, LocalFree, ERROR_INSUFFICIENT_BUFFER, HANDLE, HLOCAL, VARIANT_FALSE, VARIANT_TRUE,
};
use windows::Win32::Security::Authorization::ConvertSidToStringSidW;
use windows::Win32::Security::{
    GetTokenInformation, LookupAccountNameW, TokenUser, PSID, SID_NAME_USE, TOKEN_QUERY, TOKEN_USER,
};
use windows::Win32::System::Com::{
    CoCreateInstance, CoInitializeEx, CoUninitialize, CLSCTX_INPROC_SERVER, COINIT_MULTITHREADED,
};
use windows::Win32::System::TaskScheduler::{
    IExecAction, ILogonTrigger, IRegisteredTask, ITaskFolder, ITaskService,
    TaskScheduler as TaskSchedulerClass, TASK_ACTION_EXEC, TASK_CREATE_OR_UPDATE,
    TASK_INSTANCES_IGNORE_NEW, TASK_LOGON_INTERACTIVE_TOKEN, TASK_LOGON_TYPE, TASK_RUNLEVEL_LUA,
    TASK_RUNLEVEL_TYPE, TASK_TRIGGER_LOGON, TASK_TRIGGER_TYPE2,
};
use windows::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};

/// `HRESULT_FROM_WIN32(ERROR_FILE_NOT_FOUND)` e `(ERROR_PATH_NOT_FOUND)`: tarefa inexistente.
const NOT_FOUND: [i32; 2] = [0x8007_0002_u32 as i32, 0x8007_0003_u32 as i32];
const E_FAIL: i32 = 0x8000_4005_u32 as i32;

fn herr(e: &windows::core::Error) -> SchedulerError {
    SchedulerError(e.code().0)
}

/// SID do usuario do processo, em texto (`S-1-5-21-...`).
pub fn current_user_sid() -> Result<String, SchedulerError> {
    // SAFETY: token aberto e fechado aqui; buffer dimensionado pela propria API; o texto do SID
    // vem de `ConvertSidToStringSidW` e e liberado com `LocalFree` depois de copiado.
    unsafe {
        let mut token = HANDLE::default();
        OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token).map_err(|e| herr(&e))?;
        let result = (|| {
            let mut len = 0u32;
            let _ = GetTokenInformation(token, TokenUser, None, 0, &mut len);
            if len == 0 {
                return Err(SchedulerError(E_FAIL));
            }
            let mut buf = vec![0u8; len as usize];
            GetTokenInformation(
                token,
                TokenUser,
                Some(buf.as_mut_ptr().cast()),
                len,
                &mut len,
            )
            .map_err(|e| herr(&e))?;
            let user = &*(buf.as_ptr().cast::<TOKEN_USER>());
            sid_to_string(user.User.Sid)
        })();
        let _ = CloseHandle(token);
        result
    }
}

unsafe fn sid_to_string(sid: PSID) -> Result<String, SchedulerError> {
    let mut out = PWSTR::null();
    // SAFETY: `sid` e valido (vem do token ou de `LookupAccountNameW`); `out` e liberado abaixo.
    unsafe {
        ConvertSidToStringSidW(sid, &mut out).map_err(|e| herr(&e))?;
        let text = out.to_string().map_err(|_| SchedulerError(E_FAIL));
        let _ = LocalFree(HLOCAL(out.0.cast()));
        text
    }
}

/// `userid` (como o Agendador devolve: SID ou `DOMINIO\\conta`) e a conta `sid`?
fn same_account(userid: &str, sid: &str) -> bool {
    if userid.trim().eq_ignore_ascii_case(sid) {
        return true;
    }
    if userid.trim().is_empty() {
        return false;
    }
    let name: Vec<u16> = userid.trim().encode_utf16().chain(Some(0)).collect();
    // SAFETY: duas chamadas: a primeira so mede os buffers (falha esperada com
    // INSUFFICIENT_BUFFER); a segunda enche buffers do tamanho pedido.
    unsafe {
        let (mut sid_len, mut dom_len) = (0u32, 0u32);
        let mut kind = SID_NAME_USE::default();
        let first = LookupAccountNameW(
            PCWSTR::null(),
            PCWSTR(name.as_ptr()),
            PSID::default(),
            &mut sid_len,
            PWSTR::null(),
            &mut dom_len,
            &mut kind,
        );
        match first {
            Err(e) if e.code() == ERROR_INSUFFICIENT_BUFFER.to_hresult() => {}
            _ => return false,
        }
        let mut sid_buf = vec![0u8; sid_len as usize];
        let mut dom_buf = vec![0u16; dom_len as usize];
        let psid = PSID(sid_buf.as_mut_ptr().cast());
        if LookupAccountNameW(
            PCWSTR::null(),
            PCWSTR(name.as_ptr()),
            psid,
            &mut sid_len,
            PWSTR(dom_buf.as_mut_ptr()),
            &mut dom_len,
            &mut kind,
        )
        .is_err()
        {
            return false;
        }
        sid_to_string(psid).is_ok_and(|s| s.eq_ignore_ascii_case(sid))
    }
}

/// Roda `f` numa thread MTA propria, com o COM inicializado so ali.
fn on_com<T: Send>(
    f: impl FnOnce() -> Result<T, SchedulerError> + Send,
) -> Result<T, SchedulerError> {
    std::thread::scope(|s| {
        s.spawn(|| {
            // SAFETY: par CoInitializeEx/CoUninitialize na mesma thread; todos os objetos COM
            // criados por `f` morrem dentro dele, antes do `CoUninitialize`.
            let init = unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) };
            if init.is_err() {
                return Err(SchedulerError(init.0));
            }
            let r = f();
            unsafe { CoUninitialize() };
            r
        })
        .join()
        .unwrap_or(Err(SchedulerError(E_FAIL)))
    })
}

fn root_folder() -> Result<ITaskFolder, SchedulerError> {
    // SAFETY: COM ja inicializado pela thread de `on_com`.
    unsafe {
        let service: ITaskService =
            CoCreateInstance(&TaskSchedulerClass, None, CLSCTX_INPROC_SERVER)
                .map_err(|e| herr(&e))?;
        let empty = VARIANT::default();
        service
            .Connect(&empty, &empty, &empty, &empty)
            .map_err(|e| herr(&e))?;
        service.GetFolder(&BSTR::from("\\")).map_err(|e| herr(&e))
    }
}

#[derive(Debug, Clone)]
pub struct WinTaskScheduler {
    sid: String,
    name: String,
}

impl WinTaskScheduler {
    /// A tarefa real do Sofa para o usuario do processo: `Sofa Autostart (<SID>)`.
    pub fn for_current_user() -> Result<Self, SchedulerError> {
        let sid = current_user_sid()?;
        let name = task_name(&sid).ok_or(SchedulerError(E_FAIL))?;
        Ok(Self { sid, name })
    }

    /// Nome proprio (`<prefixo> (<SID>)`) para os testes de integracao nao tocarem na tarefa real.
    pub fn with_test_prefix(prefix: &str) -> Result<Self, SchedulerError> {
        let sid = current_user_sid()?;
        Ok(Self {
            name: format!("{prefix} ({sid})"),
            sid,
        })
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn sid(&self) -> &str {
        &self.sid
    }

    /// `IRegisteredTask::Xml` (so para conferencia em teste).
    pub fn exported_xml(&self) -> Result<String, SchedulerError> {
        on_com(|| {
            let task = self
                .get(&root_folder()?)?
                .ok_or(SchedulerError(NOT_FOUND[0]))?;
            // SAFETY: COM inicializado; `task` e nosso.
            unsafe { task.Xml().map(|b| b.to_string()).map_err(|e| herr(&e)) }
        })
    }

    fn get(&self, folder: &ITaskFolder) -> Result<Option<IRegisteredTask>, SchedulerError> {
        // SAFETY: COM inicializado; o nome vai como BSTR proprio.
        match unsafe { folder.GetTask(&BSTR::from(self.name.as_str())) } {
            Ok(t) => Ok(Some(t)),
            Err(e) if NOT_FOUND.contains(&e.code().0) => Ok(None),
            Err(e) => Err(herr(&e)),
        }
    }

    /// A tarefa bate com o que o Sofa gravaria? (`UserId`, `RunLevel`, `LogonType`, gatilho unico
    /// do mesmo SID, acao unica com o `Path` esperado e `--autostart`, tarefa habilitada).
    fn matches(&self, task: &IRegisteredTask, expected_exe: Option<&str>) -> bool {
        // SAFETY: COM inicializado; todas as interfaces vem de `task` e valem durante a chamada.
        unsafe {
            let Ok(def) = task.Definition() else {
                return false;
            };
            let Ok(principal) = def.Principal() else {
                return false;
            };
            let mut user = BSTR::new();
            let mut logon = TASK_LOGON_TYPE::default();
            let mut level = TASK_RUNLEVEL_TYPE::default();
            if principal.UserId(&mut user).is_err()
                || principal.LogonType(&mut logon).is_err()
                || principal.RunLevel(&mut level).is_err()
            {
                return false;
            }
            if logon != TASK_LOGON_INTERACTIVE_TOKEN
                || level != TASK_RUNLEVEL_LUA
                || !same_account(&user.to_string(), &self.sid)
            {
                return false;
            }
            if task.Enabled().map(|b| b != VARIANT_TRUE).unwrap_or(true) {
                return false;
            }

            // Acao unica: exec com o Path esperado e --autostart.
            let Ok(actions) = def.Actions() else {
                return false;
            };
            let mut count = 0i32;
            if actions.Count(&mut count).is_err() || count != 1 {
                return false;
            }
            let Ok(action) = actions.get_Item(1) else {
                return false;
            };
            let Ok(exec) = action.cast::<IExecAction>() else {
                return false;
            };
            let (mut path, mut args) = (BSTR::new(), BSTR::new());
            if exec.Path(&mut path).is_err() || exec.Arguments(&mut args).is_err() {
                return false;
            }
            if args.to_string().trim() != AUTOSTART_ARG
                || !exe_matches(&path.to_string(), expected_exe)
            {
                return false;
            }

            // Gatilho unico: logon do mesmo SID (nunca "qualquer usuario").
            let Ok(triggers) = def.Triggers() else {
                return false;
            };
            let mut count = 0i32;
            if triggers.Count(&mut count).is_err() || count != 1 {
                return false;
            }
            let Ok(trigger) = triggers.get_Item(1) else {
                return false;
            };
            let mut kind = TASK_TRIGGER_TYPE2::default();
            if trigger.Type(&mut kind).is_err() || kind != TASK_TRIGGER_LOGON {
                return false;
            }
            let Ok(logon_trigger) = trigger.cast::<ILogonTrigger>() else {
                return false;
            };
            let mut trigger_user = BSTR::new();
            logon_trigger.UserId(&mut trigger_user).is_ok()
                && same_account(&trigger_user.to_string(), &self.sid)
        }
    }
}

impl TaskScheduler for WinTaskScheduler {
    fn state(&self, expected_exe: Option<&str>) -> Result<TaskState, SchedulerError> {
        on_com(|| {
            let folder = root_folder()?;
            let Some(task) = self.get(&folder)? else {
                return Ok(TaskState::Absent);
            };
            if !self.matches(&task, expected_exe) {
                return Ok(TaskState::Foreign);
            }
            // SAFETY: COM inicializado; `task` e nosso.
            let last_result = unsafe { task.LastTaskResult() }.map_err(|e| herr(&e))?;
            Ok(TaskState::Mine { last_result })
        })
    }

    fn register(&self, exe: &str) -> Result<(), SchedulerError> {
        if !is_valid_sid(&self.sid) || exe.contains('"') || exe.trim().is_empty() {
            return Err(SchedulerError(E_FAIL));
        }
        on_com(|| {
            // SAFETY: COM inicializado; cada interface vem do `ITaskDefinition` recem-criado.
            unsafe {
                let folder = root_folder()?;
                let service: ITaskService =
                    CoCreateInstance(&TaskSchedulerClass, None, CLSCTX_INPROC_SERVER)
                        .map_err(|e| herr(&e))?;
                let empty = VARIANT::default();
                service
                    .Connect(&empty, &empty, &empty, &empty)
                    .map_err(|e| herr(&e))?;
                let def = service.NewTask(0).map_err(|e| herr(&e))?;
                let sid = BSTR::from(self.sid.as_str());

                // Principal: o proprio usuario, token interativo, sem privilegio extra.
                let principal = def.Principal().map_err(|e| herr(&e))?;
                principal.SetUserId(&sid).map_err(|e| herr(&e))?;
                principal
                    .SetLogonType(TASK_LOGON_INTERACTIVE_TOKEN)
                    .map_err(|e| herr(&e))?;
                principal
                    .SetRunLevel(TASK_RUNLEVEL_LUA)
                    .map_err(|e| herr(&e))?;

                // Configuracoes: sem limite de tempo (o padrao de 72 h encerraria o Sofa), sem
                // segunda instancia, sem condicao de bateria, rede ou ociosidade.
                let settings = def.Settings().map_err(|e| herr(&e))?;
                settings
                    .SetExecutionTimeLimit(&BSTR::from("PT0S"))
                    .map_err(|e| herr(&e))?;
                settings
                    .SetMultipleInstances(TASK_INSTANCES_IGNORE_NEW)
                    .map_err(|e| herr(&e))?;
                settings
                    .SetDisallowStartIfOnBatteries(VARIANT_FALSE)
                    .map_err(|e| herr(&e))?;
                settings
                    .SetStopIfGoingOnBatteries(VARIANT_FALSE)
                    .map_err(|e| herr(&e))?;
                settings
                    .SetRunOnlyIfNetworkAvailable(VARIANT_FALSE)
                    .map_err(|e| herr(&e))?;
                settings
                    .SetRunOnlyIfIdle(VARIANT_FALSE)
                    .map_err(|e| herr(&e))?;
                settings.SetEnabled(VARIANT_TRUE).map_err(|e| herr(&e))?;

                // Gatilho unico: logon deste usuario.
                let triggers = def.Triggers().map_err(|e| herr(&e))?;
                let trigger = triggers.Create(TASK_TRIGGER_LOGON).map_err(|e| herr(&e))?;
                trigger.SetEnabled(VARIANT_TRUE).map_err(|e| herr(&e))?;
                let logon: ILogonTrigger = trigger.cast().map_err(|e| herr(&e))?;
                logon.SetUserId(&sid).map_err(|e| herr(&e))?;

                // Acao unica: o exe instalado entre aspas + --autostart, sem diretorio de trabalho.
                let actions = def.Actions().map_err(|e| herr(&e))?;
                let action = actions.Create(TASK_ACTION_EXEC).map_err(|e| herr(&e))?;
                let exec: IExecAction = action.cast().map_err(|e| herr(&e))?;
                exec.SetPath(&BSTR::from(quoted_exe(exe)))
                    .map_err(|e| herr(&e))?;
                exec.SetArguments(&BSTR::from(AUTOSTART_ARG))
                    .map_err(|e| herr(&e))?;

                folder
                    .RegisterTaskDefinition(
                        &BSTR::from(self.name.as_str()),
                        &def,
                        TASK_CREATE_OR_UPDATE.0,
                        &VARIANT::from(sid),
                        &empty,
                        TASK_LOGON_INTERACTIVE_TOKEN,
                        &empty,
                    )
                    .map_err(|e| herr(&e))?;
                Ok(())
            }
        })
    }

    fn delete(&self) -> Result<(), SchedulerError> {
        on_com(|| {
            let folder = root_folder()?;
            // SAFETY: COM inicializado; nome proprio.
            match unsafe { folder.DeleteTask(&BSTR::from(self.name.as_str()), 0) } {
                Ok(()) => Ok(()),
                Err(e) if NOT_FOUND.contains(&e.code().0) => Ok(()),
                Err(e) => Err(herr(&e)),
            }
        })
    }
}
