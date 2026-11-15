//! Job Object com completion port (BE-16/BE-17, ADR-0005 rev. 6). O jogo iniciado pelo Sofa entra
//! num Job que so **rastreia**: entrada e saida de processos chegam pela porta, sem polling, e a
//! sonda decide quem e membro do jogo (`classify`). O Job nunca e usado para encerrar nada:
//! `TerminateJobObject` e proibido (SEC-BUG-01, INV-05). **Sem** `KILL_ON_JOB_CLOSE`: fechar o Sofa nao fecha o jogo
//! (CA-05, RN-16).

use std::ffi::c_void;

use windows::core::Result as WinResult;
use windows::Win32::Foundation::{CloseHandle, HANDLE, INVALID_HANDLE_VALUE};
use windows::Win32::System::JobObjects::{
    AssignProcessToJobObject, CreateJobObjectW, IsProcessInJob,
    JobObjectAssociateCompletionPortInformation, JobObjectBasicProcessIdList,
    QueryInformationJobObject, SetInformationJobObject, JOBOBJECT_ASSOCIATE_COMPLETION_PORT,
};
use windows::Win32::System::Threading::INFINITE;
use windows::Win32::System::IO::{
    CreateIoCompletionPort, GetQueuedCompletionStatus, PostQueuedCompletionStatus, OVERLAPPED,
};

/// Chave que identifica o Job na porta; a de parada e outra.
const JOB_KEY: usize = 0x50;
const STOP_KEY: usize = 0x51;

/// Mensagens do Job (`JOB_OBJECT_MSG_*`).
const MSG_ACTIVE_PROCESS_ZERO: u32 = 4;
const MSG_NEW_PROCESS: u32 = 6;
const MSG_EXIT_PROCESS: u32 = 7;
const MSG_ABNORMAL_EXIT_PROCESS: u32 = 8;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JobMessage {
    NewProcess(u32),
    ExitProcess,
    /// Nenhum processo ativo no Job.
    ActiveZero,
    Other,
    /// Alguem chamou `wake` (encerrar a espera).
    Stop,
    /// A porta falhou (handle fechado).
    Failed,
}

pub struct Job {
    job: HANDLE,
    port: HANDLE,
}

// SAFETY: os handles sao so identificadores do kernel; as chamadas usadas aqui sao seguras entre
// threads (a documentacao do Win32 permite usar o mesmo handle de Job e de porta em varias).
unsafe impl Send for Job {}
unsafe impl Sync for Job {}

impl Job {
    pub fn new() -> WinResult<Job> {
        // SAFETY: argumentos opcionais nulos; handles fechados no `Drop` (ou aqui, em erro).
        unsafe {
            let job = CreateJobObjectW(None, None)?;
            let port = match CreateIoCompletionPort(INVALID_HANDLE_VALUE, None, 0, 1) {
                Ok(p) => p,
                Err(e) => {
                    let _ = CloseHandle(job);
                    return Err(e);
                }
            };
            let assoc = JOBOBJECT_ASSOCIATE_COMPLETION_PORT {
                CompletionKey: JOB_KEY as *mut c_void,
                CompletionPort: port,
            };
            if let Err(e) = SetInformationJobObject(
                job,
                JobObjectAssociateCompletionPortInformation,
                std::ptr::from_ref(&assoc).cast(),
                std::mem::size_of::<JOBOBJECT_ASSOCIATE_COMPLETION_PORT>() as u32,
            ) {
                let _ = CloseHandle(port);
                let _ = CloseHandle(job);
                return Err(e);
            }
            Ok(Job { job, port })
        }
    }

    /// Poe o processo (criado suspenso) no Job.
    pub fn assign(&self, process: HANDLE) -> WinResult<()> {
        // SAFETY: `process` e um handle valido do chamador.
        unsafe { AssignProcessToJobObject(self.job, process) }
    }

    /// Espera a proxima mensagem do Job (bloqueia, sem polling).
    pub fn wait_message(&self) -> JobMessage {
        let mut bytes = 0u32;
        let mut key = 0usize;
        let mut overlapped: *mut OVERLAPPED = std::ptr::null_mut();
        // SAFETY: ponteiros para locais validos; `port` vive enquanto o `Job` existir.
        let r = unsafe {
            GetQueuedCompletionStatus(self.port, &mut bytes, &mut key, &mut overlapped, INFINITE)
        };
        if r.is_err() && overlapped.is_null() {
            return JobMessage::Failed;
        }
        if key == STOP_KEY {
            return JobMessage::Stop;
        }
        match bytes {
            MSG_NEW_PROCESS => JobMessage::NewProcess(overlapped as usize as u32),
            MSG_EXIT_PROCESS | MSG_ABNORMAL_EXIT_PROCESS => JobMessage::ExitProcess,
            MSG_ACTIVE_PROCESS_ZERO => JobMessage::ActiveZero,
            _ => JobMessage::Other,
        }
    }

    /// Acorda quem esta em `wait_message` com `Stop`.
    pub fn wake(&self) {
        // SAFETY: `port` valido; sem `OVERLAPPED`.
        unsafe {
            let _ = PostQueuedCompletionStatus(self.port, 0, STOP_KEY, None);
        }
    }

    /// PIDs dos processos vivos no Job.
    pub fn pids(&self) -> Vec<u32> {
        // Cabecalho de 2 DWORDs (+ padding) e N `usize`. Comeca com folga para 256 processos.
        for capacity in [256usize, 4096] {
            let words = 2 + capacity; // em `usize`, com folga para o cabecalho
            let mut buf = vec![0usize; words];
            let bytes = (buf.len() * std::mem::size_of::<usize>()) as u32;
            // SAFETY: `buf` tem `bytes` bytes, alinhado a `usize`; o Win32 escreve no maximo isso.
            let ok = unsafe {
                QueryInformationJobObject(
                    self.job,
                    JobObjectBasicProcessIdList,
                    buf.as_mut_ptr().cast(),
                    bytes,
                    None,
                )
                .is_ok()
            };
            if !ok {
                continue; // ERROR_MORE_DATA: tenta maior
            }
            // JOBOBJECT_BASIC_PROCESS_ID_LIST: { u32 assigned; u32 in_list; usize ids[] }
            let header = buf[0];
            let assigned = (header & 0xFFFF_FFFF) as usize;
            let in_list = (header >> 32) as usize;
            if in_list < assigned && capacity < 4096 {
                continue;
            }
            return buf[1..1 + in_list.min(capacity)]
                .iter()
                .map(|p| *p as u32)
                .collect();
        }
        Vec::new()
    }

    /// `true` se o processo (handle ja aberto pelo chamador) pertence a este Job. Fecha a corrida
    /// de reuso de PID entre a lista do Job e a abertura do handle.
    pub fn contains(&self, process: HANDLE) -> bool {
        let mut inside = windows::Win32::Foundation::BOOL(0);
        // SAFETY: `process` valido do chamador; `inside` e um local valido.
        unsafe { IsProcessInJob(process, self.job, &mut inside).is_ok() && inside.as_bool() }
    }
}

impl Drop for Job {
    fn drop(&mut self) {
        // SAFETY: handles proprios; sem KILL_ON_JOB_CLOSE, o jogo continua vivo.
        unsafe {
            let _ = CloseHandle(self.port);
            let _ = CloseHandle(self.job);
        }
    }
}
