//! `SessionProbe` Win32 (BE-17, ADR-0005, SEC-15, SEC-16, SEC-17).
//!
//! Duas estrategias atras da mesma interface:
//! - **Job** (manual e GOG): o Job Object do `CreateProcessW` com completion port so **rastreia**
//!   (entrada e saida de processo chegam pela porta, sem polling nem CPU). Cada PID do Job passa
//!   pelo mesmo `classify` da varredura (SEC-17) e so os membros do jogo contam para o fim da
//!   sessao, para "Fechar" e para "Forcar", por handle e tempo de criacao. Nunca ha
//!   `TerminateJobObject` (SEC-BUG-01, ADR-0005 rev. 6). **Sem snapshot ToolHelp** (ARQ-BUG-14): a
//!   unica leitura e `JobObjectBasicProcessIdList`, feita pela thread da sonda a cada mensagem da
//!   porta; `pids`, `request_close` e `force_close` so usam os membros ja conhecidos.
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
use launcher_core::domain::session::{classify, GameScope, ProcessInfo, SessionContext};
use windows::core::PCWSTR;
use windows::Win32::Foundation::{
    CloseHandle, GetLastError, BOOLEAN, ERROR_ACCESS_DENIED, FILETIME, HANDLE,
    INVALID_HANDLE_VALUE, LPARAM, WAIT_OBJECT_0, WPARAM,
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
    OpenProcessToken, QueryFullProcessImageNameW, RegisterWaitForSingleObject, SetEvent,
    TerminateProcess, UnregisterWaitEx, WaitForMultipleObjects, WaitForSingleObject, INFINITE,
    PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION, PROCESS_SYNCHRONIZE, PROCESS_TERMINATE,
    WT_EXECUTEINWAITTHREAD, WT_EXECUTEONLYONCE,
};
use windows::Win32::UI::WindowsAndMessaging::{PostMessageW, WM_CLOSE};

use crate::job::{Job, JobMessage};
use crate::windows_enum::{hwnd as to_hwnd, top_level_windows};

/// Nao membros sao reavaliados a cada ~60 s (reuso de PID, ADR-0005).
const REVALIDATE_AFTER: Duration = Duration::from_secs(60);
const MAX_WAIT_HANDLES: usize = 63;
/// Espera periodica de reserva do modo Job para membro sem `ExitWait` (ARQ-BUG-17).
const FALLBACK_POLL_MS: u32 = 250;

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

/// Espera registrada no handle de um membro (modo Job, ARQ-BUG-15): quando o objeto do processo
/// **de fato** sinaliza, o sistema chama `member_exited`, que posta um pacote proprio na porta do
/// Job. Nenhum prazo fixo e nenhum `Mutex` preso: o fim de sessao nao depende de quanto o jogo
/// leva para desmontar o processo depois de o kernel avisar `EXIT_PROCESS`.
struct ExitWait {
    wait: HANDLE,
    /// Contexto da espera (`Box::into_raw`); so liberado depois de `UnregisterWaitEx` bloquear
    /// ate o fim de qualquer chamada em curso.
    ctx: *mut ExitWaitCtx,
}

struct ExitWaitCtx {
    job: Arc<Job>,
    pid: u32,
}

// SAFETY: `wait` e um identificador do sistema e `ctx` so e lido pela callback (que apenas posta
// na porta) e liberado no `Drop`, depois de a espera ser cancelada.
unsafe impl Send for ExitWait {}

/// Callback da thread de espera do sistema. So posta o pacote: sem `Mutex`, sem alocacao.
unsafe extern "system" fn member_exited(ctx: *mut std::ffi::c_void, _timed_out: BOOLEAN) {
    // SAFETY: `ctx` e o `ExitWaitCtx` de `ExitWait::register`, vivo ate o `UnregisterWaitEx`
    // do `Drop`, que espera esta chamada terminar.
    let ctx = unsafe { &*(ctx as *const ExitWaitCtx) };
    ctx.job.post_member_exited(ctx.pid);
}

impl ExitWait {
    /// Registra a espera no handle (que precisa de `SYNCHRONIZE`). Se o handle ja sinalizou, o
    /// sistema chama a callback na hora. `None` se o registro falhar.
    fn register(handle: HANDLE, job: Arc<Job>, pid: u32) -> Option<ExitWait> {
        let ctx = Box::into_raw(Box::new(ExitWaitCtx { job, pid }));
        let mut wait = HANDLE::default();
        // SAFETY: `ctx` vive ate o `Drop`; a callback executa na propria thread de espera.
        let r = unsafe {
            RegisterWaitForSingleObject(
                &mut wait,
                handle,
                Some(member_exited),
                Some(ctx as *const std::ffi::c_void),
                INFINITE,
                WT_EXECUTEONLYONCE | WT_EXECUTEINWAITTHREAD,
            )
        };
        if r.is_err() {
            // SAFETY: nenhuma espera foi registrada: o contexto ainda e so nosso.
            drop(unsafe { Box::from_raw(ctx) });
            return None;
        }
        Some(ExitWait { wait, ctx })
    }
}

impl Drop for ExitWait {
    fn drop(&mut self) {
        // SAFETY: `INVALID_HANDLE_VALUE` bloqueia ate a callback em curso terminar (ela so posta
        // na porta, nunca espera por nos); depois disso o contexto nao e mais lido.
        unsafe {
            let _ = UnregisterWaitEx(self.wait, INVALID_HANDLE_VALUE);
            drop(Box::from_raw(self.ctx));
        }
    }
}

struct Member {
    /// Declarado antes de `handle`: a espera sai antes de o handle fechar.
    _exit_wait: Option<ExitWait>,
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
    /// Modo Job: o Job que recebe o pacote `MemberExited` de cada membro (ARQ-BUG-15).
    notify: Option<Arc<Job>>,
    /// So testes: faz o registro da espera falhar.
    #[cfg(test)]
    fail_register: bool,
    /// Quantos snapshots ToolHelp esta varredura tirou (testes: o modo Job nunca tira, ARQ-BUG-14).
    snapshots: usize,
}

impl Scanner {
    fn new(scope: GameScope, store_dirs: Vec<String>, notify: Option<Arc<Job>>) -> Self {
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
                scope,
                store_dirs,
            },
            own_sid,
            notify,
            #[cfg(test)]
            fail_register: false,
            snapshots: 0,
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

    /// Uma varredura do sistema (Steam, Epic e jogo elevado). Devolve se ha membros.
    fn scan(&mut self) -> bool {
        let procs = Self::snapshot();
        self.snapshots += 1;
        self.ctx.own_child_pids = self.own_webview_processes(&procs);
        let live: HashSet<u32> = procs.iter().map(|(p, _, _)| *p).collect();

        for &(pid, parent, _) in &procs {
            if pid == 0 || pid == 4 || pid == self.ctx.own_pid || self.members.contains_key(&pid) {
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
            self.evaluate(pid, Some(parent), None);
        }

        self.cache.retain(|p, _| live.contains(p));
        self.members.retain(|_, m| !is_signaled(m.handle.0));
        !self.members.is_empty()
    }

    /// Modo Job: **sem snapshot do sistema** (ARQ-BUG-14). Le so a lista do Job, avalia cada PID
    /// ainda desconhecido (`IsProcessInJob` no handle aberto e o mesmo `classify`) e tira dos
    /// membros quem ja saiu. Nao ha cache de nao membros: a lista do Job e curta e um PID
    /// reaproveitado dentro do Job nunca fica de fora por causa de uma avaliacao antiga.
    /// `own_child_pids` fica vazio: o WebView2 do Sofa nunca entra no Job do jogo.
    fn scan_job(&mut self, job: &Job) -> bool {
        // Primeiro tira quem ja saiu (ARQ-BUG-17): um PID reaproveitado por um processo novo do
        // mesmo Job e avaliado nesta passada, nao pulado por causa do membro velho de mesmo PID.
        self.members.retain(|_, m| !is_signaled(m.handle.0));
        for pid in job.pids() {
            if pid == 0 || pid == 4 || pid == self.ctx.own_pid || self.members.contains_key(&pid) {
                continue;
            }
            self.evaluate(pid, None, Some(job));
        }
        !self.members.is_empty()
    }

    /// Modo Job: ha membro sem espera no handle (o registro falhou)? Se sim, a `job_loop` acorda
    /// de tempos em tempos para reavaliar (ARQ-BUG-17).
    fn needs_poll(&self) -> bool {
        self.notify.is_some() && self.members.values().any(|m| m._exit_wait.is_none())
    }

    /// Abre o processo, confere o dono e a sessao e, se o `classify` aceitar, guarda o handle e o
    /// tempo de criacao como membro. Com `job`, confirma antes que o handle e mesmo do Job.
    fn evaluate(&mut self, pid: u32, parent: Option<u32>, job: Option<&Job>) {
        // SAFETY: handle devolvido e fechado por `Owned` (ou aqui, se nao for membro).
        let handle = match unsafe {
            OpenProcess(
                PROCESS_QUERY_LIMITED_INFORMATION | PROCESS_SYNCHRONIZE,
                false,
                pid,
            )
        } {
            Ok(h) => Owned(h),
            Err(_) => return, // protegido ou de sistema: nunca entra (SEC-17)
        };
        if job.is_some_and(|j| !j.contains(handle.0)) {
            return; // PID reaproveitado: nao e o processo que o Job listou
        }
        let Some(image) = image_path(handle.0) else {
            return;
        };
        let same_user = match (&self.own_sid, user_sid(handle.0)) {
            (Some(mine), Some(theirs)) => sids_equal(mine, &theirs),
            // Token ilegivel com os direitos minimos: nao arrisca (SEC-17).
            _ => false,
        };
        let info = ProcessInfo {
            pid,
            parent_pid: parent,
            image_path: image,
            session_id: session_of(pid).unwrap_or(u32::MAX),
            same_user,
            openable: true,
        };
        if classify(&info, &self.ctx).is_ok() {
            if let Some(creation) = creation_time(handle.0) {
                // Modo Job: a espera avisa pela porta quando o processo sai de fato.
                #[cfg(test)]
                let fail = self.fail_register;
                #[cfg(not(test))]
                let fail = false;
                let exit_wait = self.notify.as_ref().and_then(|job| {
                    let w = if fail {
                        None
                    } else {
                        ExitWait::register(handle.0, Arc::clone(job), pid)
                    };
                    if w.is_none() {
                        tracing::warn!(
                            pid,
                            "RegisterWaitForSingleObject falhou: espera periodica de reserva para este membro"
                        );
                    }
                    w
                });
                self.members.insert(
                    pid,
                    Member {
                        _exit_wait: exit_wait,
                        handle,
                        creation,
                    },
                );
            }
        }
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

/// Onde a imagem de um processo do jogo pode estar: a pasta (SEC-16 ja conferida pelo caso de uso)
/// ou, se ela reprovou, so o exe exato (SEC-BUG-04). `None` se o plano nao traz nenhum dos dois.
fn scope_of(scan_dir: Option<String>, exact_exe: Option<String>) -> Option<GameScope> {
    scan_dir
        .map(GameScope::Dir)
        .or_else(|| exact_exe.map(GameScope::ExactExe))
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
    /// Membros ja conhecidos, sem varrer nada. A thread da sonda os mantem em dia (mensagens do
    /// Job no modo Job, varredura periodica no outro): quem chama (o caso de uso, com o `Mutex`
    /// da sessao preso) nao paga snapshot nem leitura do Job (ARQ-BUG-14).
    fn members(&self) -> std::sync::MutexGuard<'_, Scanner> {
        let scanner = match &self.mode {
            Mode::Job(_, s) | Mode::Scan(s) => s,
        };
        scanner.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Quantos snapshots ToolHelp esta sonda tirou. So para teste: o modo Job nunca tira.
    pub fn snapshots_taken(&self) -> usize {
        self.members().snapshots
    }
}

impl SessionWatch for WinWatch {
    fn pids(&self) -> Vec<u32> {
        self.members().member_pids()
    }

    fn request_close(&self) -> Result<(), AppError> {
        // So as janelas dos membros do jogo (nunca Steam ou navegador que nasceram no Job).
        post_close(&self.pids())
    }

    fn force_close(&self) -> Result<(), AppError> {
        self.members().terminate_members()
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
        // Membro sem espera no handle (registro falhou): acorda a cada 250 ms para reavaliar.
        let poll = scanner
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .needs_poll();
        let message = if poll {
            job.wait_message_timeout(FALLBACK_POLL_MS)
        } else {
            job.wait_message()
        };
        if matches!(message, JobMessage::Stop | JobMessage::Failed)
            || stopping.load(Ordering::Acquire)
        {
            break;
        }
        // O `Mutex` do scanner solta antes de avisar o sink (que pega o `Mutex` da sessao).
        let has_members = {
            let mut s = scanner.lock().unwrap_or_else(PoisonError::into_inner);
            // Sem espera aqui: se o membro que a porta disse que saiu ainda nao sinalizou, a
            // espera registrada no handle posta `MemberExited` quando ele sinalizar (ARQ-BUG-15).
            s.scan_job(job)
        };
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
        Ok(Box::new(self.watch_win(plan, generation, sink, config)?))
    }
}

impl WinSessionProbe {
    /// Igual a `SessionProbe::watch`, mas devolve o tipo concreto (testes leem `snapshots_taken`).
    pub fn watch_win(
        &self,
        plan: WatchPlan,
        generation: u64,
        sink: Arc<dyn ProbeSink>,
        config: ProbeConfig,
    ) -> Result<WinWatch, AppError> {
        // SAFETY: evento de reset manual, nao sinalizado; fechado por `Owned`.
        let stop = Arc::new(Owned(
            unsafe { CreateEventW(None, true, false, PCWSTR::null()) }
                .map_err(|_| AppError::Internal)?,
        ));
        let stopping = Arc::new(AtomicBool::new(false));

        if let Some(job) = plan.job {
            // Sem pasta nem exe nao ha como dizer quem e membro: nao vigia (SEC-17).
            let Some(scope) = scope_of(plan.scan_dir, plan.exact_exe) else {
                return Err(AppError::Internal);
            };
            let job = job.downcast::<Job>().map_err(|_| AppError::Internal)?;
            let job: Arc<Job> = Arc::new(*job);
            let scanner = Arc::new(Mutex::new(Scanner::new(
                scope,
                self.store_dirs.clone(),
                Some(Arc::clone(&job)),
            )));
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
            return Ok(WinWatch {
                mode: Mode::Job(job, scanner),
                stop,
                stopping,
                thread: Some(thread),
            });
        }

        let Some(scope) = scope_of(plan.scan_dir, plan.exact_exe) else {
            return Err(AppError::Internal); // nada para vigiar
        };
        let scanner = Arc::new(Mutex::new(Scanner::new(
            scope,
            self.store_dirs.clone(),
            None,
        )));
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
        Ok(WinWatch {
            mode: Mode::Scan(scanner),
            stop,
            stopping,
            thread: Some(thread),
        })
    }
}

#[cfg(test)]
mod tests {
    //! ARQ-BUG-15: o fim da sessao no modo Job nao depende de prazo fixo. O kernel avisa
    //! `EXIT_PROCESS` antes de o objeto do processo sinalizar; aqui a ordem e controlada (mensagem
    //! injetada com o membro ainda vivo) e confere-se a espera registrada no handle.

    use super::*;
    use crate::job::MSG_EXIT_PROCESS;
    use std::os::windows::io::AsRawHandle;
    use std::path::{Path, PathBuf};
    use std::process::{Child, Command, Stdio};
    use std::sync::Condvar;

    struct Dir(PathBuf);

    impl Dir {
        fn new(tag: &str) -> Self {
            let p = std::env::temp_dir().join(format!("sofa-probe-{}-{tag}", std::process::id()));
            std::fs::create_dir_all(&p).unwrap();
            Dir(p)
        }
        /// Copia o `ping.exe` do sistema para `rel`: processo que vive `count - 1` segundos.
        fn ping(&self, rel: &str) -> PathBuf {
            let dst = self.0.join(rel);
            std::fs::create_dir_all(dst.parent().unwrap()).unwrap();
            let sys = std::env::var("SystemRoot").unwrap_or_else(|_| r"C:\Windows".into());
            std::fs::copy(Path::new(&sys).join(r"System32\ping.exe"), &dst).unwrap();
            dst
        }
    }

    impl Drop for Dir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    fn spawn_in_job(exe: &Path, count: u32, job: &Job) -> Child {
        let child = Command::new(exe)
            .args(["-n", &count.to_string(), "127.0.0.1"])
            .stdout(Stdio::null())
            .spawn()
            .unwrap();
        job.assign(HANDLE(child.as_raw_handle())).expect("assign");
        child
    }

    #[derive(Default)]
    struct TestSink {
        events: Mutex<Vec<&'static str>>,
        cv: Condvar,
    }

    impl ProbeSink for TestSink {
        fn process_seen(&self, _g: u64) {
            self.events.lock().unwrap().push("seen");
            self.cv.notify_all();
        }
        fn processes_gone(&self, _g: u64) {
            self.events.lock().unwrap().push("gone");
            self.cv.notify_all();
        }
    }

    impl TestSink {
        fn wait(&self, name: &str, timeout: Duration) -> bool {
            let deadline = Instant::now() + timeout;
            let mut g = self.events.lock().unwrap();
            while !g.contains(&name) {
                let Some(left) = deadline.checked_duration_since(Instant::now()) else {
                    return false;
                };
                g = self.cv.wait_timeout(g, left).unwrap().0;
            }
            true
        }
    }

    /// ARQ-BUG-17 (a): um membro velho (ja sinalizado) com o mesmo PID de um processo novo do Job
    /// sai ANTES da avaliacao, e o processo novo vira membro na mesma passada.
    #[test]
    fn pid_reaproveitado_e_avaliado_na_mesma_passada() {
        let dir = Dir::new("reuso");
        let exe = dir.ping(r"Jogo\game.exe");
        let job = Arc::new(Job::new().unwrap());
        let mut scanner = Scanner::new(
            GameScope::Dir(dir.0.join("Jogo").to_string_lossy().into_owned()),
            vec![],
            Some(Arc::clone(&job)),
        );
        // processo curto que ja saiu: seu handle fica sinalizado
        let mut old = Command::new(&exe)
            .args(["-n", "1", "127.0.0.1"])
            .stdout(Stdio::null())
            .spawn()
            .unwrap();
        old.wait().unwrap();
        // processo vivo no Job; o "membro velho" e guardado com o PID dele (PID reaproveitado)
        let mut live = spawn_in_job(&exe, 4, &job);
        let live_pid = live.id();
        // SAFETY: handle proprio do processo que ja saiu, fechado pelo `Owned` do membro.
        let stale = unsafe {
            OpenProcess(
                PROCESS_QUERY_LIMITED_INFORMATION | PROCESS_SYNCHRONIZE,
                false,
                old.id(),
            )
        };
        if let Ok(h) = stale {
            assert!(is_signaled(h), "o processo antigo ja saiu");
            scanner.members.insert(
                live_pid,
                Member {
                    _exit_wait: None,
                    handle: Owned(h),
                    creation: 0,
                },
            );
            assert!(
                scanner.scan_job(&job),
                "o PID reaproveitado e avaliado na hora, nao pulado"
            );
            assert!(!is_signaled(scanner.members[&live_pid].handle.0));
        } else {
            eprintln!("PID do processo antigo ja reciclado: cenario nao montavel");
        }
        let _ = live.kill();
        let _ = live.wait();
    }

    /// ARQ-BUG-17 (b): sem `ExitWait` (registro falhou), o scanner pede a espera periodica de
    /// reserva e a `job_loop` encerra a sessao sem depender do pacote do handle; o `Job` devolve
    /// `Timeout` quando nada chega.
    #[test]
    fn registro_da_espera_que_falha_cai_na_espera_periodica() {
        let dir = Dir::new("reserva");
        let exe = dir.ping(r"Jogo\game.exe");
        let job = Arc::new(Job::new().unwrap());
        let mut sc = Scanner::new(
            GameScope::Dir(dir.0.join("Jogo").to_string_lossy().into_owned()),
            vec![],
            Some(Arc::clone(&job)),
        );
        sc.fail_register = true;
        let scanner = Arc::new(Mutex::new(sc));
        let sink = Arc::new(TestSink::default());
        let stopping = Arc::new(AtomicBool::new(false));
        let mut member = spawn_in_job(&exe, 3, &job); // ~2 s
        let thread = {
            let (j, sc, sk, st) = (
                Arc::clone(&job),
                Arc::clone(&scanner),
                Arc::clone(&sink),
                Arc::clone(&stopping),
            );
            std::thread::spawn(move || job_loop(&j, &sc, 1, sk.as_ref(), &st))
        };
        assert!(sink.wait("seen", Duration::from_secs(5)), "inicio");
        assert!(scanner.lock().unwrap().needs_poll(), "membro sem espera");
        assert!(sink.wait("gone", Duration::from_secs(10)), "fim");
        stopping.store(true, Ordering::Release);
        job.wake();
        thread.join().unwrap();
        let _ = member.wait();
        let idle = Job::new().unwrap();
        assert_eq!(idle.wait_message_timeout(50), JobMessage::Timeout);
    }

    /// A espera registrada no handle do membro posta `MemberExited(pid)` quando o processo sai de
    /// fato, e o pacote so chega depois de o handle sinalizar.
    #[test]
    fn espera_no_handle_posta_o_pacote_proprio_quando_o_membro_sai_de_fato() {
        let dir = Dir::new("espera");
        let exe = dir.ping(r"Jogo\game.exe");
        let job = Arc::new(Job::new().unwrap());
        let mut scanner = Scanner::new(
            GameScope::Dir(dir.0.join("Jogo").to_string_lossy().into_owned()),
            vec![],
            Some(Arc::clone(&job)),
        );
        let mut child = spawn_in_job(&exe, 3, &job); // ~2 s
        let pid = child.id();
        assert!(scanner.scan_job(&job), "o ping da pasta do jogo e membro");

        // Rede de seguranca: se o pacote nao vier, a espera acaba por `Stop`.
        let guard = {
            let j = Arc::clone(&job);
            std::thread::spawn(move || {
                std::thread::sleep(Duration::from_secs(20));
                j.wake();
            })
        };
        let exited = loop {
            match job.wait_message() {
                JobMessage::MemberExited(p) => break p,
                JobMessage::Stop | JobMessage::Failed => panic!("o pacote do membro nao veio"),
                _ => {}
            }
        };
        assert_eq!(exited, pid);
        assert!(
            is_signaled(scanner.members[&pid].handle.0),
            "o pacote so e postado depois de o handle sinalizar"
        );
        assert!(
            !scanner.scan_job(&job),
            "o membro saiu de fato: sem membros"
        );
        let _ = child.wait();
        drop(guard);
    }

    /// Chega `EXIT_PROCESS` com o membro ainda vivo (o handle sinaliza so depois) e ha um
    /// acompanhante vivo no Job. A thread da sonda nao espera nada com o `Mutex` do scanner preso
    /// (`pids` e `force_close` o pegam com o `Mutex` da sessao preso) e a sessao termina quando o
    /// handle sinaliza.
    #[test]
    fn exit_process_antecipado_nao_prende_o_mutex_e_a_sessao_termina_quando_o_handle_sinaliza() {
        let dir = Dir::new("antecipado");
        let exe = dir.ping(r"Jogo\game.exe");
        let outro = dir.ping(r"Fora\acompanhante.exe");
        let job = Arc::new(Job::new().unwrap());
        let scanner = Arc::new(Mutex::new(Scanner::new(
            GameScope::Dir(dir.0.join("Jogo").to_string_lossy().into_owned()),
            vec![],
            Some(Arc::clone(&job)),
        )));
        let sink = Arc::new(TestSink::default());
        let stopping = Arc::new(AtomicBool::new(false));
        let mut member = spawn_in_job(&exe, 5, &job); // ~4 s
        let mut companion = spawn_in_job(&outro, 60, &job); // vive bem mais que o teste
        let thread = {
            let (j, sc, sk, st) = (
                Arc::clone(&job),
                Arc::clone(&scanner),
                Arc::clone(&sink),
                Arc::clone(&stopping),
            );
            std::thread::spawn(move || job_loop(&j, &sc, 1, sk.as_ref(), &st))
        };
        assert!(sink.wait("seen", Duration::from_secs(5)), "inicio");

        job.inject_kernel_message(MSG_EXIT_PROCESS, member.id());
        std::thread::sleep(Duration::from_millis(100)); // a thread da sonda ja pegou a mensagem
        let t = Instant::now();
        drop(scanner.lock().unwrap());
        let took = t.elapsed();
        assert!(
            took < Duration::from_millis(150),
            "o Mutex do scanner nao pode ficar preso esperando o handle: {took:?}"
        );
        assert!(
            !sink.wait("gone", Duration::from_millis(500)),
            "o membro segue vivo: a sessao nao acabou"
        );

        assert!(
            sink.wait("gone", Duration::from_secs(10)),
            "o handle sinalizou: a sessao termina, com o acompanhante vivo"
        );
        assert!(companion.try_wait().unwrap().is_none(), "acompanhante vivo");

        stopping.store(true, Ordering::Release);
        job.wake();
        thread.join().unwrap();
        let _ = companion.kill();
        let _ = companion.wait();
        let _ = member.wait();
    }
}
