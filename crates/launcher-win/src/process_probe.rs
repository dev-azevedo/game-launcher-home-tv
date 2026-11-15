//! `SessionProbe` Win32 (BE-17, ADR-0005, SEC-15, SEC-16, SEC-17).
//!
//! Duas estrategias atras da mesma interface:
//! - **Job** (manual e GOG): o Job Object do `CreateProcessW` com completion port so **rastreia**
//!   (entrada e saida de processo chegam pela porta, sem polling nem CPU). Cada PID do Job passa
//!   pelo mesmo `classify` da varredura (SEC-17) e so os membros do jogo contam para o fim da
//!   sessao, para "Fechar" e para "Forcar", por handle e tempo de criacao. Nunca ha
//!   `TerminateJobObject` (SEC-BUG-01, ADR-0005 rev. 6).
//! - **Varredura** (Steam, Epic e jogo elevado, que fica fora do Job por UIPI): snapshot ToolHelp
//!   so de PIDs novos (cache por PID, revalidado a cada ~60 s), handle
//!   `PROCESS_QUERY_LIMITED_INFORMATION | SYNCHRONIZE` com tempo de criacao, associacao por
//!   pasta pelo dominio (`classify`), intervalo de 1 s em `Launching`/`Ending` e 2 s em
//!   `Running`, e espera nos handles dos membros para ver o fim na hora. Na Steam,
//!   `RunningAppID` mantem a sessao viva.
//!
//! Sem `SeDebugPrivilege`, sem `PROCESS_ALL_ACCESS`. O que nao abre com os direitos minimos
//! (protegido ou de sistema) nunca entra no conjunto.

use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, PoisonError};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use launcher_core::application::ports::{
    ProbeConfig, ProbeSink, SessionProbe, SessionWatch, WatchPlan,
};
use launcher_core::application::AppError;
use launcher_core::domain::session::{classify, ProcessInfo, SessionContext};
use windows::core::PCWSTR;
use windows::Win32::Foundation::{
    CloseHandle, GetLastError, ERROR_ACCESS_DENIED, FILETIME, HANDLE, LPARAM, WAIT_OBJECT_0, WPARAM,
};
use windows::Win32::Security::{
    EqualSid, GetTokenInformation, TokenUser, PSID, TOKEN_QUERY, TOKEN_USER,
};
use windows::Win32::System::Diagnostics::ToolHelp::{
    CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W, TH32CS_SNAPPROCESS,
};
use windows::Win32::System::Registry::{
    RegGetValueW, HKEY_CURRENT_USER, RRF_RT_REG_DWORD, RRF_RT_REG_SZ,
};
use windows::Win32::System::RemoteDesktop::ProcessIdToSessionId;
use windows::Win32::System::Threading::{
    CreateEventW, GetCurrentProcess, GetCurrentProcessId, GetProcessTimes, OpenProcess,
    OpenProcessToken, QueryFullProcessImageNameW, SetEvent, TerminateProcess,
    WaitForMultipleObjects, WaitForSingleObject, PROCESS_NAME_WIN32,
    PROCESS_QUERY_LIMITED_INFORMATION, PROCESS_SYNCHRONIZE, PROCESS_TERMINATE,
};
use windows::Win32::UI::WindowsAndMessaging::{PostMessageW, WM_CLOSE};

use crate::job::{Job, JobMessage};
use crate::windows_enum::{hwnd as to_hwnd, top_level_windows};

/// Nao membros sao reavaliados a cada ~60 s (reuso de PID, ADR-0005).
const REVALIDATE_AFTER: Duration = Duration::from_secs(60);
const MAX_WAIT_HANDLES: usize = 63;

// ---------------------------------------------------------------------------- handles

/// Handle de processo ou evento que se fecha sozinho.
struct Owned(HANDLE);

// SAFETY: identificador do kernel; as chamadas usadas (Wait*, Terminate*, Close) valem em
// qualquer thread.
unsafe impl Send for Owned {}
unsafe impl Sync for Owned {}

impl Drop for Owned {
    fn drop(&mut self) {
        // SAFETY: handle proprio, fechado uma unica vez.
        unsafe {
            let _ = CloseHandle(self.0);
        }
    }
}

fn filetime_u64(ft: FILETIME) -> u64 {
    (u64::from(ft.dwHighDateTime) << 32) | u64::from(ft.dwLowDateTime)
}

/// Tempo de criacao do processo (FILETIME), o que distingue um PID reaproveitado.
pub fn creation_time(h: HANDLE) -> Option<u64> {
    let (mut c, mut e, mut k, mut u) = (
        FILETIME::default(),
        FILETIME::default(),
        FILETIME::default(),
        FILETIME::default(),
    );
    // SAFETY: ponteiros para locais validos.
    unsafe { GetProcessTimes(h, &mut c, &mut e, &mut k, &mut u).ok()? };
    Some(filetime_u64(c))
}

fn image_path(h: HANDLE) -> Option<String> {
    let mut buf = [0u16; 1024];
    let mut n = buf.len() as u32;
    // SAFETY: `buf` e `n` consistentes; `h` tem QUERY_LIMITED.
    unsafe {
        QueryFullProcessImageNameW(
            h,
            PROCESS_NAME_WIN32,
            windows::core::PWSTR(buf.as_mut_ptr()),
            &mut n,
        )
        .ok()?;
    }
    Some(String::from_utf16_lossy(&buf[..n as usize]))
}

fn is_signaled(h: HANDLE) -> bool {
    // SAFETY: `h` valido (tem SYNCHRONIZE).
    unsafe { WaitForSingleObject(h, 0) == WAIT_OBJECT_0 }
}

fn session_of(pid: u32) -> Option<u32> {
    let mut s = 0u32;
    // SAFETY: ponteiro para local valido.
    unsafe { ProcessIdToSessionId(pid, &mut s).ok()? };
    Some(s)
}

/// SID do dono do token do processo, em bytes. `None` se o token nao abre.
fn user_sid(process: HANDLE) -> Option<Vec<u8>> {
    // SAFETY: token aberto e fechado aqui; buffer com o tamanho informado.
    unsafe {
        let mut token = HANDLE::default();
        OpenProcessToken(process, TOKEN_QUERY, &mut token).ok()?;
        let mut len = 0u32;
        let _ = GetTokenInformation(token, TokenUser, None, 0, &mut len);
        let mut buf = vec![0u64; (len as usize).div_ceil(8).max(1)];
        let ok = GetTokenInformation(
            token,
            TokenUser,
            Some(buf.as_mut_ptr().cast()),
            len,
            &mut len,
        )
        .is_ok();
        let _ = CloseHandle(token);
        if !ok {
            return None;
        }
        let user = &*(buf.as_ptr().cast::<TOKEN_USER>());
        // Copia o SID para um buffer proprio (o do token morre aqui).
        let sid = user.User.Sid;
        let len = windows::Win32::Security::GetLengthSid(sid) as usize;
        Some(std::slice::from_raw_parts(sid.0.cast::<u8>(), len).to_vec())
    }
}

fn sids_equal(a: &[u8], b: &[u8]) -> bool {
    // SAFETY: os dois buffers sao SIDs completos copiados de tokens.
    unsafe { EqualSid(PSID(a.as_ptr() as *mut _), PSID(b.as_ptr() as *mut _)).is_ok() }
}

// -------------------------------------------------------------------------- varredura

struct Member {
    handle: Owned,
    creation: u64,
}

struct CacheEntry {
    parent: u32,
    checked: Instant,
}

/// Estado da varredura: conjunto de membros e cache de nao membros.
struct Scanner {
    members: HashMap<u32, Member>,
    cache: HashMap<u32, CacheEntry>,
    ctx: SessionContext,
    own_sid: Option<Vec<u8>>,
}

impl Scanner {
    fn new(install_dir: String, store_dirs: Vec<String>) -> Self {
        // SAFETY: handle pseudo do proprio processo.
        let own_pid = unsafe { GetCurrentProcessId() };
        let own_sid = user_sid(unsafe { GetCurrentProcess() });
        Self {
            members: HashMap::new(),
            cache: HashMap::new(),
            ctx: SessionContext {
                own_pid,
                own_session_id: session_of(own_pid).unwrap_or(0),
                own_child_pids: Vec::new(),
                install_dir,
                store_dirs,
            },
            own_sid,
        }
    }

    /// `(pid, pai, nome do exe)` de cada processo.
    fn snapshot() -> Vec<(u32, u32, String)> {
        let mut out = Vec::with_capacity(512);
        // SAFETY: snapshot fechado ao fim; `entry` com `dwSize` correto.
        unsafe {
            let Ok(snap) = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) else {
                return out;
            };
            let mut entry = PROCESSENTRY32W {
                dwSize: std::mem::size_of::<PROCESSENTRY32W>() as u32,
                ..Default::default()
            };
            let mut ok = Process32FirstW(snap, &mut entry).is_ok();
            while ok {
                let len = entry
                    .szExeFile
                    .iter()
                    .position(|c| *c == 0)
                    .unwrap_or(entry.szExeFile.len());
                out.push((
                    entry.th32ProcessID,
                    entry.th32ParentProcessID,
                    String::from_utf16_lossy(&entry.szExeFile[..len]),
                ));
                ok = Process32NextW(snap, &mut entry).is_ok();
            }
            let _ = CloseHandle(snap);
        }
        out
    }

    /// `msedgewebview2.exe` do proprio Sofa, por fecho transitivo dos pais. So o WebView2: a
    /// Steam iniciada pelo Sofa e os jogos dela tambem sao descendentes e precisam entrar.
    fn own_webview_processes(&self, procs: &[(u32, u32, String)]) -> Vec<u32> {
        let mut set: HashSet<u32> = HashSet::from([self.ctx.own_pid]);
        loop {
            let before = set.len();
            for (pid, parent, name) in procs {
                if set.contains(parent) && name.eq_ignore_ascii_case("msedgewebview2.exe") {
                    set.insert(*pid);
                }
            }
            if set.len() == before {
                break;
            }
        }
        set.remove(&self.ctx.own_pid);
        set.into_iter().collect()
    }

    /// Uma varredura do sistema. Devolve se ha membros.
    fn scan(&mut self) -> bool {
        self.scan_with(None, None)
    }

    /// Modo Job: avalia so os PIDs que o Job lista agora e confirma `IsProcessInJob` no handle.
    fn scan_job(&mut self, job: &Job) -> bool {
        let only: HashSet<u32> = job.pids().into_iter().collect();
        self.scan_with(Some(&only), Some(job))
    }

    fn scan_with(&mut self, only: Option<&HashSet<u32>>, job: Option<&Job>) -> bool {
        let procs = Self::snapshot();
        self.ctx.own_child_pids = self.own_webview_processes(&procs);
        let live: HashSet<u32> = procs.iter().map(|(p, _, _)| *p).collect();

        for &(pid, parent, _) in &procs {
            if pid == 0 || pid == 4 || pid == self.ctx.own_pid || self.members.contains_key(&pid) {
                continue;
            }
            if only.is_some_and(|set| !set.contains(&pid)) {
                continue;
            }
            if let Some(c) = self.cache.get(&pid) {
                if c.parent == parent && c.checked.elapsed() < REVALIDATE_AFTER {
                    continue;
                }
            }
            self.cache.insert(
                pid,
                CacheEntry {
                    parent,
                    checked: Instant::now(),
                },
            );
            // SAFETY: handle devolvido e fechado por `Owned` (ou aqui, se nao for membro).
            let handle = match unsafe {
                OpenProcess(
                    PROCESS_QUERY_LIMITED_INFORMATION | PROCESS_SYNCHRONIZE,
                    false,
                    pid,
                )
            } {
                Ok(h) => Owned(h),
                Err(_) => continue, // protegido ou de sistema: nunca entra (SEC-17)
            };
            if job.is_some_and(|j| !j.contains(handle.0)) {
                continue; // PID reaproveitado: nao e o processo que o Job listou
            }
            let Some(image) = image_path(handle.0) else {
                continue;
            };
            let same_user = match (&self.own_sid, user_sid(handle.0)) {
                (Some(mine), Some(theirs)) => sids_equal(mine, &theirs),
                // Token ilegivel com os direitos minimos: nao arrisca (SEC-17).
                _ => false,
            };
            let info = ProcessInfo {
                pid,
                parent_pid: Some(parent),
                image_path: image,
                session_id: session_of(pid).unwrap_or(u32::MAX),
                same_user,
                openable: true,
            };
            if classify(&info, &self.ctx).is_ok() {
                if let Some(creation) = creation_time(handle.0) {
                    self.members.insert(pid, Member { handle, creation });
                }
            }
        }

        self.cache.retain(|p, _| live.contains(p));
        self.members.retain(|_, m| !is_signaled(m.handle.0));
        !self.members.is_empty()
    }

    fn member_pids(&self) -> Vec<u32> {
        self.members.keys().copied().collect()
    }

    /// Encerra so os membros, cada um por handle e tempo de criacao (SEC-15, SEC-17).
    fn terminate_members(&self) -> Result<(), AppError> {
        let mut denied = false;
        for (pid, m) in &self.members {
            if is_signaled(m.handle.0) {
                continue;
            }
            if terminate_if_same(*pid, m.creation).is_err() {
                denied = true;
            }
        }
        if denied {
            Err(AppError::AccessDenied)
        } else {
            Ok(())
        }
    }
}

/// `RunningAppID` da Steam: o jogo em execucao, 0 se nenhum.
fn steam_running_app_id() -> u32 {
    let mut value = 0u32;
    let mut size = std::mem::size_of::<u32>() as u32;
    let key: Vec<u16> = "Software\\Valve\\Steam\0".encode_utf16().collect();
    let name: Vec<u16> = "RunningAppID\0".encode_utf16().collect();
    // SAFETY: buffers validos; leitura de DWORD em HKCU.
    unsafe {
        if RegGetValueW(
            HKEY_CURRENT_USER,
            PCWSTR(key.as_ptr()),
            PCWSTR(name.as_ptr()),
            RRF_RT_REG_DWORD,
            None,
            Some(std::ptr::from_mut(&mut value).cast()),
            Some(&mut size),
        )
        .is_ok()
        {
            value
        } else {
            0
        }
    }
}

/// `HKCU\Software\Valve\Steam\SteamPath` (barras normais e unidade minuscula).
fn steam_path() -> Option<String> {
    let key: Vec<u16> = "Software\\Valve\\Steam\0".encode_utf16().collect();
    let name: Vec<u16> = "SteamPath\0".encode_utf16().collect();
    let mut buf = [0u16; 512];
    let mut size = std::mem::size_of_val(&buf) as u32;
    // SAFETY: buffer e tamanho consistentes.
    let rc = unsafe {
        RegGetValueW(
            HKEY_CURRENT_USER,
            PCWSTR(key.as_ptr()),
            PCWSTR(name.as_ptr()),
            RRF_RT_REG_SZ,
            None,
            Some(buf.as_mut_ptr().cast()),
            Some(&mut size),
        )
    };
    if rc.is_err() {
        return None;
    }
    let len = buf.iter().position(|c| *c == 0).unwrap_or(buf.len());
    Some(String::from_utf16_lossy(&buf[..len]).replace('/', "\\"))
}

/// Pastas do **cliente** das lojas (nunca a pasta onde ficam os jogos): processos ali nunca
/// contam (RN-10). O `steam.exe` e afins tambem saem por nome, no dominio.
pub fn client_store_dirs() -> Vec<String> {
    let mut dirs = Vec::new();
    if let Some(steam) = steam_path() {
        for sub in ["bin", "steamui", "package", "crashhandler"] {
            dirs.push(format!("{steam}\\{sub}"));
        }
    }
    if let Ok(pf86) = std::env::var("ProgramFiles(x86)") {
        dirs.push(format!("{pf86}\\Epic Games\\Launcher"));
        dirs.push(format!("{pf86}\\GOG Galaxy"));
    }
    dirs
}

// ------------------------------------------------------------------------- terminar

/// Encerra o processo `pid` **so se** ainda for o mesmo (tempo de criacao igual ao guardado):
/// um PID reaproveitado nunca e encerrado (SEC-15). `Ok(true)` = encerrou.
pub fn terminate_if_same(pid: u32, expected_creation: u64) -> Result<bool, AppError> {
    // SAFETY: handle fechado por `Owned`; `PROCESS_TERMINATE` so aqui, so a pedido do usuario.
    unsafe {
        let h = match OpenProcess(
            PROCESS_TERMINATE | PROCESS_QUERY_LIMITED_INFORMATION,
            false,
            pid,
        ) {
            Ok(h) => Owned(h),
            Err(_) => {
                return if GetLastError() == ERROR_ACCESS_DENIED {
                    Err(AppError::AccessDenied) // jogo elevado (UIPI)
                } else {
                    Ok(false) // ja saiu
                };
            }
        };
        if creation_time(h.0) != Some(expected_creation) {
            tracing::warn!(pid, "PID reaproveitado: nao sera encerrado");
            return Ok(false);
        }
        TerminateProcess(h.0, 1).map_err(|_| AppError::AccessDenied)?;
        Ok(true)
    }
}

fn post_close(pids: &[u32]) -> Result<(), AppError> {
    let windows = top_level_windows(pids);
    if windows.is_empty() {
        tracing::info!("pedido de fechamento: nenhuma janela de topo do jogo");
        return Ok(());
    }
    let mut delivered = 0usize;
    for w in &windows {
        // SAFETY: `PostMessageW` e assincrono e nao toca memoria do outro processo.
        if unsafe { PostMessageW(to_hwnd(*w), WM_CLOSE, WPARAM(0), LPARAM(0)) }.is_ok() {
            delivered += 1;
        }
    }
    if delivered == 0 {
        return Err(AppError::AccessDenied);
    }
    Ok(())
}

// --------------------------------------------------------------------------- a sonda

#[derive(Debug, Default)]
pub struct WinSessionProbe {
    store_dirs: Vec<String>,
}

impl WinSessionProbe {
    pub fn new() -> Self {
        Self {
            store_dirs: client_store_dirs(),
        }
    }

    /// Com pastas de cliente explicitas (testes).
    pub fn with_store_dirs(store_dirs: Vec<String>) -> Self {
        Self { store_dirs }
    }
}

enum Mode {
    Job(Arc<Job>, Arc<Mutex<Scanner>>),
    Scan(Arc<Mutex<Scanner>>),
}

pub struct WinWatch {
    mode: Mode,
    stop: Arc<Owned>,
    stopping: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

impl WinWatch {
    /// Conjunto de membros em dia. No modo Job rele a lista do Job antes (a mensagem da porta
    /// pode ainda nao ter sido tratada pela thread da sonda).
    fn members_fresh(&self) -> std::sync::MutexGuard<'_, Scanner> {
        let (scanner, job) = match &self.mode {
            Mode::Job(job, s) => (s, Some(job)),
            Mode::Scan(s) => (s, None),
        };
        let mut guard = scanner.lock().unwrap_or_else(PoisonError::into_inner);
        if let Some(job) = job {
            guard.scan_job(job);
        }
        guard
    }
}

impl SessionWatch for WinWatch {
    fn pids(&self) -> Vec<u32> {
        self.members_fresh().member_pids()
    }

    fn request_close(&self) -> Result<(), AppError> {
        // So as janelas dos membros do jogo (nunca Steam ou navegador que nasceram no Job).
        post_close(&self.pids())
    }

    fn force_close(&self) -> Result<(), AppError> {
        self.members_fresh().terminate_members()
    }
}

impl Drop for WinWatch {
    /// So sinaliza e solta a thread, **sem** `join`: quem larga a sonda (o caso de uso, com o
    /// `Mutex` da sessao preso, ou a propria thread da sonda no fim da sessao) nao pode esperar por
    /// uma thread que, para sair, talvez precise desse mesmo `Mutex` (aviso ao `ProbeSink`). A
    /// thread acorda na hora (porta do Job ou evento de parada) e fecha os proprios handles.
    fn drop(&mut self) {
        self.stopping.store(true, Ordering::Release);
        match &self.mode {
            Mode::Job(job, _) => job.wake(),
            Mode::Scan(_) => {
                // SAFETY: evento proprio, vivo ate o fim do `Drop`.
                unsafe {
                    let _ = SetEvent(self.stop.0);
                }
            }
        }
        drop(self.thread.take());
    }
}

/// O Job so avisa que algo mudou; quem decide e o `classify` sobre os PIDs do Job. O fim da sessao
/// e "nao sobrou membro do jogo", mesmo que a Steam ou o navegador sigam vivos no Job.
fn job_loop(
    job: &Job,
    scanner: &Mutex<Scanner>,
    generation: u64,
    sink: &dyn ProbeSink,
    stopping: &AtomicBool,
) {
    let mut present = false;
    // O primeiro processo do Job e o jogo que o Sofa acabou de iniciar. Se ele ja saiu quando a
    // lista e lida (jogo que abre e fecha em milissegundos), o inicio e o fim sao avisados em
    // seguida, como sempre foi, em vez de a sessao esperar 60 s por uma confirmacao que nao vem.
    let mut first_pending = true;
    loop {
        let message = job.wait_message();
        if matches!(message, JobMessage::Stop | JobMessage::Failed)
            || stopping.load(Ordering::Acquire)
        {
            break;
        }
        // O `Mutex` do scanner solta antes de avisar o sink (que pega o `Mutex` da sessao).
        let has_members = scanner
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .scan_job(job);
        if matches!(message, JobMessage::NewProcess(_)) && first_pending {
            first_pending = false;
            if !has_members && !present {
                sink.process_seen(generation);
                sink.processes_gone(generation);
                continue;
            }
        }
        if has_members && !present {
            present = true;
            sink.process_seen(generation);
        } else if !has_members && present {
            present = false;
            sink.processes_gone(generation);
        }
    }
}

fn scan_loop(
    scanner: &Mutex<Scanner>,
    cfg: ProbeConfig,
    steam_app_id: Option<u32>,
    generation: u64,
    sink: &dyn ProbeSink,
    stop: HANDLE,
    stopping: &AtomicBool,
) {
    let mut present = false;
    while !stopping.load(Ordering::Acquire) {
        let (has_members, handles) = {
            let mut s = scanner.lock().unwrap_or_else(PoisonError::into_inner);
            let has = s.scan();
            let handles: Vec<HANDLE> = s
                .members
                .values()
                .take(MAX_WAIT_HANDLES)
                .map(|m| m.handle.0)
                .collect();
            (has, handles)
        };
        let steam_alive = cfg.use_steam_running_app_id
            && steam_app_id.is_some_and(|id| steam_running_app_id() == id);
        let now_present = has_members || steam_alive;
        if now_present && !present {
            present = true;
            sink.process_seen(generation);
        } else if !now_present && present {
            present = false;
            sink.processes_gone(generation);
        }

        // Espera na parada e nos handles dos membros: a saida do ultimo membro acorda na hora.
        let interval_ms = if present {
            cfg.running_interval_ms
        } else {
            cfg.launching_interval_ms
        };
        let mut wait = Vec::with_capacity(handles.len() + 1);
        wait.push(stop);
        wait.extend(handles);
        // SAFETY: handles vivos (membros so saem do conjunto na varredura, nesta thread).
        unsafe {
            WaitForMultipleObjects(&wait, false, interval_ms);
        }
    }
}

impl SessionProbe for WinSessionProbe {
    fn watch(
        &self,
        plan: WatchPlan,
        generation: u64,
        sink: Arc<dyn ProbeSink>,
        config: ProbeConfig,
    ) -> Result<Box<dyn SessionWatch>, AppError> {
        // SAFETY: evento de reset manual, nao sinalizado; fechado por `Owned`.
        let stop = Arc::new(Owned(
            unsafe { CreateEventW(None, true, false, PCWSTR::null()) }
                .map_err(|_| AppError::Internal)?,
        ));
        let stopping = Arc::new(AtomicBool::new(false));

        if let Some(job) = plan.job {
            // Sem pasta de instalacao nao ha como dizer quem e membro: nao vigia (SEC-17).
            let Some(dir) = plan.scan_dir else {
                return Err(AppError::Internal);
            };
            let job = job.downcast::<Job>().map_err(|_| AppError::Internal)?;
            let job: Arc<Job> = Arc::new(*job);
            let scanner = Arc::new(Mutex::new(Scanner::new(dir, self.store_dirs.clone())));
            let (j, sc, st, sk) = (
                Arc::clone(&job),
                Arc::clone(&scanner),
                Arc::clone(&stopping),
                sink,
            );
            let thread = std::thread::Builder::new()
                .name("session-probe-job".into())
                .spawn(move || job_loop(&j, &sc, generation, sk.as_ref(), &st))
                .map_err(|_| AppError::Internal)?;
            return Ok(Box::new(WinWatch {
                mode: Mode::Job(job, scanner),
                stop,
                stopping,
                thread: Some(thread),
            }));
        }

        let Some(dir) = plan.scan_dir else {
            return Err(AppError::Internal); // nada para vigiar
        };
        let scanner = Arc::new(Mutex::new(Scanner::new(dir, self.store_dirs.clone())));
        let (sc, st, stop_ev) = (
            Arc::clone(&scanner),
            Arc::clone(&stopping),
            Arc::clone(&stop),
        );
        let app_id = plan.steam_app_id;
        let thread = std::thread::Builder::new()
            .name("session-probe-scan".into())
            .spawn(move || {
                scan_loop(
                    &sc,
                    config,
                    app_id,
                    generation,
                    sink.as_ref(),
                    stop_ev.0,
                    &st,
                )
            })
            .map_err(|_| AppError::Internal)?;
        Ok(Box::new(WinWatch {
            mode: Mode::Scan(scanner),
            stop,
            stopping,
            thread: Some(thread),
        }))
    }
}
