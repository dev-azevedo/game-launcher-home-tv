//! Repositorios JSON de `library.json` e `settings.json` (ADR-0004, SEC-18, SEC-29, INV-09).
//!
//! Unico dono desses arquivos: nenhum caminho vem de fora, so a pasta de dados recebida na
//! construcao e nomes de arquivo constantes. Escrita atomica (temporario na mesma pasta +
//! `MoveFileExW` com `REPLACE_EXISTING | WRITE_THROUGH`). Leitura com limite de 4 MB; arquivo
//! invalido (ou grande demais) e renomeado para `*.corrupt-<data>.json` e recriado com os padroes.
//! Nenhuma RN-xx aqui: formato, limites e migracao vivem no `launcher-core`.

use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Write};
use std::os::windows::ffi::OsStrExt;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use launcher_core::application::ports::{
    Clock, LibraryRepository, Loaded, Recovery, RecoveryReason, ScanCache, SettingsRepository,
    StoreError,
};
use launcher_core::domain::library::{
    parse_library, parse_scan, serialize_library, serialize_scan, DetectedGame, StoredLibrary,
};
use launcher_core::domain::settings::{parse_settings, serialize_settings, Settings};
use windows::core::PCWSTR;
use windows::Win32::Storage::FileSystem::{
    MoveFileExW, MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH,
};

/// Limite de leitura e de gravacao de cada arquivo (ADR-0004).
pub const MAX_FILE_BYTES: usize = 4 * 1024 * 1024;
/// Backups `*.corrupt-*` mantidos por arquivo; os mais antigos saem.
const MAX_BACKUPS: usize = 5;

const LIBRARY_FILE: &str = "library.json";
const SETTINGS_FILE: &str = "settings.json";
const SCAN_FILE: &str = "scan.json";

// ---------------------------------------------------------------------------------------------
// E/S de arquivo

enum Raw {
    Missing,
    Bytes(Vec<u8>),
    TooLarge,
}

/// Le no maximo `MAX_FILE_BYTES + 1` bytes: nunca carrega arquivo gigante na memoria, mesmo que
/// ele cresca depois do `metadata`.
fn read_limited(path: &Path) -> io::Result<Raw> {
    let file = match File::open(path) {
        Ok(f) => f,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(Raw::Missing),
        Err(e) => return Err(e),
    };
    let mut buf = Vec::new();
    file.take(MAX_FILE_BYTES as u64 + 1).read_to_end(&mut buf)?;
    if buf.len() > MAX_FILE_BYTES {
        Ok(Raw::TooLarge)
    } else {
        Ok(Raw::Bytes(buf))
    }
}

fn wide(path: &Path) -> Vec<u16> {
    path.as_os_str().encode_wide().chain(Some(0)).collect()
}

/// Troca atomica de `from` para `to` (`ReplaceFileW`-like via `MoveFileExW`).
fn replace_file(from: &Path, to: &Path) -> io::Result<()> {
    let (from_w, to_w) = (wide(from), wide(to));
    // SAFETY: os dois buffers terminam em NUL e vivem ate o fim da chamada.
    unsafe {
        MoveFileExW(
            PCWSTR(from_w.as_ptr()),
            PCWSTR(to_w.as_ptr()),
            MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
        )
        .map_err(|e| io::Error::from_raw_os_error(e.code().0 & 0xFFFF))
    }
}

/// Nome do temporario de `name`, na mesma pasta (necessario para a troca ser atomica).
fn tmp_path(dir: &Path, name: &str) -> PathBuf {
    dir.join(format!("{name}.tmp"))
}

/// Grava `bytes` em `dir\name` sem nunca deixar o arquivo anterior pela metade: escreve e sincroniza
/// o temporario inteiro e so entao troca. Falha em qualquer ponto deixa o arquivo anterior intacto
/// e apaga o temporario.
pub fn atomic_write(dir: &Path, name: &str, bytes: &[u8]) -> Result<(), StoreError> {
    if bytes.len() > MAX_FILE_BYTES {
        return Err(StoreError::TooLarge(MAX_FILE_BYTES));
    }
    fs::create_dir_all(dir)?;
    let tmp = tmp_path(dir, name);
    let result = (|| -> io::Result<()> {
        let mut f = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&tmp)?;
        f.write_all(bytes)?;
        f.sync_all()?;
        drop(f);
        replace_file(&tmp, &dir.join(name))
    })();
    if let Err(e) = result {
        let _ = fs::remove_file(&tmp);
        return Err(StoreError::Io(e));
    }
    Ok(())
}

// ---------------------------------------------------------------------------------------------
// Backup de arquivo invalido

/// `YYYYMMDD-HHMMSS` (UTC) a partir de ms Unix. Algoritmo de dias civis de H. Hinnant.
fn stamp(unix_ms: i64) -> String {
    let secs = unix_ms.div_euclid(1000);
    let days = secs.div_euclid(86_400);
    let rem = secs.rem_euclid(86_400);
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let mut year = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    if month <= 2 {
        year += 1;
    }
    format!(
        "{year:04}{month:02}{day:02}-{:02}{:02}{:02}",
        rem / 3600,
        rem % 3600 / 60,
        rem % 60
    )
}

fn backup_stem(name: &str) -> &str {
    name.strip_suffix(".json").unwrap_or(name)
}

/// Renomeia `dir\name` para `<stem>.corrupt-<data>[-n].json` e poda os mais antigos.
fn move_to_backup(dir: &Path, name: &str, clock: &dyn Clock) -> io::Result<String> {
    let stem = backup_stem(name);
    let base = format!("{stem}.corrupt-{}", stamp(clock.unix_ms()));
    let mut candidate = format!("{base}.json");
    let mut n = 1;
    while dir.join(&candidate).exists() {
        candidate = format!("{base}-{n}.json");
        n += 1;
    }
    fs::rename(dir.join(name), dir.join(&candidate))?;
    prune_backups(dir, stem);
    Ok(candidate)
}

fn prune_backups(dir: &Path, stem: &str) {
    let prefix = format!("{stem}.corrupt-");
    let Ok(read) = fs::read_dir(dir) else { return };
    let mut backups: Vec<String> = read
        .filter_map(Result::ok)
        .filter_map(|e| e.file_name().into_string().ok())
        .filter(|n| n.starts_with(&prefix) && n.ends_with(".json"))
        .collect();
    backups.sort(); // o carimbo ordena por data
    while backups.len() > MAX_BACKUPS {
        let oldest = backups.remove(0);
        let _ = fs::remove_file(dir.join(oldest));
    }
}

// ---------------------------------------------------------------------------------------------
// Carga generica

enum Parsed<T> {
    Ok(T, Vec<String>),
    Invalid,
}

fn load_file<T>(
    dir: &Path,
    name: &str,
    clock: &dyn Clock,
    default: impl Fn() -> T,
    serialize: impl Fn(&T) -> Vec<u8>,
    parse: impl Fn(&[u8]) -> Parsed<T>,
) -> Loaded<T> {
    let path = dir.join(name);
    let reason = match read_limited(&path) {
        Ok(Raw::Missing) => {
            return Loaded {
                value: default(),
                recovery: None,
                warnings: Vec::new(),
            }
        }
        Ok(Raw::Bytes(bytes)) => match parse(&bytes) {
            Parsed::Ok(value, warnings) => {
                return Loaded {
                    value,
                    recovery: None,
                    warnings,
                }
            }
            Parsed::Invalid => RecoveryReason::Invalid,
        },
        Ok(Raw::TooLarge) => RecoveryReason::TooLarge,
        Err(e) => {
            // Nao deu para ler (permissao, trava): usa o padrao em memoria e NAO mexe no arquivo.
            tracing::error!(file = name, error = %e, "falha ao ler arquivo de dados; usando padroes");
            return Loaded {
                value: default(),
                recovery: None,
                warnings: Vec::new(),
            };
        }
    };

    let backup_name = match move_to_backup(dir, name, clock) {
        Ok(n) => Some(n),
        Err(e) => {
            tracing::error!(file = name, error = %e, "falha ao mover arquivo invalido para backup");
            None
        }
    };
    let value = default();
    if backup_name.is_some() {
        if let Err(e) = atomic_write(dir, name, &serialize(&value)) {
            tracing::error!(file = name, error = %e, "falha ao recriar arquivo com os padroes");
        }
    }
    tracing::warn!(file = name, ?reason, backup = ?backup_name, "arquivo de dados invalido; recriado com os padroes");
    Loaded {
        value,
        recovery: Some(Recovery {
            reason,
            backup_name,
        }),
        warnings: Vec::new(),
    }
}

// ---------------------------------------------------------------------------------------------
// Repositorios

#[derive(Clone)]
pub struct JsonLibraryRepository {
    dir: PathBuf,
    clock: Arc<dyn Clock>,
}

impl JsonLibraryRepository {
    /// `dir` e a pasta de dados do usuario (`app_local_data_dir()`).
    pub fn new(dir: impl Into<PathBuf>, clock: Arc<dyn Clock>) -> Self {
        Self {
            dir: dir.into(),
            clock,
        }
    }
}

impl LibraryRepository for JsonLibraryRepository {
    fn load(&self) -> Loaded<StoredLibrary> {
        load_file(
            &self.dir,
            LIBRARY_FILE,
            self.clock.as_ref(),
            StoredLibrary::default,
            serialize_library,
            |bytes| match parse_library(bytes) {
                Ok(lib) => Parsed::Ok(lib, Vec::new()),
                Err(e) => {
                    tracing::warn!(error = %e, "library.json recusado");
                    Parsed::Invalid
                }
            },
        )
    }

    fn save(&self, library: &StoredLibrary) -> Result<(), StoreError> {
        atomic_write(&self.dir, LIBRARY_FILE, &serialize_library(library))
    }
}

/// `cache\scan.json` (BE-40): resultado da ultima varredura, descartavel. Arquivo ausente,
/// grande demais ou invalido volta vazio (a proxima varredura o reescreve); nunca vira backup.
#[derive(Clone)]
pub struct JsonScanCache {
    dir: PathBuf,
}

impl JsonScanCache {
    /// `dir` e a pasta de dados do usuario; o arquivo fica em `dir\cache\scan.json`.
    pub fn new(dir: impl Into<PathBuf>) -> Self {
        Self { dir: dir.into() }
    }

    fn cache_dir(&self) -> PathBuf {
        self.dir.join("cache")
    }
}

impl ScanCache for JsonScanCache {
    fn load(&self) -> Vec<DetectedGame> {
        match read_limited(&self.cache_dir().join(SCAN_FILE)) {
            Ok(Raw::Bytes(bytes)) => parse_scan(&bytes).unwrap_or_else(|error| {
                tracing::warn!(%error, "scan.json ignorado");
                Vec::new()
            }),
            Ok(Raw::Missing) => Vec::new(),
            Ok(Raw::TooLarge) => {
                tracing::warn!("scan.json acima de 4 MB ignorado");
                Vec::new()
            }
            Err(error) => {
                tracing::warn!(%error, "scan.json ilegivel");
                Vec::new()
            }
        }
    }

    fn save(&self, games: &[DetectedGame]) {
        if let Err(error) = atomic_write(&self.cache_dir(), SCAN_FILE, &serialize_scan(games)) {
            tracing::warn!(%error, "scan.json nao gravado");
        }
    }
}

#[derive(Clone)]
pub struct JsonSettingsRepository {
    dir: PathBuf,
    clock: Arc<dyn Clock>,
}

impl JsonSettingsRepository {
    pub fn new(dir: impl Into<PathBuf>, clock: Arc<dyn Clock>) -> Self {
        Self {
            dir: dir.into(),
            clock,
        }
    }
}

impl SettingsRepository for JsonSettingsRepository {
    fn load(&self) -> Loaded<Settings> {
        load_file(
            &self.dir,
            SETTINGS_FILE,
            self.clock.as_ref(),
            Settings::default,
            serialize_settings,
            |bytes| match parse_settings(bytes) {
                Ok(s) => {
                    let warnings = s.advanced().warnings;
                    for w in &warnings {
                        tracing::warn!(aviso = %w, "valor avancado invalido; usando o padrao");
                    }
                    Parsed::Ok(s, warnings)
                }
                Err(e) => {
                    tracing::warn!(error = %e, "settings.json recusado");
                    Parsed::Invalid
                }
            },
        )
    }

    fn save(&self, settings: &Settings) -> Result<(), StoreError> {
        atomic_write(&self.dir, SETTINGS_FILE, &serialize_settings(settings))
    }
}

// ---------------------------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use launcher_core::application::ports::ManualClock;
    use launcher_core::domain::library::{GameId, ManualGameRecord, OverlayEntry};
    use serde_json::json;
    use std::os::windows::fs::OpenOptionsExt;
    use std::sync::atomic::{AtomicU32, Ordering};

    /// Pasta temporaria unica, apagada no fim. Ficam so os arquivos que o teste espera.
    struct TempDir(PathBuf);

    impl TempDir {
        fn new() -> Self {
            static N: AtomicU32 = AtomicU32::new(0);
            let p = std::env::temp_dir().join(format!(
                "sofa-persist-{}-{}",
                std::process::id(),
                N.fetch_add(1, Ordering::SeqCst)
            ));
            fs::create_dir_all(&p).unwrap();
            Self(p)
        }
        fn path(&self) -> &Path {
            &self.0
        }
        fn names(&self) -> Vec<String> {
            let mut v: Vec<String> = fs::read_dir(&self.0)
                .unwrap()
                .map(|e| e.unwrap().file_name().into_string().unwrap())
                .collect();
            v.sort();
            v
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn clock() -> Arc<ManualClock> {
        let c = Arc::new(ManualClock::new());
        c.set_unix_ms(1_794_610_123_000); // 2026-11-13 ...
        c
    }

    fn sample_library() -> StoredLibrary {
        let mut lib = StoredLibrary::default();
        lib.manual_games.push(ManualGameRecord {
            id: GameId::parse("manual:9f3a1c2b7d4e5a60").unwrap(),
            name: "Meu Jogo".into(),
            path: "C:\\Jogos\\a.exe".into(),
        });
        lib.overlay.insert(
            GameId::parse("steam:620").unwrap(),
            OverlayEntry {
                custom_name: Some("Portal".into()),
                hidden: true,
                last_played_at: Some(7),
            },
        );
        lib
    }

    #[test]
    fn carimbo_de_data_confere_com_datas_conhecidas() {
        assert_eq!(stamp(0), "19700101-000000");
        assert_eq!(stamp(951_782_400_000 + 3_723_000), "20000229-010203"); // ano bissexto
        assert_eq!(stamp(1_794_610_123_000), "20261113-224843");
        assert_eq!(stamp(-1000), "19691231-235959");
    }

    #[test]
    fn arquivo_ausente_da_padrao_sem_criar_nada() {
        let d = TempDir::new();
        let repo = JsonLibraryRepository::new(d.path(), clock());
        let loaded = repo.load();
        assert_eq!(loaded.value, StoredLibrary::default());
        assert!(loaded.recovery.is_none());
        assert!(d.names().is_empty());
    }

    #[test]
    fn grava_e_le_de_volta_e_so_existe_o_arquivo_dono() {
        let d = TempDir::new();
        let repo = JsonLibraryRepository::new(d.path(), clock());
        repo.save(&sample_library()).unwrap();
        assert_eq!(repo.load().value, sample_library());
        // INV-09: so o arquivo esperado na pasta de dados; nem `.tmp` sobra.
        assert_eq!(d.names(), vec!["library.json".to_owned()]);
    }

    #[test]
    fn gravacao_interrompida_nao_corrompe_o_arquivo_anterior() {
        let d = TempDir::new();
        let repo = JsonLibraryRepository::new(d.path(), clock());
        repo.save(&sample_library()).unwrap();

        // Queda de energia no meio da escrita: sobra um temporario truncado ao lado.
        fs::write(
            d.path().join("library.json.tmp"),
            b"{ \"schemaVersion\": 1, \"manualGa",
        )
        .unwrap();

        let loaded = repo.load();
        assert_eq!(
            loaded.value,
            sample_library(),
            "o arquivo anterior continua valido"
        );
        assert!(loaded.recovery.is_none());

        // A proxima gravacao reaproveita o nome do temporario e termina limpa.
        let mut novo = sample_library();
        novo.overlay.clear();
        repo.save(&novo).unwrap();
        assert_eq!(repo.load().value, novo);
        assert_eq!(d.names(), vec!["library.json".to_owned()]);
    }

    #[test]
    fn scan_cache_ida_e_volta_e_arquivo_invalido_e_ignorado() {
        use launcher_core::domain::library::{GameId, LaunchSpec};
        let t = TempDir::new();
        let cache = JsonScanCache::new(t.path());
        assert!(cache.load().is_empty()); // ausente
        let games = vec![DetectedGame {
            id: GameId::parse("steam:620").unwrap(),
            name: "Portal 2".into(),
            install_dir: Some(r"C:\Jogos\Portal 2".into()),
            launch: LaunchSpec::SteamUri { app_id: 620 },
            cover_source: None,
            icon_source: None,
        }];
        cache.save(&games);
        assert_eq!(cache.load(), games);
        // So `cache\scan.json`: nada de temporario nem de backup.
        let names: Vec<String> = fs::read_dir(t.path().join("cache"))
            .unwrap()
            .map(|e| e.unwrap().file_name().into_string().unwrap())
            .collect();
        assert_eq!(names, ["scan.json"]);
        // Adulterado e grande demais: volta vazio, sem apagar nem mover o arquivo.
        let file = t.path().join(r"cache\scan.json");
        fs::write(&file, b"{ nao e json").unwrap();
        assert!(cache.load().is_empty());
        fs::write(&file, vec![b' '; MAX_FILE_BYTES + 10]).unwrap();
        assert!(cache.load().is_empty());
        assert!(file.exists());
    }

    #[test]
    fn troca_recusada_pelo_windows_deixa_o_anterior_intacto() {
        let d = TempDir::new();
        let repo = JsonLibraryRepository::new(d.path(), clock());
        repo.save(&sample_library()).unwrap();
        let before = fs::read(d.path().join("library.json")).unwrap();

        // Outro processo segura o arquivo aberto sem FILE_SHARE_DELETE: a troca nao pode acontecer.
        let held = OpenOptions::new()
            .read(true)
            .share_mode(1 | 2) // READ | WRITE, sem DELETE
            .open(d.path().join("library.json"))
            .unwrap();
        let mut novo = sample_library();
        novo.overlay.clear();
        let err = repo.save(&novo);
        assert!(err.is_err(), "a troca devia falhar com o destino travado");
        drop(held);

        assert_eq!(fs::read(d.path().join("library.json")).unwrap(), before);
        assert_eq!(
            d.names(),
            vec!["library.json".to_owned()],
            "temporario removido"
        );
    }

    #[test]
    fn campo_desconhecido_leva_o_arquivo_para_backup_e_recria() {
        let d = TempDir::new();
        let ruim = json!({ "schemaVersion": 1, "manualGames": [
            { "id": "manual:9f3a1c2b7d4e5a60", "name": "x", "path": "C:\\a.exe", "args": "-evil" } ] })
        .to_string();
        fs::write(d.path().join("library.json"), &ruim).unwrap();

        let repo = JsonLibraryRepository::new(d.path(), clock());
        let loaded = repo.load();
        assert_eq!(loaded.value, StoredLibrary::default());
        let rec = loaded.recovery.expect("recuperacao");
        assert_eq!(rec.reason, RecoveryReason::Invalid);
        assert_eq!(
            rec.backup_name.as_deref(),
            Some("library.corrupt-20261113-224843.json")
        );

        // O backup tem o conteudo original; o arquivo novo e valido e vazio.
        assert_eq!(
            fs::read_to_string(d.path().join("library.corrupt-20261113-224843.json")).unwrap(),
            ruim
        );
        assert_eq!(repo.load().value, StoredLibrary::default());
        assert!(repo.load().recovery.is_none());
        assert_eq!(
            d.names(),
            vec![
                "library.corrupt-20261113-224843.json".to_owned(),
                "library.json".to_owned()
            ]
        );
    }

    #[test]
    fn arquivo_acima_de_4_mb_e_recusado_sem_ser_lido_inteiro() {
        let d = TempDir::new();
        let path = d.path().join("library.json");
        {
            let mut f = File::create(&path).unwrap();
            f.write_all(b"{\"schemaVersion\":1,\"x\":\"").unwrap();
            f.write_all(&vec![b'a'; MAX_FILE_BYTES]).unwrap();
            f.write_all(b"\"}").unwrap();
        }
        let repo = JsonLibraryRepository::new(d.path(), clock());
        let loaded = repo.load();
        assert_eq!(loaded.value, StoredLibrary::default());
        assert_eq!(loaded.recovery.unwrap().reason, RecoveryReason::TooLarge);
        assert!(d.names().iter().any(|n| n.starts_with("library.corrupt-")));
        // Exatamente 4 MB ainda e aceito como tamanho (o conteudo e que decide).
        fs::write(&path, vec![b' '; MAX_FILE_BYTES]).unwrap();
        assert_eq!(
            repo.load().recovery.unwrap().reason,
            RecoveryReason::Invalid
        );
    }

    #[test]
    fn gravar_acima_do_limite_e_recusado() {
        let d = TempDir::new();
        let err =
            atomic_write(d.path(), "library.json", &vec![b'a'; MAX_FILE_BYTES + 1]).unwrap_err();
        assert!(matches!(err, StoreError::TooLarge(_)));
        assert!(d.names().is_empty());
    }

    #[test]
    fn backups_antigos_sao_podados() {
        let d = TempDir::new();
        let c = clock();
        let repo = JsonLibraryRepository::new(d.path(), c.clone());
        for _ in 0..8 {
            fs::write(d.path().join("library.json"), b"lixo").unwrap();
            let _ = repo.load();
            c.advance_ms(1000);
        }
        let backups: Vec<_> = d
            .names()
            .into_iter()
            .filter(|n| n.contains(".corrupt-"))
            .collect();
        assert_eq!(backups.len(), MAX_BACKUPS);
    }

    #[test]
    fn mesmo_segundo_nao_sobrescreve_backup() {
        let d = TempDir::new();
        let repo = JsonLibraryRepository::new(d.path(), clock());
        for _ in 0..2 {
            fs::write(d.path().join("library.json"), b"lixo").unwrap();
            let _ = repo.load();
        }
        let backups: Vec<_> = d
            .names()
            .into_iter()
            .filter(|n| n.contains(".corrupt-"))
            .collect();
        assert_eq!(backups.len(), 2);
    }

    // ---- settings ----

    #[test]
    fn settings_ida_e_volta_e_preserva_advanced_cru() {
        let d = TempDir::new();
        let advanced =
            json!({ "foreground": { "step2": false, "verifyDelayMs": 9999 }, "meu": [1, 2] });
        fs::write(
            d.path().join("settings.json"),
            json!({ "schemaVersion": 1, "monitorId": "M1", "advanced": advanced }).to_string(),
        )
        .unwrap();

        let repo = JsonSettingsRepository::new(d.path(), clock());
        let loaded = repo.load();
        assert!(loaded.recovery.is_none());
        assert!(
            !loaded.warnings.is_empty(),
            "verifyDelayMs fora da faixa gera aviso, nao recusa"
        );
        let adv = loaded.value.advanced().settings;
        assert_eq!(adv.foreground.verify_delay_ms, 150);
        assert!(!adv.foreground.step2);

        // settings_set_monitor: muda o monitor e grava; advanced fica exatamente como estava.
        let mut s = loaded.value;
        s.monitor_id = Some("M2".into());
        repo.save(&s).unwrap();
        let again = repo.load().value;
        assert_eq!(again.monitor_id.as_deref(), Some("M2"));
        assert_eq!(again.advanced_raw, advanced);
        assert_eq!(d.names(), vec!["settings.json".to_owned()]);
    }

    #[test]
    fn settings_com_campo_desconhecido_vai_para_backup() {
        let d = TempDir::new();
        fs::write(
            d.path().join("settings.json"),
            json!({ "schemaVersion": 1, "autostart": true }).to_string(),
        )
        .unwrap();
        let repo = JsonSettingsRepository::new(d.path(), clock());
        let loaded = repo.load();
        assert_eq!(loaded.value, Settings::default());
        assert_eq!(
            loaded.recovery.unwrap().backup_name.as_deref(),
            Some("settings.corrupt-20261113-224843.json")
        );
        assert!(d.names().contains(&"settings.json".to_owned()));
    }

    #[test]
    fn settings_advanced_invalido_nao_derruba_o_arquivo() {
        let d = TempDir::new();
        fs::write(
            d.path().join("settings.json"),
            json!({ "schemaVersion": 1, "monitorId": "M1", "advanced": "texto solto" }).to_string(),
        )
        .unwrap();
        let loaded = JsonSettingsRepository::new(d.path(), clock()).load();
        assert!(loaded.recovery.is_none());
        assert_eq!(loaded.value.monitor_id.as_deref(), Some("M1"));
        assert_eq!(loaded.value.advanced().settings, Default::default());
        assert_eq!(d.names(), vec!["settings.json".to_owned()]);
    }

    #[test]
    fn repositorios_nao_escrevem_fora_da_pasta_de_dados() {
        // Um repositorio so sabe a pasta recebida e dois nomes constantes: nada a sair dela.
        let d = TempDir::new();
        let outer = d.path().to_path_buf();
        let inner = outer.join("dados");
        let lib = JsonLibraryRepository::new(&inner, clock());
        let set = JsonSettingsRepository::new(&inner, clock());
        lib.save(&sample_library()).unwrap();
        set.save(&Settings::default()).unwrap();
        assert_eq!(d.names(), vec!["dados".to_owned()]);
        let mut inside: Vec<_> = fs::read_dir(&inner)
            .unwrap()
            .map(|e| e.unwrap().file_name().into_string().unwrap())
            .collect();
        inside.sort();
        assert_eq!(
            inside,
            vec!["library.json".to_owned(), "settings.json".to_owned()]
        );
    }
}
