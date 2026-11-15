//! Modo `sofa.exe --purge-user-data` (BE-41, SEC-25, SEC-31): apaga a pasta de dados do usuario
//! **sem elevacao** e **por handle**, para a remocao nao seguir junction trocada no meio do
//! caminho (a corrida de checar e depois apagar pelo caminho).
//!
//! Garantias:
//! - o alvo e calculado aqui (`FOLDERID_LocalAppData` + identificador fixo no build); nenhum
//!   caminho vem de argumento, ambiente ou registro;
//! - recusa rodar elevado (`TokenElevation`), sem apagar nada;
//! - SEC-25: o alvo, aberto como handle, nao e ponto de reanalise, esta exatamente um nivel abaixo
//!   de `%LOCALAPPDATA%` (pelo caminho final do handle) e contem `library.json` ou `settings.json`;
//! - cada entrada e aberta com `FILE_FLAG_OPEN_REPARSE_POINT | FILE_FLAG_BACKUP_SEMANTICS` e os
//!   atributos vem **do handle** (`FileAttributeTagInfo`): ponto de reanalise e apagado pelo
//!   handle, sem descer nele; arquivo e apagado pelo handle; pasta e percorrida pelo handle;
//! - o caminho final de cada filho aberto tem que estar dentro do caminho final do pai; se nao
//!   estiver, aquela entrada e abandonada e conta como falha (o invariante vem do handle);
//! - pastas ficam abertas sem `FILE_SHARE_DELETE` enquanto sao percorridas: ninguem as renomeia
//!   nem as troca por junction nesse intervalo;
//! - exclusao por `FileDispositionInfoEx(DELETE | POSIX_SEMANTICS | IGNORE_READONLY)` (Win10
//!   1709+) com fallback para `FileDispositionInfo`;
//! - nao apaga nada fora do alvo; jogos nunca sao tocados (SEC-29).
//!
//! Codigos de saida (`PurgeExit`): 0 ok, 1 parcial (log em `%TEMP%\sofa-purge.log`), 2 alvo
//! invalido ou argumento extra, 3 elevado, 4 Sofa aberto (mutex da instancia unica; nada apagado).
//! Dono unico da remocao recursiva (INV-09).

use std::ffi::OsString;
use std::os::windows::ffi::{OsStrExt, OsStringExt};
use std::path::Path;

use launcher_core::domain::data_dir_policy::{
    is_direct_child, plan, target_dir, PurgeExit, PurgePlan, TargetFacts, APP_IDENTIFIER,
    MARKER_FILES,
};
use windows::core::PCWSTR;
use windows::Win32::Foundation::{
    CloseHandle, GetLastError, ERROR_NO_MORE_FILES, HANDLE, INVALID_HANDLE_VALUE,
};
use windows::Win32::Storage::FileSystem::{
    CreateFileW, FileAttributeTagInfo, FileBasicInfo, FileDispositionInfo, FileDispositionInfoEx,
    FileIdBothDirectoryInfo, FileIdBothDirectoryRestartInfo, GetFileInformationByHandleEx,
    GetFinalPathNameByHandleW, SetFileInformationByHandle, DELETE, FILE_ATTRIBUTE_DIRECTORY,
    FILE_ATTRIBUTE_REPARSE_POINT, FILE_ATTRIBUTE_TAG_INFO, FILE_BASIC_INFO,
    FILE_DISPOSITION_FLAG_DELETE, FILE_DISPOSITION_FLAG_IGNORE_READONLY_ATTRIBUTE,
    FILE_DISPOSITION_FLAG_POSIX_SEMANTICS, FILE_DISPOSITION_INFO, FILE_DISPOSITION_INFO_EX,
    FILE_DISPOSITION_INFO_EX_FLAGS, FILE_FLAG_BACKUP_SEMANTICS, FILE_FLAG_OPEN_REPARSE_POINT,
    FILE_ID_BOTH_DIR_INFO, FILE_LIST_DIRECTORY, FILE_NAME_NORMALIZED, FILE_READ_ATTRIBUTES,
    FILE_SHARE_DELETE, FILE_SHARE_MODE, FILE_SHARE_READ, FILE_SHARE_WRITE, FILE_WRITE_ATTRIBUTES,
    GETFINALPATHNAMEBYHANDLE_FLAGS, OPEN_EXISTING, VOLUME_NAME_DOS,
};

use crate::hardening::is_process_elevated;
use crate::path_canon::{canonicalize, known, strip_verbatim};

const SYNCHRONIZE: u32 = 0x0010_0000;
/// Profundidade maxima de pastas (a pasta de dados real tem 3 ou 4 niveis).
const MAX_DEPTH: usize = 64;

/// O que a remocao fez.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct PurgeReport {
    pub files: u64,
    pub dirs: u64,
    /// Pontos de reanalise apagados **sem descer**.
    pub links: u64,
    /// Caminhos (relativos ao alvo) que nao foram apagados, com o motivo.
    pub failures: Vec<String>,
}

fn wide(path: &str) -> Vec<u16> {
    OsString::from(path).encode_wide().chain(Some(0)).collect()
}

/// Handle fechado no `Drop`.
struct Handle(HANDLE);

impl Drop for Handle {
    fn drop(&mut self) {
        // SAFETY: o handle foi aberto por `open` e so e fechado aqui.
        unsafe {
            let _ = CloseHandle(self.0);
        }
    }
}

/// `CreateFileW` com `FILE_FLAG_OPEN_REPARSE_POINT | FILE_FLAG_BACKUP_SEMANTICS`: abre o proprio
/// ponto de reanalise (nao o destino) e permite abrir pastas.
fn open(path: &str, access: u32, share: FILE_SHARE_MODE) -> Result<Handle, String> {
    let p = wide(path);
    // SAFETY: `p` termina em nulo e vive durante a chamada.
    let h = unsafe {
        CreateFileW(
            PCWSTR(p.as_ptr()),
            access,
            share,
            None,
            OPEN_EXISTING,
            FILE_FLAG_OPEN_REPARSE_POINT | FILE_FLAG_BACKUP_SEMANTICS,
            None,
        )
    };
    match h {
        Ok(h) if h != INVALID_HANDLE_VALUE => Ok(Handle(h)),
        Ok(_) => Err("handle invalido".into()),
        Err(e) => Err(format!("abrir: {:#x}", e.code().0)),
    }
}

/// Atributos lidos **do handle**.
fn attributes(h: &Handle) -> Result<u32, String> {
    let mut info = FILE_ATTRIBUTE_TAG_INFO::default();
    // SAFETY: buffer do tamanho exato da estrutura pedida.
    unsafe {
        GetFileInformationByHandleEx(
            h.0,
            FileAttributeTagInfo,
            std::ptr::from_mut(&mut info).cast(),
            std::mem::size_of::<FILE_ATTRIBUTE_TAG_INFO>() as u32,
        )
    }
    .map_err(|e| format!("atributos: {:#x}", e.code().0))?;
    Ok(info.FileAttributes)
}

/// Caminho final do handle (sem o prefixo `\\?\`).
fn final_path(h: &Handle) -> Result<String, String> {
    let mut buf = vec![0u16; 1024];
    // SAFETY: buffer proprio com o tamanho informado.
    let n = unsafe {
        GetFinalPathNameByHandleW(
            h.0,
            &mut buf,
            GETFINALPATHNAMEBYHANDLE_FLAGS(FILE_NAME_NORMALIZED.0 | VOLUME_NAME_DOS.0),
        )
    };
    if n == 0 {
        return Err("caminho final indisponivel".into());
    }
    if n as usize > buf.len() {
        buf.resize(n as usize + 1, 0);
        // SAFETY: como acima, com o buffer maior.
        let n = unsafe {
            GetFinalPathNameByHandleW(
                h.0,
                &mut buf,
                GETFINALPATHNAMEBYHANDLE_FLAGS(FILE_NAME_NORMALIZED.0 | VOLUME_NAME_DOS.0),
            )
        };
        if n == 0 || n as usize > buf.len() {
            return Err("caminho final indisponivel".into());
        }
        buf.truncate(n as usize);
    } else {
        buf.truncate(n as usize);
    }
    Ok(strip_verbatim(&OsString::from_wide(&buf).to_string_lossy()))
}

/// Apaga a entrada **pelo handle** (a exclusao vale ao fechar, ou na hora com POSIX).
fn delete_by_handle(h: &Handle) -> Result<(), String> {
    let ex = FILE_DISPOSITION_INFO_EX {
        Flags: FILE_DISPOSITION_INFO_EX_FLAGS(
            FILE_DISPOSITION_FLAG_DELETE.0
                | FILE_DISPOSITION_FLAG_POSIX_SEMANTICS.0
                | FILE_DISPOSITION_FLAG_IGNORE_READONLY_ATTRIBUTE.0,
        ),
    };
    // SAFETY: a estrutura vive durante a chamada e tem o tamanho informado.
    let first = unsafe {
        SetFileInformationByHandle(
            h.0,
            FileDispositionInfoEx,
            std::ptr::from_ref(&ex).cast(),
            std::mem::size_of::<FILE_DISPOSITION_INFO_EX>() as u32,
        )
    };
    if first.is_ok() {
        return Ok(());
    }
    // Windows anterior ao 1709 (ou sistema de arquivos sem POSIX): exclusao comum, tirando o
    // atributo somente leitura antes.
    let basic = FILE_BASIC_INFO {
        FileAttributes: 0x80, // FILE_ATTRIBUTE_NORMAL
        ..Default::default()
    };
    // SAFETY: como acima. Falha aqui (sem `FILE_WRITE_ATTRIBUTES`) e ignorada: a exclusao
    // seguinte diz se deu certo.
    unsafe {
        let _ = SetFileInformationByHandle(
            h.0,
            FileBasicInfo,
            std::ptr::from_ref(&basic).cast(),
            std::mem::size_of::<FILE_BASIC_INFO>() as u32,
        );
    }
    let plain = FILE_DISPOSITION_INFO {
        DeleteFile: true.into(),
    };
    // SAFETY: como acima.
    unsafe {
        SetFileInformationByHandle(
            h.0,
            FileDispositionInfo,
            std::ptr::from_ref(&plain).cast(),
            std::mem::size_of::<FILE_DISPOSITION_INFO>() as u32,
        )
    }
    .map_err(|e| format!("excluir: {:#x}", e.code().0))
}

/// Nomes e atributos das entradas da pasta aberta em `dir` (sem `.` e `..`).
fn list(dir: &Handle) -> Result<Vec<(String, u32)>, String> {
    let mut out = Vec::new();
    let mut buf = vec![0u8; 64 * 1024];
    let mut class = FileIdBothDirectoryRestartInfo;
    loop {
        // SAFETY: buffer proprio; a API preenche entradas encadeadas por `NextEntryOffset`.
        let r = unsafe {
            GetFileInformationByHandleEx(dir.0, class, buf.as_mut_ptr().cast(), buf.len() as u32)
        };
        class = FileIdBothDirectoryInfo;
        if r.is_err() {
            // SAFETY: le o ultimo erro da propria thread.
            if unsafe { GetLastError() } == ERROR_NO_MORE_FILES {
                return Ok(out);
            }
            return Err("listar a pasta falhou".into());
        }
        let mut offset = 0usize;
        loop {
            // SAFETY: `offset` anda por entradas validas dentro do buffer preenchido pela API; o
            // nome tem `FileNameLength` bytes a partir do campo `FileName`.
            let (name, attrs, next) = unsafe {
                let entry = buf.as_ptr().add(offset).cast::<FILE_ID_BOTH_DIR_INFO>();
                let len = (*entry).FileNameLength as usize / 2;
                let name_ptr = std::ptr::addr_of!((*entry).FileName).cast::<u16>();
                let name = OsString::from_wide(std::slice::from_raw_parts(name_ptr, len));
                (
                    name.to_string_lossy().into_owned(),
                    (*entry).FileAttributes,
                    (*entry).NextEntryOffset as usize,
                )
            };
            if name != "." && name != ".." {
                out.push((name, attrs));
            }
            if next == 0 {
                break;
            }
            offset += next;
        }
    }
}

fn is_inside(parent_final: &str, child_final: &str) -> bool {
    is_direct_child(parent_final, child_final)
}

/// Apaga o conteudo de `dir` (aberta em `handle`, sem `FILE_SHARE_DELETE`) e depois a propria
/// pasta, tudo pelos handles.
fn purge_dir(handle: &Handle, path: &str, rel: &str, depth: usize, report: &mut PurgeReport) {
    if depth > MAX_DEPTH {
        report
            .failures
            .push(format!("{rel}: profundidade acima do limite"));
        return;
    }
    let parent_final = match final_path(handle) {
        Ok(p) => p,
        Err(e) => {
            report.failures.push(format!("{rel}: {e}"));
            return;
        }
    };
    let entries = match list(handle) {
        Ok(e) => e,
        Err(e) => {
            report.failures.push(format!("{rel}: {e}"));
            return;
        }
    };
    let mut all_gone = true;
    for (name, listed_attrs) in entries {
        let child_path = format!("{path}\\{name}");
        let child_rel = if rel.is_empty() {
            name.clone()
        } else {
            format!("{rel}\\{name}")
        };
        let want_dir = listed_attrs & FILE_ATTRIBUTE_DIRECTORY.0 != 0
            && listed_attrs & FILE_ATTRIBUTE_REPARSE_POINT.0 == 0;
        // Pasta comum: lista, sem compartilhar DELETE. Arquivo ou ponto de reanalise: so o
        // necessario para ler atributos e apagar.
        let (access, share) = if want_dir {
            (
                DELETE.0
                    | FILE_LIST_DIRECTORY.0
                    | FILE_READ_ATTRIBUTES.0
                    | FILE_WRITE_ATTRIBUTES.0
                    | SYNCHRONIZE,
                FILE_SHARE_READ | FILE_SHARE_WRITE,
            )
        } else {
            (
                DELETE.0 | FILE_READ_ATTRIBUTES.0 | FILE_WRITE_ATTRIBUTES.0 | SYNCHRONIZE,
                FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
            )
        };
        let child = match open(&child_path, access, share) {
            Ok(c) => c,
            Err(e) => {
                report.failures.push(format!("{child_rel}: {e}"));
                all_gone = false;
                continue;
            }
        };
        // O invariante: tudo abaixo vem do handle aberto, nao do caminho.
        let attrs = match attributes(&child) {
            Ok(a) => a,
            Err(e) => {
                report.failures.push(format!("{child_rel}: {e}"));
                all_gone = false;
                continue;
            }
        };
        match final_path(&child) {
            Ok(f) if is_inside(&parent_final, &f) => {}
            _ => {
                report.failures.push(format!(
                    "{child_rel}: caminho final fora da pasta pai, abandonado"
                ));
                all_gone = false;
                continue;
            }
        }
        let is_link = attrs & FILE_ATTRIBUTE_REPARSE_POINT.0 != 0;
        let is_dir = attrs & FILE_ATTRIBUTE_DIRECTORY.0 != 0;
        if is_link {
            // Ponto de reanalise: apaga o proprio link, nunca desce nele.
            match delete_by_handle(&child) {
                Ok(()) => report.links += 1,
                Err(e) => {
                    report.failures.push(format!("{child_rel}: {e}"));
                    all_gone = false;
                }
            }
        } else if is_dir {
            if !want_dir {
                // Trocou de arquivo para pasta entre a listagem e a abertura: sem acesso de
                // listagem, nao arrisca; fica para outra rodada.
                report
                    .failures
                    .push(format!("{child_rel}: mudou de tipo durante a remocao"));
                all_gone = false;
                continue;
            }
            let before = report.failures.len();
            purge_dir(&child, &child_path, &child_rel, depth + 1, report);
            if report.failures.len() != before {
                all_gone = false;
            }
        } else if want_dir {
            report
                .failures
                .push(format!("{child_rel}: mudou de tipo durante a remocao"));
            all_gone = false;
        } else {
            match delete_by_handle(&child) {
                Ok(()) => report.files += 1,
                Err(e) => {
                    report.failures.push(format!("{child_rel}: {e}"));
                    all_gone = false;
                }
            }
        }
    }
    if all_gone {
        match delete_by_handle(handle) {
            Ok(()) => report.dirs += 1,
            Err(e) => report.failures.push(format!(
                "{}: {e}",
                if rel.is_empty() { "(alvo)" } else { rel }
            )),
        }
    }
}

/// Apaga `root` e tudo abaixo, por handle. O chamador ja validou o alvo (`plan`).
pub fn purge_tree(root: &Path) -> PurgeReport {
    let mut report = PurgeReport::default();
    let path = root.display().to_string();
    let handle = match open(
        &path,
        DELETE.0
            | FILE_LIST_DIRECTORY.0
            | FILE_READ_ATTRIBUTES.0
            | FILE_WRITE_ATTRIBUTES.0
            | SYNCHRONIZE,
        FILE_SHARE_READ | FILE_SHARE_WRITE,
    ) {
        Ok(h) => h,
        Err(e) => {
            report.failures.push(format!("(alvo): {e}"));
            return report;
        }
    };
    match attributes(&handle) {
        Ok(a) if a & FILE_ATTRIBUTE_REPARSE_POINT.0 != 0 => {
            // O alvo virou link: apaga so o link.
            match delete_by_handle(&handle) {
                Ok(()) => report.links += 1,
                Err(e) => report.failures.push(format!("(alvo): {e}")),
            }
            return report;
        }
        Ok(a) if a & FILE_ATTRIBUTE_DIRECTORY.0 == 0 => {
            report.failures.push("(alvo): nao e uma pasta".into());
            return report;
        }
        Ok(_) => {}
        Err(e) => {
            report.failures.push(format!("(alvo): {e}"));
            return report;
        }
    }
    purge_dir(&handle, &path, "", 0, &mut report);
    report
}

/// Fatos do alvo para `plan` (SEC-25), lidos pelo handle aberto sem seguir link.
fn inspect(target: &str, local_app_data: &str) -> TargetFacts {
    let handle = match open(
        target,
        FILE_READ_ATTRIBUTES.0 | SYNCHRONIZE,
        FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
    ) {
        Ok(h) => h,
        Err(_) => {
            return TargetFacts {
                exists: Path::new(target).symlink_metadata().is_ok(),
                is_reparse_point: false,
                one_level_below_local_app_data: false,
                has_marker: false,
            }
        }
    };
    let attrs = attributes(&handle).unwrap_or(FILE_ATTRIBUTE_REPARSE_POINT.0);
    let one_level = final_path(&handle).is_ok_and(|f| {
        let lad = canonicalize(local_app_data)
            .map(|c| strip_verbatim(&c))
            .unwrap_or_else(|_| local_app_data.to_owned());
        is_direct_child(&lad, &f)
    });
    let is_dir = attrs & FILE_ATTRIBUTE_DIRECTORY.0 != 0;
    let has_marker = is_dir
        && MARKER_FILES.iter().any(|m| {
            Path::new(target)
                .join(m)
                .symlink_metadata()
                .is_ok_and(|md| md.is_file())
        });
    TargetFacts {
        exists: true,
        // Um alvo que nao e pasta tambem nao serve: tratado como invalido.
        is_reparse_point: attrs & FILE_ATTRIBUTE_REPARSE_POINT.0 != 0 || !is_dir,
        one_level_below_local_app_data: one_level,
        has_marker,
    }
}

/// `%TEMP%\sofa-purge.log` com o que ficou (so em remocao parcial ou recusa).
fn write_log(lines: &[String]) {
    let path = std::env::temp_dir().join("sofa-purge.log");
    let _ = std::fs::write(path, lines.join("\r\n"));
}

/// `true` se existe o mutex da instancia unica do Sofa (`<identificador>-sim`, criado pelo
/// plugin `tauri-plugin-single-instance`): ha um Sofa aberto nesta sessao (SEC-BUG-03). So abre
/// para `SYNCHRONIZE`; nunca cria nem segura o mutex.
pub fn app_is_open(identifier: &str) -> bool {
    use windows::Win32::System::Threading::{OpenMutexW, SYNCHRONIZATION_SYNCHRONIZE};
    let name: Vec<u16> = format!("{identifier}-sim\0").encode_utf16().collect();
    // SAFETY: `name` termina em NUL e vive ate o fim da chamada; o handle e fechado aqui.
    unsafe {
        match OpenMutexW(SYNCHRONIZATION_SYNCHRONIZE, false, PCWSTR(name.as_ptr())) {
            Ok(h) => {
                let _ = CloseHandle(h);
                true
            }
            Err(_) => false,
        }
    }
}

/// O nucleo testavel: valida e apaga `<local_app_data>\<identifier>`.
pub fn run_in(local_app_data: &str, identifier: &str, elevated: bool) -> PurgeExit {
    run_in_with(local_app_data, identifier, elevated, false)
}

/// Como `run_in`, com o Sofa aberto informado de fora (`app_is_open` no processo real).
pub fn run_in_with(
    local_app_data: &str,
    identifier: &str,
    elevated: bool,
    app_open: bool,
) -> PurgeExit {
    if elevated {
        write_log(&["recusado: o Sofa esta elevado (SEC-31)".to_owned()]);
        return PurgeExit::Elevated;
    }
    if app_open {
        // Com o Sofa aberto a remocao ficaria parcial (EBWebView, log) e o Sofa regravaria os
        // arquivos: nao apaga nada (SEC-BUG-03).
        write_log(&["recusado: o Sofa esta aberto (mutex da instancia unica)".to_owned()]);
        return PurgeExit::AppOpen;
    }
    let Some(target) = target_dir(local_app_data, identifier) else {
        write_log(&["recusado: alvo invalido".to_owned()]);
        return PurgeExit::InvalidTarget;
    };
    let facts = inspect(&target, local_app_data);
    match plan(&facts) {
        Ok(PurgePlan::NothingToDo) => PurgeExit::Done,
        Err(refusal) => {
            write_log(&[format!(
                "recusado: alvo nao passou nas travas de SEC-25 ({facts:?})"
            )]);
            refusal
        }
        Ok(PurgePlan::Delete) => {
            let report = purge_tree(Path::new(&target));
            if report.failures.is_empty() {
                PurgeExit::Done
            } else {
                let mut lines = vec![format!(
                    "parcial: {} arquivos, {} pastas, {} links removidos; {} falhas",
                    report.files,
                    report.dirs,
                    report.links,
                    report.failures.len()
                )];
                lines.extend(report.failures);
                write_log(&lines);
                PurgeExit::Partial
            }
        }
    }
}

/// Entrada do modo de remocao: so o `main` chama, antes de criar Tauri, WebView2 e instancia
/// unica. `refused` = `--purge-user-data` veio com outro argumento: nada e apagado.
pub fn run_main(refused: bool) -> i32 {
    if refused {
        write_log(&["recusado: argumento extra junto de --purge-user-data (SEC-31)".to_owned()]);
        return PurgeExit::InvalidTarget.code();
    }
    let Some(lad) = known::local_app_data() else {
        return PurgeExit::InvalidTarget.code();
    };
    run_in_with(
        &lad,
        APP_IDENTIFIER,
        is_process_elevated(),
        app_is_open(APP_IDENTIFIER),
    )
    .code()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::sync::atomic::{AtomicU32, Ordering};

    struct Temp(std::path::PathBuf);
    impl Temp {
        fn new(tag: &str) -> Self {
            static N: AtomicU32 = AtomicU32::new(0);
            let p = std::env::temp_dir().join(format!(
                "sofa-purge-{tag}-{}-{}",
                std::process::id(),
                N.fetch_add(1, Ordering::SeqCst)
            ));
            fs::create_dir_all(&p).unwrap();
            Self(p)
        }
    }
    impl Drop for Temp {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn arvore_simples_e_removida_por_inteiro() {
        let t = Temp::new("simples");
        let root = t.0.join("dados");
        fs::create_dir_all(root.join("cache").join("covers")).unwrap();
        fs::create_dir_all(root.join("logs")).unwrap();
        for f in [
            "library.json",
            "settings.json",
            "cache/covers/a.jpg",
            "logs/x.log",
        ] {
            fs::write(root.join(f), b"x").unwrap();
        }
        let r = purge_tree(&root);
        assert!(r.failures.is_empty(), "{r:?}");
        assert_eq!(r.files, 4);
        assert_eq!(r.dirs, 4);
        assert!(!root.exists());
    }

    #[test]
    fn arquivo_somente_leitura_tambem_sai() {
        let t = Temp::new("ro");
        let root = t.0.join("dados");
        fs::create_dir_all(&root).unwrap();
        let f = root.join("settings.json");
        fs::write(&f, b"x").unwrap();
        let mut perm = fs::metadata(&f).unwrap().permissions();
        perm.set_readonly(true);
        fs::set_permissions(&f, perm).unwrap();
        let r = purge_tree(&root);
        assert!(r.failures.is_empty(), "{r:?}");
        assert!(!root.exists());
    }

    #[test]
    fn elevado_recusa_e_nao_apaga() {
        let t = Temp::new("elev");
        let id = "app.sofa.launcher";
        let root = t.0.join(id);
        fs::create_dir_all(&root).unwrap();
        fs::write(root.join("library.json"), b"{}").unwrap();
        let code = run_in(&t.0.display().to_string(), id, true);
        assert_eq!(code, PurgeExit::Elevated);
        assert!(root.join("library.json").exists());
    }

    #[test]
    fn sofa_aberto_recusa_com_codigo_4_e_nao_apaga_nada() {
        let t = Temp::new("aberto");
        let id = "app.sofa.launcher";
        let root = t.0.join(id);
        fs::create_dir_all(root.join("cache")).unwrap();
        fs::write(root.join("library.json"), b"{}").unwrap();
        fs::write(root.join("cache").join("x.bin"), b"x").unwrap();
        let code = run_in_with(&t.0.display().to_string(), id, false, true);
        assert_eq!(code, PurgeExit::AppOpen);
        assert_eq!(code.code(), 4);
        assert!(root.join("library.json").exists());
        assert!(root.join("cache").join("x.bin").exists());
    }

    #[test]
    fn app_is_open_enxerga_o_mutex_da_instancia_unica() {
        use windows::Win32::System::Threading::CreateMutexW;
        let id = format!("app.sofa.teste-{}", std::process::id());
        assert!(!app_is_open(&id), "sem mutex: fechado");
        let name: Vec<u16> = format!("{id}-sim\0").encode_utf16().collect();
        // SAFETY: nome terminado em NUL; handle fechado ao fim do teste.
        let h = unsafe { CreateMutexW(None, true, PCWSTR(name.as_ptr())) }.unwrap();
        assert!(app_is_open(&id), "mutex existe: aberto");
        // SAFETY: handle proprio.
        unsafe {
            let _ = CloseHandle(h);
        }
        assert!(!app_is_open(&id), "mutex fechado: de novo fechado");
    }

    #[test]
    fn pasta_sem_marcador_nao_e_apagada() {
        let t = Temp::new("semmarca");
        let id = "app.sofa.launcher";
        let root = t.0.join(id);
        fs::create_dir_all(&root).unwrap();
        fs::write(root.join("qualquer-coisa.txt"), b"x").unwrap();
        let code = run_in(&t.0.display().to_string(), id, false);
        assert_eq!(code, PurgeExit::InvalidTarget);
        assert!(root.join("qualquer-coisa.txt").exists());
    }

    #[test]
    fn pasta_inexistente_e_sucesso_e_identificador_ruim_e_recusado() {
        let t = Temp::new("ausente");
        assert_eq!(
            run_in(&t.0.display().to_string(), "app.sofa.launcher", false),
            PurgeExit::Done
        );
        assert_eq!(
            run_in(&t.0.display().to_string(), r"..\fora", false),
            PurgeExit::InvalidTarget
        );
    }

    #[test]
    fn marcador_valido_apaga_tudo_e_devolve_zero() {
        let t = Temp::new("ok");
        let id = "app.sofa.launcher";
        let root = t.0.join(id);
        fs::create_dir_all(root.join("EBWebView").join("Default")).unwrap();
        fs::write(root.join("settings.json"), b"{}").unwrap();
        fs::write(root.join("EBWebView/Default/Cookies"), b"x").unwrap();
        let code = run_in(&t.0.display().to_string(), id, false);
        assert_eq!(code, PurgeExit::Done);
        assert!(!root.exists());
        assert!(t.0.exists(), "so o alvo, nunca o pai");
    }
}
