//! Ports implementados pela infraestrutura (`launcher-win`). Entram com a tarefa que os usa; a
//! lista completa esta na arquitetura 2.3.

use std::any::Any;
use std::sync::atomic::{AtomicI64, AtomicU64, Ordering};
use std::sync::Arc;

use crate::application::AppError;
use crate::domain::display::Monitor;
use crate::domain::input::{InputMode, IntentEvent, PadSample};
use crate::domain::library::{
    BrowseTarget, DetectedGame, DirListing, GameId, LaunchTarget, Origin, StoredLibrary,
};
use crate::domain::power::PowerAction;
use crate::domain::session::FrontOutcome;
use crate::domain::settings::{Settings, Step2Key};
use crate::domain::time::Millis;
use crate::domain::volume::VolumeState;

use super::display_service::MonitorView;
use super::power_service::PowerCapabilities;
use super::volume_service::VolumeView;

/// Relogios. O dominio recebe `Millis`; quem chama o caso de uso le daqui.
pub trait Clock: Send + Sync {
    /// Monotonico, para prazos (60 s, 5 s, 10 s, 1,5 s).
    fn now(&self) -> Millis;
    /// Parede, em ms desde a epoca Unix, para `lastPlayedAt` e nomes de backup.
    fn unix_ms(&self) -> i64;
}

/// Relogio fake para testes: so anda quando mandam.
#[derive(Debug, Default)]
pub struct ManualClock {
    now: AtomicU64,
    unix: AtomicI64,
}

impl ManualClock {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn advance_ms(&self, ms: u64) {
        self.now.fetch_add(ms, Ordering::SeqCst);
        self.unix.fetch_add(ms as i64, Ordering::SeqCst);
    }

    pub fn set_unix_ms(&self, v: i64) {
        self.unix.store(v, Ordering::SeqCst);
    }
}

impl Clock for ManualClock {
    fn now(&self) -> Millis {
        Millis(self.now.load(Ordering::SeqCst))
    }
    fn unix_ms(&self) -> i64 {
        self.unix.load(Ordering::SeqCst)
    }
}

/// Por que um arquivo foi recriado com os padroes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RecoveryReason {
    /// Maior que o limite de 4 MB (nem foi lido).
    TooLarge,
    /// JSON invalido, campo desconhecido, versao desconhecida ou limite excedido.
    Invalid,
    /// Nao deu para ler (antivirus, backup ou indexador seguraram o arquivo) mesmo depois de
    /// tentar de novo. O arquivo **nao** foi tocado e o valor devolvido e so o padrao em memoria:
    /// o repositorio recusa `save` nessa execucao para nao gravar por cima (QA-BUG-02).
    Unreadable,
}

/// O arquivo anterior foi renomeado para `backup_name` (nome do arquivo, sem pasta).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Recovery {
    pub reason: RecoveryReason,
    pub backup_name: Option<String>,
}

/// Resultado de `load`: sempre ha um valor utilizavel; `recovery` diz se foi preciso recriar.
#[derive(Debug, Clone, PartialEq)]
pub struct Loaded<T> {
    pub value: T,
    pub recovery: Option<Recovery>,
    /// Avisos de valores corrigidos para o padrao (ex.: `advanced`), para o log.
    pub warnings: Vec<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum StoreError {
    #[error("falha de E/S na pasta de dados: {0}")]
    Io(#[from] std::io::Error),
    #[error("conteudo acima do limite de {0} bytes")]
    TooLarge(usize),
    #[error("arquivo ilegivel na carga: gravacao recusada para nao perder os dados do usuario")]
    Unreadable,
}

/// Dono unico de `library.json` (INV-03, INV-09).
pub trait LibraryRepository: Send + Sync {
    fn load(&self) -> Loaded<StoredLibrary>;
    fn save(&self, library: &StoredLibrary) -> Result<(), StoreError>;
}

/// Dono unico de `settings.json`. `save` preserva `advanced_raw` como recebido.
pub trait SettingsRepository: Send + Sync {
    fn load(&self) -> Loaded<Settings>;
    fn save(&self, settings: &Settings) -> Result<(), StoreError>;
}

// ------------------------------------------------------------------------------- entrada (BE-14)

/// Le os 4 slots de controle de uma vez. O adapter XInput consulta slot vazio so a cada 1 s
/// (ADR-0002) e devolve `connected = false` nos demais.
pub trait GamepadSource: Send {
    fn read(&mut self, now: Millis) -> [PadSample; 4];
}

/// Modo de entrada (CA-12): quem muda e a sessao (jogo na frente = `Background`).
pub trait InputGate: Send + Sync {
    fn set_mode(&self, mode: InputMode);
}

/// Para onde a thread de entrada despeja o que o dominio decidiu.
pub trait InputSink: Send + Sync {
    fn intent(&self, event: IntentEvent);
    /// Quantidade de controles mudou (CA-13 a CA-15).
    fn controllers(&self, connected: u8);
    /// A combinacao de retorno completou 1,5 s (CA-32).
    fn combo_fired(&self);
}

// ----------------------------------------------------------------------- lancamento (BE-16)

/// Verbo de inicio de um `.exe`: `Elevated` so depois do erro 740 e do aviso do Sofa (AF-01).
/// Nunca `runas`: e o verbo padrao, e o Windows decide se pede o UAC.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StartVerb {
    Normal,
    Elevated,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StartError {
    /// Erro 740: o jogo exige administrador. `fingerprint` identifica o arquivo naquele momento.
    NeedsElevation {
        fingerprint: Option<String>,
    },
    /// O usuario recusou o UAC (erro 1223).
    ElevationDenied,
    FileMissing,
    /// Caminho de rede, inclusive unidade mapeada.
    NetworkPath,
    StoreNotInstalled,
    /// Falha inesperada (so log).
    Failed,
}

/// O que o `GameStarter` entrega a quem vigia o jogo. `job` e o Job Object do `CreateProcessW`
/// (opaco para o nucleo; o adapter da sonda sabe o tipo).
#[derive(Default)]
pub struct Started {
    pub job: Option<Box<dyn Any + Send>>,
}

pub trait GameStarter: Send + Sync {
    /// Confere arquivo, forma do caminho e unidade sem iniciar nada (usado por `session_replace`
    /// antes de pedir o fechamento do jogo atual).
    fn preflight(&self, target: &LaunchTarget) -> Result<(), StartError>;
    /// Inicia. Revalida o caminho imediatamente antes (SEC-09, AF-01).
    fn start(&self, target: &LaunchTarget, verb: StartVerb) -> Result<Started, StartError>;
    /// Identidade do arquivo executavel agora (caminho + tamanho + data), para a exigencia de
    /// AF-01: o `.exe` nao pode ter trocado entre o erro 740 e o UAC. `None` para URI de loja.
    fn fingerprint(&self, target: &LaunchTarget) -> Option<String>;
}

// ------------------------------------------------------------------------ sonda (BE-17)

/// O que a sonda vigia.
pub struct WatchPlan {
    pub game: GameId,
    /// Job Object do jogo iniciado pelo Sofa (manual e GOG).
    pub job: Option<Box<dyn Any + Send>>,
    /// Pasta de instalacao para a varredura (Steam, Epic e jogo elevado, fora do Job por UIPI).
    pub scan_dir: Option<String>,
    /// Steam: `RunningAppID` mantem a sessao viva (ADR-0005).
    pub steam_app_id: Option<u32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProbeConfig {
    /// Intervalo em `Launching` e `Ending`.
    pub launching_interval_ms: u32,
    pub running_interval_ms: u32,
    pub use_steam_running_app_id: bool,
}

/// A sonda avisa a transicao do conjunto de processos associados (0 -> n e n -> 0).
/// `generation` identifica a sessao: aviso de uma sonda antiga e ignorado.
pub trait ProbeSink: Send + Sync {
    fn process_seen(&self, generation: u64);
    fn processes_gone(&self, generation: u64);
}

/// Sessao vigiada. Largar o objeto para a thread e fecha os handles; nao encerra o jogo.
pub trait SessionWatch: Send {
    /// PIDs dos processos associados agora (para localizar as janelas do jogo).
    fn pids(&self) -> Vec<u32>;
    /// `WM_CLOSE` nas janelas de topo da sessao (pedido normal, RN-12).
    fn request_close(&self) -> Result<(), AppError>;
    /// Encerra so os membros do jogo, cada um por handle guardado com tempo de criacao
    /// conferido (SEC-15, SEC-BUG-01: nunca o Job inteiro). So depois de `force_close` confirmado (INV-05).
    fn force_close(&self) -> Result<(), AppError>;
}

pub trait SessionProbe: Send + Sync {
    fn watch(
        &self,
        plan: WatchPlan,
        generation: u64,
        sink: Arc<dyn ProbeSink>,
        config: ProbeConfig,
    ) -> Result<Box<dyn SessionWatch>, AppError>;
}

// ------------------------------------------------------------------- primeiro plano (BE-18)

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Step3Result {
    Done,
    Failed,
    TimedOut,
    /// Uma tentativa anterior ainda nao voltou: nao cria outra thread (SEC-28).
    Pending,
}

/// Win32 da escada. Cada metodo e um unico passo; a ordem, as condicoes e a verificacao sao da
/// `ForegroundBroker` (aplicacao).
pub trait WindowSystem: Send + Sync {
    fn sleep_ms(&self, ms: u64);
    /// O desktop de entrada e o do usuario (`Default`) e a estacao nao esta bloqueada.
    fn desktop_is_interactive(&self) -> bool;
    /// A janela em primeiro plano e a do Sofa.
    fn launcher_is_foreground(&self) -> bool;
    /// A janela em primeiro plano pertence a processo de integridade maior que a do Sofa (UIPI).
    fn foreground_is_elevated(&self) -> bool;
    /// Mostra a janela do Sofa sem ativar e a deixa `HWND_TOPMOST` (temporario).
    fn prepare_launcher_topmost(&self);
    fn release_topmost(&self);
    /// Degrau 1: `SetForegroundWindow` simples.
    fn step1_set_foreground(&self);
    /// Degrau 3: `AttachThreadInput` + `SetForegroundWindow` + `BringWindowToTop`, em thread de
    /// trabalho com tempo limite e desanexando sempre.
    fn step3_attach(&self, timeout_ms: u64) -> Step3Result;
    /// Degrau 2: um unico par down/up da tecla + `SetForegroundWindow`.
    fn step2_key_and_foreground(&self, key: Step2Key);
    /// Degrau 4: minimiza as janelas de topo dos `pids` e chama `SetForegroundWindow`. Devolve
    /// quantas janelas minimizou.
    fn step4_minimize_games(&self, pids: &[u32]) -> usize;
    /// "Voltar ao jogo": restaura a janela do jogo e a traz para a frente (o Sofa ja esta na frente).
    fn front_game(&self, pids: &[u32]) -> bool;
    /// A janela em primeiro plano e de um dos `pids`.
    fn game_is_foreground(&self, pids: &[u32]) -> bool;
    /// `AllowSetForegroundWindow(ASFW_ANY)` antes de iniciar um jogo (CA-27).
    fn allow_set_foreground_any(&self);
}

/// Janela principal e WebView2: esconder e suspender na sessao, retomar na volta (SP-06).
pub trait LauncherWindow: Send + Sync {
    /// `hide` + `IsVisible(false)` + memoria `LOW` + `TrySuspend`. Nao esvazia a UI.
    fn hide_and_suspend(&self);
    /// `Resume` + `IsVisible(true)` + `show`.
    fn show_and_resume(&self);
}

// ----------------------------------------------------------------------- biblioteca (BE-24..BE-30)

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScanError {
    /// A loja nao esta instalada: nao e erro e nao chega a UI (CA-18, CA-59).
    NotInstalled,
    /// Falha ao ler a loja (so log; a loja some desta varredura, as outras carregam).
    Failed,
}

/// Seam de extensao das lojas (arquitetura 2.1). O adapter le arquivo e registro; as regras de
/// validacao sao do dominio (`domain::library::detect`).
pub trait StoreScanner: Send + Sync {
    fn origin(&self) -> Origin;
    fn scan(&self) -> Result<Vec<DetectedGame>, ScanError>;
}

/// Confere o `.exe` de um cadastro manual no disco (SEC-09): canonicaliza, exige unidade fixa ou
/// removivel, arquivo regular e cabecalho `MZ` + `PE`. Devolve o caminho **canonico**.
/// Erros: `PathNotAllowed` (fora dos discos), `NotFound`, `NotAnExecutable`, `AccessDenied`.
pub trait ManualExeInspector: Send + Sync {
    fn inspect(&self, path: &str) -> Result<String, AppError>;
}

/// Navegador de pastas (CA-19, SEC-09, AF-02): so unidades locais, pastas e `.exe`; nunca le
/// conteudo.
pub trait FolderBrowser: Send + Sync {
    fn list(&self, target: &BrowseTarget) -> Result<DirListing, AppError>;
}

/// De onde sai a imagem de um jogo: capa da loja e, na falta dela, o icone do `.exe`.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CoverSource {
    pub cover: Option<String>,
    pub icon: Option<String>,
}

/// Imagem reencodada pelo nucleo (SEC-08): o WebView so recebe bytes do nosso encoder.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CoverImage {
    pub mime: &'static str,
    pub bytes: Vec<u8>,
}

/// Capas e icones reencodados em cache proprio. `None` = sem imagem (404, placeholder).
pub trait CoverCache: Send + Sync {
    fn image(&self, source: &CoverSource) -> Option<CoverImage>;
}

/// Cache da ultima varredura (`cache\scan.json`, BE-40): a biblioteca aparece antes da proxima
/// varredura. Descartavel: arquivo ausente ou invalido volta vazio, e nunca e verdade para
/// iniciar jogo (SEC-18).
pub trait ScanCache: Send + Sync {
    fn load(&self) -> Vec<DetectedGame>;
    fn save(&self, games: &[DetectedGame]);
}

/// Sem cache (testes e quem nao precisa dele).
#[derive(Debug, Default, Clone, Copy)]
pub struct NullScanCache;

impl ScanCache for NullScanCache {
    fn load(&self) -> Vec<DetectedGame> {
        Vec::new()
    }
    fn save(&self, _games: &[DetectedGame]) {}
}

/// A biblioteca mudou; quem implementa le o estado do `LibraryService` e emite
/// `library://changed`.
pub trait LibraryEvents: Send + Sync {
    fn changed(&self);
}

// ------------------------------------------------------------------------- volume (BE-32)

/// Quem ouve o endpoint de audio: mudanca externa (teclado, mixer) ou troca do dispositivo padrao
/// (CA-41, CA-42). `None` = ficou sem dispositivo. Chamado de threads do COM: nao bloqueia.
pub trait AudioListener: Send + Sync {
    fn changed(&self, state: Option<VolumeState>);
}

/// Volume mestre do dispositivo de saida padrao (RN-13). O adapter vive numa thread MTA propria e
/// segue o dispositivo padrao quando ele muda.
pub trait AudioEndpoint: Send + Sync {
    /// Estado agora; `None` = sem dispositivo.
    fn state(&self) -> Option<VolumeState>;
    /// Grava nivel e mudo. Mudanca feita por aqui **nao** volta pelo `AudioListener` (o caso de
    /// uso publica o resultado). Erros: `NoAudioDevice`, `Internal`.
    fn set(&self, state: VolumeState) -> Result<(), AppError>;
    /// Registra o ouvinte (uma vez, depois de criar o caso de uso).
    fn subscribe(&self, listener: Arc<dyn AudioListener>);
}

/// Saida do volume para a UI (`volume://changed`).
pub trait VolumeEvents: Send + Sync {
    fn changed(&self, view: &VolumeView);
}

// ------------------------------------------------------------------------- energia (BE-33)

/// Win32 de energia (SEC-27). `execute` nunca forca o encerramento de outros apps (RN-15).
pub trait PowerControl: Send + Sync {
    /// O que o PC permite agora (CA-47: suspensao desligada pelo Windows ou pelo hardware).
    fn capabilities(&self) -> PowerCapabilities;
    /// Desligar, reiniciar ou suspender pelo caminho normal do Windows. `Rejected` se o sistema
    /// recusar o pedido.
    fn execute(&self, action: PowerAction) -> Result<(), AppError>;
}

/// Numeros aleatorios criptograficos do sistema (CSPRNG) para o `PowerTicket` (CT-07).
pub trait RandomSource: Send + Sync {
    fn fill(&self, buf: &mut [u8]) -> Result<(), AppError>;
}

// -------------------------------------------------------------- monitor e configuracoes (BE-34)

/// Monitores ligados agora (Win32: `EnumDisplayMonitors` + `DisplayConfig`).
pub trait DisplayTopology: Send + Sync {
    /// Lista vazia so em estado transitorio (todos desligados) ou falha de enumeracao.
    fn monitors(&self) -> Vec<Monitor>;
}

/// Coloca a janela do launcher em tela cheia sem borda no monitor dado (RN-20). Devolve `false`
/// quando nao conseguiu (janela ausente). Quem sabe se a janela esta oculta (jogo aberto) e o
/// `DisplayService`, que adia o movimento: o adapter nao deve perguntar a visibilidade ao
/// toolkit (getter bloqueante, ARQ-BUG-02).
pub trait LauncherPlacement: Send + Sync {
    fn place(&self, monitor: &Monitor) -> bool;
}

/// Preferencias gravadas em `settings.json` (a secao `advanced` nunca passa por aqui, SEC-28).
pub trait SettingsStore: Send + Sync {
    fn monitor_id(&self) -> Option<String>;
    fn set_monitor_id(&self, id: Option<String>) -> Result<(), AppError>;
    /// `apply_install_default_once` ja rodou neste perfil (BE-36).
    fn install_defaults_applied(&self) -> bool;
    fn mark_install_defaults_applied(&self) -> Result<(), AppError>;
}

/// Saida da topologia para a UI (`display://changed`).
pub trait DisplayEvents: Send + Sync {
    fn changed(&self, monitors: &[MonitorView]);
}

// ------------------------------------------------------------------ iniciar com o Windows (BE-36)

/// Falha do Agendador de Tarefas com o HRESULT (so log).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SchedulerError(pub i32);

/// O que existe no Agendador com o nome `Sofa Autostart (<SID>)`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskState {
    Absent,
    /// `UserId` (por SID), `RunLevel`, `LogonType`, acao e `Path` batem com o esperado (SEC-22).
    /// `last_result` e o `LastTaskResult`.
    Mine {
        last_result: i32,
    },
    /// Existe com esse nome mas o conteudo nao bate: nunca e tratada como valida.
    Foreign,
}

/// Tarefa de logon do proprio usuario, montada campo a campo por `ITaskService` (SEC-22). Nada de
/// `schtasks.exe` nem XML concatenado. `expected_exe = None` (rodando fora de `Program Files`):
/// o `Path` so precisa terminar em `sofa.exe`.
pub trait TaskScheduler: Send + Sync {
    fn state(&self, expected_exe: Option<&str>) -> Result<TaskState, SchedulerError>;
    /// Cria ou substitui a tarefa. `LeastPrivilege`, `InteractiveToken`, `LogonTrigger` do mesmo
    /// SID, acao `"<exe>" --autostart`, `PT0S`, `IgnoreNew`.
    fn register(&self, exe: &str) -> Result<(), SchedulerError>;
    /// Remove a tarefa pelo nome; ausente nao e erro.
    fn delete(&self) -> Result<(), SchedulerError>;
}

/// O valor em `HKCU\Software\Microsoft\Windows\CurrentVersion\Run`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunState {
    Absent,
    /// O valor e `"<exe>" --autostart` e o `StartupApproved` nao o desligou.
    Enabled,
    /// O usuario o desligou no Gerenciador de Tarefas (`StartupApproved`).
    DisabledByUser,
    /// O valor existe mas aponta para outra coisa.
    Foreign,
}

pub trait RunEntry: Send + Sync {
    fn state(&self, expected_exe: Option<&str>) -> Result<RunState, SchedulerError>;
    /// Grava `"<exe>" --autostart` e tira o "desligado" do `StartupApproved` (o usuario esta
    /// ligando a opcao de proposito).
    fn set(&self, exe: &str) -> Result<(), SchedulerError>;
    fn remove(&self) -> Result<(), SchedulerError>;
}

/// Fatos do Windows sobre a instalacao.
pub trait InstallEnv: Send + Sync {
    /// Caminho canonico do `sofa.exe` em execucao.
    fn current_exe(&self) -> Option<String>;
    /// Pastas `Program Files` do Windows (`SHGetKnownFolderPath`, nao variavel de ambiente).
    fn program_files(&self) -> Vec<String>;
    /// `HKLM\Software\app.sofa.launcher\InstallDefaults` quando for DWORD; outro tipo = `None`.
    fn install_default(&self) -> Option<u32>;
}

// ------------------------------------------------------------------------ casos de uso

/// Resolve um `GameId` no `LaunchTarget` revalidado (SEC-09, SEC-12, SEC-14). Quem implementa e
/// a biblioteca (BE-29): `UnknownGame` para id fora dela, `NetworkPathBlocked` para caminho de
/// rede e `GameFileMissing` para forma invalida.
pub trait GameResolver: Send + Sync {
    fn resolve(&self, id: &GameId) -> Result<LaunchTarget, AppError>;
}

/// `lastPlayedAt` = inicio da sessao (P-06).
pub trait PlayedRecorder: Send + Sync {
    fn mark_played(&self, id: &GameId, at_unix_ms: i64);
}

/// Executa trabalho fora da thread de quem chamou (a escada nunca roda na thread da janela).
pub trait TaskRunner: Send + Sync {
    fn run(&self, name: &'static str, task: Box<dyn FnOnce() + Send>);
}

/// Saida da sessao para a UI. Quem implementa converte para DTO e emite o evento.
pub trait SessionEvents: Send + Sync {
    fn state_changed(&self, view: &crate::application::session_service::SessionView);
    /// Combinacao de retorno concluida (`session://panel`).
    fn panel_opened(&self, outcome: FrontOutcome);
    /// A sessao acabou: a biblioteca volta com este jogo em foco (CA-30).
    fn game_ended(&self, focus: &GameId);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn relogio_fake_so_anda_quando_mandam() {
        let c = ManualClock::new();
        assert_eq!(c.now(), Millis(0));
        c.advance_ms(1500);
        assert_eq!(c.now(), Millis(1500));
        assert_eq!(c.unix_ms(), 1500);
        c.set_unix_ms(1_794_610_000_000);
        assert_eq!(c.unix_ms(), 1_794_610_000_000);
    }
}
