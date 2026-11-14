//! Casos de uso da biblioteca (BE-29, BE-30): varredura das lojas, camada do usuario, cadastro
//! manual e o resolver de lancamento. Substitui o acesso minimo de `library.json` do BE-20.
//!
//! O estado e um `Arc<LibraryState>` imutavel, trocado inteiro a cada mudanca (arquitetura 2.6):
//! `library_get` so le a copia em memoria e nunca varre. Um unico escritor de `library.json`
//! (`write`, INV-09). Jogo de loja so inicia a partir da deteccao atual, nunca de caminho gravado
//! (SEC-18).

use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::{Arc, Mutex, PoisonError, RwLock};

use crate::application::ports::{
    CoverSource, FolderBrowser, GameResolver, LibraryEvents, LibraryRepository, ManualExeInspector,
    PlayedRecorder, ScanError, StoreScanner,
};
use crate::application::AppError;
use crate::domain::library::stored::MAX_MANUAL_GAMES;
use crate::domain::library::{
    check_launch_exe_path, check_manual_exe_path, merge_library, normalize_display_name,
    parse_browse_path, DetectedGame, DirListing, GameId, LaunchPathError, LaunchSpec, LaunchTarget,
    LibraryEntry, ManualGameRecord, ManualPathError, Origin, OverlayEntry, StoredLibrary,
};

/// Foto imutavel da biblioteca: tudo que a UI e o lancamento precisam, na ordem da RN-05.
#[derive(Debug, Default)]
pub struct LibraryState {
    /// Ultimo resultado das lojas (insumo da fusao quando a camada do usuario muda).
    detected: Vec<DetectedGame>,
    /// Todos os jogos, inclusive ocultos.
    entries: Vec<LibraryEntry>,
}

impl LibraryState {
    /// Jogos visiveis, na ordem da RN-05 (`library_get`).
    pub fn visible(&self) -> impl Iterator<Item = &LibraryEntry> {
        self.entries.iter().filter(|e| !e.hidden)
    }

    /// Itens ocultos (CA-70).
    pub fn hidden(&self) -> impl Iterator<Item = &LibraryEntry> {
        self.entries.iter().filter(|e| e.hidden)
    }

    pub fn get(&self, id: &GameId) -> Option<&LibraryEntry> {
        self.entries.iter().find(|e| &e.id == id)
    }

    /// O jogo em foco ao abrir: o ultimo jogado (CA-22). `None` se nunca jogou nada.
    pub fn focus(&self) -> Option<&GameId> {
        self.visible()
            .next()
            .filter(|e| e.last_played_at.is_some())
            .map(|e| &e.id)
    }
}

pub struct LibraryDeps {
    pub repo: Arc<dyn LibraryRepository>,
    pub scanners: Vec<Arc<dyn StoreScanner>>,
    pub inspector: Arc<dyn ManualExeInspector>,
    pub browser: Arc<dyn FolderBrowser>,
    pub events: Arc<dyn LibraryEvents>,
}

pub struct LibraryService {
    deps: LibraryDeps,
    state: RwLock<Arc<LibraryState>>,
    /// Serializa ler-alterar-gravar de `library.json` e a troca do estado (INV-09).
    write: Mutex<()>,
}

impl LibraryService {
    /// Carrega so o que e do usuario (jogos manuais e overlay): a biblioteca ja responde antes da
    /// primeira varredura.
    pub fn new(deps: LibraryDeps) -> Self {
        let service = Self {
            deps,
            state: RwLock::new(Arc::new(LibraryState::default())),
            write: Mutex::new(()),
        };
        {
            let _guard = service.write.lock().unwrap_or_else(PoisonError::into_inner);
            service.rebuild(Vec::new());
        }
        service
    }

    pub fn snapshot(&self) -> Arc<LibraryState> {
        Arc::clone(&self.state.read().unwrap_or_else(PoisonError::into_inner))
    }

    fn load_stored(&self) -> StoredLibrary {
        let loaded = self.deps.repo.load();
        if let Some(r) = &loaded.recovery {
            tracing::warn!(motivo = ?r.reason, backup = ?r.backup_name, "library.json recriado com os padroes");
        }
        loaded.value
    }

    /// Refaz o estado com `detected` e a camada do usuario atual. Chamar com `write` tomado.
    fn rebuild(&self, detected: Vec<DetectedGame>) {
        let stored = self.load_stored();
        self.install(detected, &stored);
    }

    fn install(&self, detected: Vec<DetectedGame>, stored: &StoredLibrary) {
        let entries = merge_library(detected.clone(), &stored.manual_games, &stored.overlay);
        *self.state.write().unwrap_or_else(PoisonError::into_inner) =
            Arc::new(LibraryState { detected, entries });
    }

    /// Grava `stored` e refaz o estado sobre a ultima deteccao. Chamar com `write` tomado.
    fn commit(&self, stored: &StoredLibrary) -> Result<(), AppError> {
        self.deps.repo.save(stored).map_err(|e| {
            tracing::error!(erro = %e, "nao gravou library.json");
            AppError::Internal
        })?;
        let detected = self.snapshot().detected.clone();
        self.install(detected, stored);
        Ok(())
    }

    fn notify(&self) {
        self.deps.events.changed();
    }

    // ------------------------------------------------------------------------------ varredura

    /// `library_refresh`: lojas em paralelo, cada uma isolada (erro e `panic` viram log e a loja
    /// some desta varredura; as outras e os jogos manuais carregam, SEC-13). Emite
    /// `library://changed`. A serializacao (`Busy`) e de quem chama.
    pub fn refresh(&self) {
        let started = std::time::Instant::now();
        let results: Vec<(Origin, Result<Vec<DetectedGame>, ScanError>)> = std::thread::scope(
            |scope| {
                let handles: Vec<_> = self
                    .deps
                    .scanners
                    .iter()
                    .map(|scanner| {
                        let scanner = Arc::clone(scanner);
                        let origin = scanner.origin();
                        let handle = std::thread::Builder::new()
                            .name(format!("scan-{}", origin.prefix()))
                            .spawn_scoped(scope, move || {
                                catch_unwind(AssertUnwindSafe(|| scanner.scan()))
                            });
                        (origin, handle)
                    })
                    .collect();
                handles
                    .into_iter()
                    .map(|(origin, handle)| {
                        let outcome = match handle.map(|h| h.join()) {
                            Ok(Ok(Ok(result))) => result,
                            Ok(_) => {
                                tracing::error!(loja = origin.prefix(), "varredura entrou em panico");
                                Err(ScanError::Failed)
                            }
                            Err(error) => {
                                tracing::error!(loja = origin.prefix(), %error, "thread de varredura nao iniciou");
                                Err(ScanError::Failed)
                            }
                        };
                        (origin, outcome)
                    })
                    .collect()
            },
        );

        let mut detected = Vec::new();
        for (origin, result) in results {
            match result {
                Ok(games) => {
                    tracing::debug!(
                        loja = origin.prefix(),
                        jogos = games.len(),
                        "varredura concluida"
                    );
                    // So o que e da propria origem entra: um scanner nao injeta id de outra.
                    detected.extend(games.into_iter().filter(|g| g.id.origin() == origin));
                }
                Err(ScanError::NotInstalled) => {
                    tracing::debug!(loja = origin.prefix(), "loja nao instalada");
                }
                Err(ScanError::Failed) => {
                    tracing::warn!(loja = origin.prefix(), "loja ignorada nesta varredura");
                }
            }
        }
        {
            let _guard = self.write.lock().unwrap_or_else(PoisonError::into_inner);
            self.rebuild(detected);
        }
        tracing::info!(
            jogos = self.snapshot().entries.len(),
            ms = started.elapsed().as_millis() as u64,
            "biblioteca atualizada"
        );
        self.notify();
    }

    // -------------------------------------------------------------------------- camada do usuario

    fn with_overlay(
        &self,
        id: &GameId,
        change: impl FnOnce(&mut OverlayEntry),
    ) -> Result<(), AppError> {
        let _guard = self.write.lock().unwrap_or_else(PoisonError::into_inner);
        if self.snapshot().get(id).is_none() {
            return Err(AppError::UnknownGame);
        }
        let mut stored = self.load_stored();
        let entry = stored.overlay.entry(id.clone()).or_default();
        change(entry);
        if *entry == OverlayEntry::default() {
            stored.overlay.remove(id);
        }
        self.commit(&stored)
    }

    fn dto_entry(&self, id: &GameId) -> Result<LibraryEntry, AppError> {
        self.snapshot()
            .get(id)
            .cloned()
            .ok_or(AppError::UnknownGame)
    }

    /// `game_rename` (CA-61, RN-22): normaliza; vazio depois de normalizar = `EmptyName`.
    /// Renomear nao altera o ID.
    pub fn rename(&self, id: &GameId, name: &str) -> Result<LibraryEntry, AppError> {
        let normalized = normalize_display_name(name);
        // Jogo desconhecido tem prioridade sobre nome vazio: nao diz nada de um id que nao existe.
        let current = self.dto_entry(id)?;
        if normalized.is_empty() {
            return Err(AppError::EmptyName);
        }
        let custom = (normalized != current.original_name).then_some(normalized);
        self.with_overlay(id, |o| o.custom_name = custom)?;
        self.notify();
        self.dto_entry(id)
    }

    /// `game_restore_name` (CA-62): volta ao nome da origem.
    pub fn restore_name(&self, id: &GameId) -> Result<LibraryEntry, AppError> {
        self.with_overlay(id, |o| o.custom_name = None)?;
        self.notify();
        self.dto_entry(id)
    }

    /// `game_hide` (RN-04): nunca apaga arquivo; o item vai para a lista de ocultos.
    pub fn hide(&self, id: &GameId) -> Result<(), AppError> {
        self.with_overlay(id, |o| o.hidden = true)?;
        self.notify();
        Ok(())
    }

    /// `game_unhide` (CA-70): preserva nome personalizado e data de ultimo jogo.
    pub fn unhide(&self, id: &GameId) -> Result<(), AppError> {
        self.with_overlay(id, |o| o.hidden = false)?;
        self.notify();
        Ok(())
    }

    // --------------------------------------------------------------------------- cadastro manual

    /// `library_add_manual` (CA-19, CA-20, CA-72, SEC-09, RN-29): forma, disco (canonicalizar,
    /// unidade local, `MZ` + `PE`), duplicata (RN-03). Nunca aceita argumentos.
    pub fn add_manual(&self, path: &str) -> Result<LibraryEntry, AppError> {
        let map = |e: ManualPathError| match e {
            ManualPathError::Malformed => AppError::InvalidRequest,
            ManualPathError::NotAllowed => AppError::PathNotAllowed,
            ManualPathError::NotExecutable => AppError::NotAnExecutable,
        };
        check_manual_exe_path(path).map_err(map)?;
        let canonical = self.deps.inspector.inspect(path)?;
        // A forma vale de novo sobre o caminho canonico (resolver links nao a contorna).
        check_manual_exe_path(&canonical).map_err(map)?;

        let id = GameId::manual_from_canonical_path(&canonical);
        let _guard = self.write.lock().unwrap_or_else(PoisonError::into_inner);
        let mut stored = self.load_stored();
        // Mesmo ID = mesmo caminho em outra caixa (RN-03) ou colisao de 64 bits: `Duplicate`.
        if stored.manual_games.iter().any(|g| g.id == id) {
            return Err(AppError::Duplicate);
        }
        if stored.manual_games.len() >= MAX_MANUAL_GAMES {
            return Err(AppError::InvalidRequest);
        }
        let stem = canonical
            .rsplit('\\')
            .next()
            .and_then(|f| f.rsplit_once('.').map(|(s, _)| s))
            .unwrap_or("");
        let name = match normalize_display_name(stem) {
            n if n.is_empty() => id.key().to_owned(),
            n => n,
        };
        stored.manual_games.push(ManualGameRecord {
            id: id.clone(),
            name,
            path: canonical,
        });
        self.commit(&stored)?;
        drop(_guard);
        self.notify();
        self.dto_entry(&id)
    }

    /// `fs_list_dir` (CT-04): valida o pedido (SEC-09) e delega ao navegador.
    pub fn list_dir(&self, path: Option<&str>) -> Result<DirListing, AppError> {
        let target = parse_browse_path(path).map_err(|e| match e {
            crate::domain::library::BrowseError::Malformed => AppError::InvalidRequest,
            crate::domain::library::BrowseError::NotAllowed => AppError::PathNotAllowed,
        })?;
        self.deps.browser.list(&target)
    }

    // ---------------------------------------------------------------------------------- capas

    /// De onde sai a imagem do jogo; `None` para id fora da biblioteca (`cover://` responde 404,
    /// SEC-08, CT-06). So busca por igualdade de `GameId`: o id nunca compoe caminho.
    pub fn cover_source(&self, id: &GameId) -> Option<CoverSource> {
        let state = self.snapshot();
        let e = state.get(id)?;
        Some(CoverSource {
            cover: e.cover_source.clone(),
            icon: e.icon_source.clone(),
        })
    }
}

/// Pasta que contem o arquivo (separador `\`).
fn parent_dir(path: &str) -> Option<String> {
    path.rsplit_once('\\')
        .map(|(dir, _)| dir.to_owned())
        .filter(|d| !d.is_empty())
}

impl GameResolver for LibraryService {
    fn resolve(&self, id: &GameId) -> Result<LaunchTarget, AppError> {
        let state = self.snapshot();
        let entry = state.get(id).ok_or(AppError::UnknownGame)?;
        let spec = entry.launch.clone();
        if id.origin() == Origin::Manual {
            // Revalidacao no lancamento (SEC-18, CA-71 2o momento): o arquivo pode ter sido
            // editado a mao.
            let LaunchSpec::Exe { path, .. } = &spec else {
                return Err(AppError::UnknownGame);
            };
            match check_launch_exe_path(path) {
                Ok(()) => {}
                Err(LaunchPathError::Network) => return Err(AppError::NetworkPathBlocked),
                Err(LaunchPathError::Invalid) => {
                    tracing::warn!(game = %id, "jogo manual com caminho invalido em library.json");
                    return Err(AppError::GameFileMissing {
                        game_id: id.clone(),
                    });
                }
            }
            let dir = parent_dir(path);
            return Ok(LaunchTarget {
                game: id.clone(),
                spec,
                install_dir: dir,
            });
        }
        Ok(LaunchTarget {
            game: id.clone(),
            spec,
            install_dir: entry.install_dir.clone(),
        })
    }
}

impl PlayedRecorder for LibraryService {
    fn mark_played(&self, id: &GameId, at_unix_ms: i64) {
        let result = self.with_overlay(id, |o| o.last_played_at = Some(at_unix_ms));
        if let Err(e) = result {
            tracing::warn!(game = %id, erro = %e, "nao gravou lastPlayedAt");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::ports::{Loaded, StoreError};
    use crate::domain::library::{BrowseTarget, OverlayEntry};
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[derive(Default)]
    struct Mem(Mutex<StoredLibrary>);
    impl LibraryRepository for Mem {
        fn load(&self) -> Loaded<StoredLibrary> {
            Loaded {
                value: self.0.lock().unwrap().clone(),
                recovery: None,
                warnings: Vec::new(),
            }
        }
        fn save(&self, library: &StoredLibrary) -> Result<(), StoreError> {
            *self.0.lock().unwrap() = library.clone();
            Ok(())
        }
    }

    enum Behavior {
        Games(Vec<DetectedGame>),
        Fail,
        Panic,
        NotInstalled,
    }
    struct FakeScanner(Origin, Behavior);
    impl StoreScanner for FakeScanner {
        fn origin(&self) -> Origin {
            self.0
        }
        fn scan(&self) -> Result<Vec<DetectedGame>, ScanError> {
            match &self.1 {
                Behavior::Games(g) => Ok(g.clone()),
                Behavior::Fail => Err(ScanError::Failed),
                Behavior::NotInstalled => Err(ScanError::NotInstalled),
                Behavior::Panic => panic!("manifesto malformado"),
            }
        }
    }

    /// Aceita o que o cadastro pediria ao disco: devolve o caminho como esta, ou o erro roteirizado.
    struct FakeInspector(Mutex<Option<AppError>>);
    impl ManualExeInspector for FakeInspector {
        fn inspect(&self, path: &str) -> Result<String, AppError> {
            match self.0.lock().unwrap().clone() {
                Some(e) => Err(e),
                None => Ok(path.to_owned()),
            }
        }
    }

    struct NoBrowser;
    impl FolderBrowser for NoBrowser {
        fn list(&self, _: &BrowseTarget) -> Result<DirListing, AppError> {
            Err(AppError::Internal)
        }
    }

    #[derive(Default)]
    struct Count(AtomicUsize);
    impl LibraryEvents for Count {
        fn changed(&self) {
            self.0.fetch_add(1, Ordering::SeqCst);
        }
    }

    struct Rig {
        svc: LibraryService,
        mem: Arc<Mem>,
        events: Arc<Count>,
        inspector: Arc<FakeInspector>,
    }

    fn det(game: &str, name: &str) -> DetectedGame {
        let id = GameId::parse(game).unwrap();
        let launch = match id.origin() {
            Origin::Steam => LaunchSpec::SteamUri {
                app_id: id.key().parse().unwrap(),
            },
            _ => LaunchSpec::Exe {
                path: r"C:\GOG Games\X\g.exe".into(),
                args: vec!["-a".into()],
                working_dir: Some(r"C:\GOG Games\X".into()),
            },
        };
        DetectedGame {
            id,
            name: name.into(),
            install_dir: Some(r"C:\Jogos\X".into()),
            launch,
            cover_source: Some(r"C:\capa.jpg".into()),
            icon_source: None,
        }
    }

    fn rig_with(scanners: Vec<FakeScanner>, stored: StoredLibrary) -> Rig {
        let mem = Arc::new(Mem(Mutex::new(stored)));
        let events = Arc::new(Count::default());
        let inspector = Arc::new(FakeInspector(Mutex::new(None)));
        let svc = LibraryService::new(LibraryDeps {
            repo: mem.clone(),
            scanners: scanners
                .into_iter()
                .map(|s| Arc::new(s) as Arc<dyn StoreScanner>)
                .collect(),
            inspector: inspector.clone(),
            browser: Arc::new(NoBrowser),
            events: events.clone(),
        });
        Rig {
            svc,
            mem,
            events,
            inspector,
        }
    }

    fn rig(scanners: Vec<FakeScanner>) -> Rig {
        rig_with(scanners, StoredLibrary::default())
    }

    fn names(r: &Rig) -> Vec<String> {
        r.svc
            .snapshot()
            .visible()
            .map(|e| e.display_name().to_owned())
            .collect()
    }

    fn id(s: &str) -> GameId {
        GameId::parse(s).unwrap()
    }

    #[test]
    fn manuais_aparecem_antes_da_primeira_varredura() {
        let path = r"C:\Jogos\Foo\foo.exe";
        let stored = StoredLibrary {
            manual_games: vec![ManualGameRecord {
                id: GameId::manual_from_canonical_path(path),
                name: "Foo".into(),
                path: path.into(),
            }],
            ..StoredLibrary::default()
        };
        let r = rig_with(Vec::new(), stored);
        assert_eq!(names(&r), ["Foo"]);
        assert_eq!(r.events.0.load(Ordering::SeqCst), 0); // construir nao emite evento
    }

    #[test]
    fn refresh_une_as_lojas_e_emite_um_evento() {
        let r = rig(vec![
            FakeScanner(
                Origin::Steam,
                Behavior::Games(vec![det("steam:620", "Portal 2")]),
            ),
            FakeScanner(
                Origin::Epic,
                Behavior::Games(vec![det("epic:Sample", "Sample")]),
            ),
            FakeScanner(Origin::Gog, Behavior::NotInstalled),
        ]);
        r.svc.refresh();
        assert_eq!(names(&r), ["Portal 2", "Sample"]);
        assert_eq!(r.events.0.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn loja_que_falha_ou_entra_em_panico_some_e_as_outras_carregam() {
        let path = r"C:\Jogos\Foo\foo.exe";
        let stored = StoredLibrary {
            manual_games: vec![ManualGameRecord {
                id: GameId::manual_from_canonical_path(path),
                name: "Foo".into(),
                path: path.into(),
            }],
            ..StoredLibrary::default()
        };
        let r = rig_with(
            vec![
                FakeScanner(Origin::Steam, Behavior::Panic),
                FakeScanner(Origin::Epic, Behavior::Fail),
                FakeScanner(
                    Origin::Gog,
                    Behavior::Games(vec![det("gog:1000000001", "Sample GOG")]),
                ),
            ],
            stored,
        );
        r.svc.refresh();
        assert_eq!(names(&r), ["Foo", "Sample GOG"]);
        assert_eq!(r.events.0.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn scanner_nao_injeta_id_de_outra_origem() {
        let r = rig(vec![FakeScanner(
            Origin::Epic,
            Behavior::Games(vec![det("steam:620", "Intruso"), det("epic:Ok", "Ok")]),
        )]);
        r.svc.refresh();
        assert_eq!(names(&r), ["Ok"]);
    }

    #[test]
    fn jogo_desinstalado_some_na_proxima_atualizacao() {
        let r = rig(vec![FakeScanner(
            Origin::Steam,
            Behavior::Games(vec![det("steam:620", "Portal 2")]),
        )]);
        r.svc.refresh();
        assert_eq!(names(&r).len(), 1);
        let vazio = rig_with(
            vec![FakeScanner(Origin::Steam, Behavior::NotInstalled)],
            r.mem.0.lock().unwrap().clone(),
        );
        vazio.svc.refresh();
        assert!(names(&vazio).is_empty());
    }

    #[test]
    fn oculto_nao_volta_apos_refresh_e_unhide_preserva_nome_e_data() {
        let r = rig(vec![FakeScanner(
            Origin::Steam,
            Behavior::Games(vec![det("steam:620", "Portal 2")]),
        )]);
        r.svc.refresh();
        let g = id("steam:620");
        r.svc.rename(&g, "Portal Coop").unwrap();
        r.svc.mark_played(&g, 123);
        r.svc.hide(&g).unwrap();
        assert!(names(&r).is_empty());
        r.svc.refresh();
        assert!(names(&r).is_empty(), "oculto nao volta na varredura");
        let snap = r.svc.snapshot();
        let ocultos: Vec<_> = snap.hidden().collect();
        assert_eq!(ocultos.len(), 1);
        r.svc.unhide(&g).unwrap();
        let snap = r.svc.snapshot();
        let e = snap.get(&g).unwrap();
        assert_eq!(e.display_name(), "Portal Coop");
        assert_eq!(e.last_played_at, Some(123));
        assert!(!e.hidden);
    }

    #[test]
    fn renomear_normaliza_restaura_e_valida() {
        let r = rig(vec![FakeScanner(
            Origin::Steam,
            Behavior::Games(vec![det("steam:620", "Portal 2")]),
        )]);
        r.svc.refresh();
        let g = id("steam:620");
        let e = r.svc.rename(&g, "  Portal\u{202E} 2 (coop)\u{0} ").unwrap();
        assert_eq!(e.display_name(), "Portal 2 (coop)");
        assert_eq!(e.original_name, "Portal 2");
        assert_eq!(
            r.svc.rename(&g, "\u{202E}  ").unwrap_err(),
            AppError::EmptyName
        );
        assert_eq!(r.svc.rename(&g, "").unwrap_err(), AppError::EmptyName);
        assert_eq!(
            r.svc.rename(&id("steam:999"), "x").unwrap_err(),
            AppError::UnknownGame
        );
        // Nome personalizado sobrevive a nova varredura (RN-22).
        r.svc.refresh();
        assert_eq!(names(&r), ["Portal 2 (coop)"]);
        let e = r.svc.restore_name(&g).unwrap();
        assert_eq!(e.display_name(), "Portal 2");
        assert!(
            r.mem.0.lock().unwrap().overlay.is_empty(),
            "overlay padrao e removido"
        );
        // Renomear para o nome original nao grava nome personalizado.
        r.svc.rename(&g, "Portal 2").unwrap();
        assert!(r.mem.0.lock().unwrap().overlay.is_empty());
        assert_eq!(
            r.svc.restore_name(&id("steam:999")).unwrap_err(),
            AppError::UnknownGame
        );
        assert_eq!(
            r.svc.hide(&id("steam:999")).unwrap_err(),
            AppError::UnknownGame
        );
        assert_eq!(
            r.svc.unhide(&id("steam:999")).unwrap_err(),
            AppError::UnknownGame
        );
    }

    #[test]
    fn cadastro_manual_canonicaliza_deduplica_e_nomeia_pelo_arquivo() {
        let r = rig(Vec::new());
        let e = r.svc.add_manual(r"C:\Jogos\Meu Jogo\meu.jogo.exe").unwrap();
        assert_eq!(e.display_name(), "meu.jogo");
        assert_eq!(e.id.origin(), Origin::Manual);
        assert_eq!(r.events.0.load(Ordering::SeqCst), 1);
        // O mesmo arquivo em outra caixa e duplicata (RN-03, CA-20).
        assert_eq!(
            r.svc
                .add_manual(r"c:\JOGOS\meu jogo\MEU.JOGO.EXE")
                .unwrap_err(),
            AppError::Duplicate
        );
        assert_eq!(r.mem.0.lock().unwrap().manual_games.len(), 1);
    }

    #[test]
    fn cadastro_manual_recusa_com_o_codigo_certo() {
        let r = rig(Vec::new());
        let casos: &[(&str, AppError)] = &[
            (r"C:\Jogos\atalho.lnk", AppError::NotAnExecutable),
            (r"C:\Jogos\run.bat", AppError::NotAnExecutable),
            (r"C:\Jogos\run.cmd", AppError::NotAnExecutable),
            (r"C:\Jogos\leia.txt", AppError::NotAnExecutable),
            (r"C:\jogo.exe:ads", AppError::NotAnExecutable),
            (r"\\127.0.0.1\c$\Jogos\x.exe", AppError::PathNotAllowed),
            (r"\\?\C:\Jogos\x.exe", AppError::PathNotAllowed),
            (r"jogo.exe", AppError::InvalidRequest),
            ("", AppError::InvalidRequest),
        ];
        for (path, esperado) in casos {
            assert_eq!(r.svc.add_manual(path).unwrap_err(), *esperado, "{path}");
        }
        // O disco recusa (txt renomeado para .exe, unidade de rede mapeada, arquivo ausente).
        for erro in [
            AppError::NotAnExecutable,
            AppError::PathNotAllowed,
            AppError::NotFound,
        ] {
            *r.inspector.0.lock().unwrap() = Some(erro.clone());
            assert_eq!(r.svc.add_manual(r"C:\Jogos\x.exe").unwrap_err(), erro);
        }
        assert!(r.mem.0.lock().unwrap().manual_games.is_empty());
        assert_eq!(r.events.0.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn caminho_canonico_que_vira_rede_e_recusado() {
        // Um link que resolve para UNC passa na forma bruta e cai na checagem do canonico.
        struct ToUnc;
        impl ManualExeInspector for ToUnc {
            fn inspect(&self, _: &str) -> Result<String, AppError> {
                Ok(r"\\srv\share\x.exe".into())
            }
        }
        let mem = Arc::new(Mem::default());
        let svc = LibraryService::new(LibraryDeps {
            repo: mem,
            scanners: Vec::new(),
            inspector: Arc::new(ToUnc),
            browser: Arc::new(NoBrowser),
            events: Arc::new(Count::default()),
        });
        assert_eq!(
            svc.add_manual(r"C:\Jogos\link.exe").unwrap_err(),
            AppError::PathNotAllowed
        );
    }

    #[test]
    fn manual_editado_a_mao_nao_inicia_e_rede_da_network_path_blocked() {
        let mk = |path: &str| {
            let id = GameId::manual_from_canonical_path(path);
            (
                id.clone(),
                StoredLibrary {
                    manual_games: vec![ManualGameRecord {
                        id,
                        name: "Jogo".into(),
                        path: path.into(),
                    }],
                    ..StoredLibrary::default()
                },
            )
        };
        let (rede_id, rede) = mk(r"\\servidor\share\game.exe");
        assert_eq!(
            rig_with(Vec::new(), rede)
                .svc
                .resolve(&rede_id)
                .unwrap_err(),
            AppError::NetworkPathBlocked
        );
        let (bat_id, bat) = mk(r"C:\Jogos\script.bat");
        assert!(matches!(
            rig_with(Vec::new(), bat).svc.resolve(&bat_id).unwrap_err(),
            AppError::GameFileMissing { .. }
        ));
        let (ok_id, ok) = mk(r"C:\Jogos\A & B\game.exe");
        let t = rig_with(Vec::new(), ok).svc.resolve(&ok_id).unwrap();
        assert_eq!(t.install_dir.as_deref(), Some(r"C:\Jogos\A & B"));
        match t.spec {
            LaunchSpec::Exe { path, args, .. } => {
                assert_eq!(path, r"C:\Jogos\A & B\game.exe");
                assert!(args.is_empty());
            }
            other => panic!("esperava Exe, veio {other:?}"),
        }
    }

    #[test]
    fn jogo_de_loja_resolve_da_deteccao_atual_e_id_desconhecido_nao() {
        let r = rig(vec![
            FakeScanner(
                Origin::Steam,
                Behavior::Games(vec![det("steam:620", "Portal 2")]),
            ),
            FakeScanner(
                Origin::Gog,
                Behavior::Games(vec![det("gog:1000000001", "G")]),
            ),
        ]);
        // Antes da varredura nao ha deteccao: nenhum jogo de loja inicia de caminho gravado.
        assert_eq!(
            r.svc.resolve(&id("steam:620")).unwrap_err(),
            AppError::UnknownGame
        );
        r.svc.refresh();
        let t = r.svc.resolve(&id("steam:620")).unwrap();
        assert_eq!(t.spec, LaunchSpec::SteamUri { app_id: 620 });
        assert_eq!(t.install_dir.as_deref(), Some(r"C:\Jogos\X"));
        let g = r.svc.resolve(&id("gog:1000000001")).unwrap();
        assert!(matches!(g.spec, LaunchSpec::Exe { ref args, .. } if args == &["-a".to_owned()]));
        assert_eq!(
            r.svc.resolve(&id("steam:1")).unwrap_err(),
            AppError::UnknownGame
        );
        assert_eq!(
            r.svc.resolve(&id("manual:0123456789abcdef")).unwrap_err(),
            AppError::UnknownGame
        );
    }

    #[test]
    fn mark_played_ordena_e_foco_e_o_ultimo_jogado() {
        let r = rig(vec![FakeScanner(
            Origin::Steam,
            Behavior::Games(vec![
                det("steam:1", "Alfa"),
                det("steam:2", "Beta"),
                det("steam:3", "Gama"),
            ]),
        )]);
        r.svc.refresh();
        assert_eq!(r.svc.snapshot().focus(), None);
        r.svc.mark_played(&id("steam:3"), 10);
        r.svc.mark_played(&id("steam:2"), 20);
        assert_eq!(names(&r), ["Beta", "Gama", "Alfa"]);
        assert_eq!(r.svc.snapshot().focus(), Some(&id("steam:2")));
        let stored = r.mem.0.lock().unwrap().overlay.clone();
        assert_eq!(
            stored[&id("steam:2")],
            OverlayEntry {
                last_played_at: Some(20),
                ..OverlayEntry::default()
            }
        );
        // Jogo desconhecido: so log, sem gravar nada.
        r.svc.mark_played(&id("steam:999"), 1);
        assert!(!r
            .mem
            .0
            .lock()
            .unwrap()
            .overlay
            .contains_key(&id("steam:999")));
    }

    #[test]
    fn cover_source_so_para_id_da_biblioteca() {
        let r = rig(vec![FakeScanner(
            Origin::Steam,
            Behavior::Games(vec![det("steam:620", "Portal 2")]),
        )]);
        r.svc.refresh();
        let s = r.svc.cover_source(&id("steam:620")).unwrap();
        assert_eq!(s.cover.as_deref(), Some(r"C:\capa.jpg"));
        assert!(r.svc.cover_source(&id("steam:1")).is_none());
        assert!(r.svc.cover_source(&id("epic:..")).is_none());
    }

    #[test]
    fn list_dir_valida_o_pedido_antes_de_chegar_ao_disco() {
        let r = rig(Vec::new());
        assert_eq!(
            r.svc.list_dir(Some(r"\\127.0.0.1\c$")).unwrap_err(),
            AppError::PathNotAllowed
        );
        assert_eq!(
            r.svc.list_dir(Some(r"C:\a\..\b")).unwrap_err(),
            AppError::PathNotAllowed
        );
        assert_eq!(
            r.svc.list_dir(Some("relativo")).unwrap_err(),
            AppError::InvalidRequest
        );
        // Pedido valido chega ao navegador (que aqui falha de proposito).
        assert_eq!(r.svc.list_dir(None).unwrap_err(), AppError::Internal);
    }
}
