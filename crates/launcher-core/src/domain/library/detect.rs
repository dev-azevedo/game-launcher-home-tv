//! Do manifesto da loja ao `DetectedGame` (SEC-11 a SEC-14, SEC-16, RN-28). Os leitores de
//! arquivo e de registro (`launcher-win/stores`) so extraem campos crus; **toda a validacao e
//! daqui**. ID ou caminho fora da regra faz o jogo ser ignorado com o motivo; nada e "sanitizado".
//!
//! O disco entra por `canon`: devolve o caminho canonico (links resolvidos) ou `None` se nao
//! existe. Assim a regra continua pura e os testes usam um `canon` de mentira.

use super::exe_policy::check_store_exe;
use super::id::GameId;
use super::id_formats::{is_epic_component, is_gog_id, parse_steam_app_id};
use super::install_path_policy::{
    local_components, normalize_local, resolve_confined, InstallPathPolicy, PathPolicyError,
};
use super::launch::LaunchSpec;
use super::model::DetectedGame;
use super::steam_non_games::is_steam_non_game;
use crate::domain::session::path_is_within;

/// Por que um item da loja nao entrou na biblioteca.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reject {
    /// Steam: item da lista fixa de nao-jogos (RN-28).
    NotAGame,
    /// Steam: `StateFlags` sem o bit de instalado (CA-17).
    NotInstalled,
    /// Epic: instalacao incompleta.
    Incomplete,
    /// Epic: DLC (`AppName` diferente de `MainGameAppName`).
    Dlc,
    /// Campo obrigatorio ausente ou vazio.
    MissingField,
    /// ID fora do formato da origem (SEC-11).
    BadId,
    /// `installdir`, `InstallLocation`, `path` ou `workingDir` fora da regra (SEC-12).
    BadPath,
    /// Pasta de instalacao proibida (SEC-16).
    BadInstallDir,
    /// A pasta ou o executavel nao existe (inclui unidade desconectada, E-18).
    Missing,
    /// Executavel que nao e `.exe`, e de `%SystemRoot%` ou escapa da pasta (SEC-12, SEC-14).
    BadExe,
    /// GOG: `.info` sem tarefa primaria de jogo (`URLTask`, DLC) ou argumentos invalidos.
    NoPlayTask,
}

impl Reject {
    pub fn as_str(self) -> &'static str {
        match self {
            Reject::NotAGame => "item que nao e jogo",
            Reject::NotInstalled => "nao instalado",
            Reject::Incomplete => "instalacao incompleta",
            Reject::Dlc => "dlc",
            Reject::MissingField => "campo obrigatorio ausente",
            Reject::BadId => "id fora do formato",
            Reject::BadPath => "caminho fora da regra",
            Reject::BadInstallDir => "pasta de instalacao nao permitida",
            Reject::Missing => "pasta ou executavel inexistente",
            Reject::BadExe => "executavel nao permitido",
            Reject::NoPlayTask => "sem tarefa de jogo valida",
        }
    }

    /// Filtros esperados (nao sao defeito do manifesto): log de depuracao, nao aviso.
    pub fn is_expected(self) -> bool {
        matches!(
            self,
            Reject::NotAGame | Reject::NotInstalled | Reject::Incomplete | Reject::Dlc
        )
    }
}

fn non_empty(s: &Option<String>) -> Option<&str> {
    s.as_deref().map(str::trim).filter(|s| !s.is_empty())
}

fn required(v: &Option<String>) -> Result<&str, Reject> {
    v.as_deref()
        .filter(|s| !s.is_empty())
        .ok_or(Reject::MissingField)
}

fn canon_dir(
    dir: &str,
    canon: &dyn Fn(&str) -> Option<String>,
    policy: &InstallPathPolicy,
) -> Result<String, Reject> {
    local_components(dir).map_err(|_| Reject::BadPath)?;
    let canonical = canon(dir).ok_or(Reject::Missing)?;
    policy.check_install_dir(&canonical).map_err(|e| match e {
        PathPolicyError::NotLocalAbsolute | PathPolicyError::EscapesBase => Reject::BadPath,
        _ => Reject::BadInstallDir,
    })
}

/// Executavel de loja: confinado a `dir` (antes e depois de resolver links), `.exe`, fora de
/// `%SystemRoot%`. `dir` ja e canonico.
fn confined_exe(
    dir: &str,
    rel: &str,
    canon: &dyn Fn(&str) -> Option<String>,
    policy: &InstallPathPolicy,
) -> Result<String, Reject> {
    let lexical = resolve_confined(dir, rel).map_err(|_| Reject::BadPath)?;
    check_store_exe(&lexical, policy).map_err(|_| Reject::BadExe)?;
    let real = canon(&lexical).ok_or(Reject::Missing)?;
    if !path_is_within(dir, &real) || normalize_local(&real).is_err() {
        return Err(Reject::BadExe); // link apontando para fora da pasta
    }
    check_store_exe(&real, policy).map_err(|_| Reject::BadExe)?;
    Ok(real)
}

// ------------------------------------------------------------------------------------ Steam

/// Campos crus de um `appmanifest_<appid>.acf` (qualquer texto: o parser nao valida).
#[derive(Debug, Clone, Default)]
pub struct SteamManifestRaw {
    pub appid: Option<String>,
    pub name: Option<String>,
    pub state_flags: Option<String>,
    pub installdir: Option<String>,
}

/// `StateFlags` com o bit 4 (`StateFullyInstalled`): testa o bit, nao a igualdade (`6` = instalado
/// com atualizacao pendente aparece; `1`, `2` e `1026` nao).
fn steam_installed(flags: &str) -> bool {
    flags.trim().parse::<u32>().is_ok_and(|f| f & 4 != 0)
}

/// `installdir` e um unico componente: sem separador, `..`, `.`, `:` nem controle.
fn single_component(s: &str) -> bool {
    !s.is_empty()
        && s != "."
        && s != ".."
        && !s.contains(['\\', '/', ':', '<', '>', '"', '|', '?', '*'])
        && !s.chars().any(char::is_control)
        && !s.ends_with('.')
        && !s.ends_with(' ')
}

/// `library_dir` ja e canonico e passou pela validacao de raiz (SEC-12). Devolve o jogo sem
/// capa: achar o arquivo da capa e do adapter.
pub fn steam_game(
    raw: &SteamManifestRaw,
    library_dir: &str,
    policy: &InstallPathPolicy,
    canon: &dyn Fn(&str) -> Option<String>,
) -> Result<DetectedGame, Reject> {
    let appid = non_empty(&raw.appid).ok_or(Reject::MissingField)?;
    let app_id = parse_steam_app_id(appid).ok_or(Reject::BadId)?;
    if is_steam_non_game(app_id) {
        return Err(Reject::NotAGame);
    }
    if !steam_installed(non_empty(&raw.state_flags).ok_or(Reject::MissingField)?) {
        return Err(Reject::NotInstalled);
    }
    let name = non_empty(&raw.name).ok_or(Reject::MissingField)?;
    let installdir = raw.installdir.as_deref().ok_or(Reject::MissingField)?;
    if !single_component(installdir) {
        return Err(Reject::BadPath);
    }
    let common = format!("{}\\steamapps\\common", library_dir.trim_end_matches('\\'));
    let dir = canon_dir(&format!("{common}\\{installdir}"), canon, policy)?;
    if !path_is_within(&common, &dir) {
        return Err(Reject::BadPath); // junction saindo de `steamapps\common`
    }
    Ok(DetectedGame {
        id: GameId::parse(&format!("steam:{app_id}")).map_err(|_| Reject::BadId)?,
        name: name.to_owned(),
        install_dir: Some(dir),
        launch: LaunchSpec::SteamUri { app_id },
        cover_source: None,
        icon_source: None,
    })
}

// ------------------------------------------------------------------------------------- Epic

/// Campos crus de um `.item` (campos desconhecidos ja foram ignorados pelo leitor).
#[derive(Debug, Clone, Default)]
pub struct EpicManifestRaw {
    pub app_name: Option<String>,
    pub main_game_app_name: Option<String>,
    pub catalog_namespace: Option<String>,
    pub catalog_item_id: Option<String>,
    pub display_name: Option<String>,
    pub install_location: Option<String>,
    pub launch_executable: Option<String>,
    pub incomplete: bool,
}

pub fn epic_game(
    raw: &EpicManifestRaw,
    policy: &InstallPathPolicy,
    canon: &dyn Fn(&str) -> Option<String>,
) -> Result<DetectedGame, Reject> {
    if raw.incomplete {
        return Err(Reject::Incomplete);
    }
    // Valor cru, sem aparar: "nada e sanitizado", o que veio e o que e conferido.
    let app_name = required(&raw.app_name)?;
    let namespace = required(&raw.catalog_namespace)?;
    let item_id = required(&raw.catalog_item_id)?;
    if ![app_name, namespace, item_id]
        .into_iter()
        .all(is_epic_component)
    {
        return Err(Reject::BadId);
    }
    // DLC: `MainGameAppName` ausente ou vazio = jogo.
    if non_empty(&raw.main_game_app_name).is_some_and(|main| main != app_name) {
        return Err(Reject::Dlc);
    }
    let location = raw
        .install_location
        .as_deref()
        .ok_or(Reject::MissingField)?;
    let dir = canon_dir(location, canon, policy)?;
    let rel = raw
        .launch_executable
        .as_deref()
        .ok_or(Reject::MissingField)?;
    // `LaunchExecutable` e relativo: absoluto, mesmo dentro da pasta, e manifesto fora do padrao.
    if rel.starts_with(['\\', '/']) || rel.as_bytes().get(1) == Some(&b':') {
        return Err(Reject::BadPath);
    }
    let exe = confined_exe(&dir, rel, canon, policy)?;
    let name = non_empty(&raw.display_name).unwrap_or(app_name);
    Ok(DetectedGame {
        id: GameId::parse(&format!("epic:{app_name}")).map_err(|_| Reject::BadId)?,
        name: name.to_owned(),
        install_dir: Some(dir),
        launch: LaunchSpec::EpicUri {
            namespace: namespace.to_owned(),
            catalog_item_id: item_id.to_owned(),
            app_name: app_name.to_owned(),
        },
        cover_source: None,
        icon_source: Some(exe),
    })
}

// -------------------------------------------------------------------------------------- GOG

/// Tarefa de `playTasks` de um `goggame-<id>.info`.
#[derive(Debug, Clone, Default)]
pub struct GogTaskRaw {
    pub category: Option<String>,
    pub is_primary: bool,
    pub kind: Option<String>,
    pub path: Option<String>,
    pub arguments: Option<String>,
    pub working_dir: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct GogInfoRaw {
    pub game_id: Option<String>,
    pub name: Option<String>,
    pub play_tasks: Vec<GogTaskRaw>,
}

/// Um jogo do registro `GOG.com\Games\<id>` e, se achado, o `.info` da pasta dele.
#[derive(Debug, Clone, Default)]
pub struct GogGameRaw {
    pub game_id: Option<String>,
    pub name: Option<String>,
    pub path: Option<String>,
    pub exe: Option<String>,
    pub working_dir: Option<String>,
    pub launch_param: Option<String>,
    pub info: Option<GogInfoRaw>,
}

const MAX_ARGS: usize = 32;
const MAX_ARG_CHARS: usize = 1024;

/// Divide a linha de argumentos pelas regras de `CommandLineToArgvW` (aspas e barras). O
/// resultado e uma **lista**: o `GameStarter` a escapa de volta (SEC-10). `None` se passar dos
/// limites ou tiver controle.
pub fn split_arguments(line: &str) -> Option<Vec<String>> {
    if line.chars().any(char::is_control) {
        return None;
    }
    let chars: Vec<char> = line.chars().collect();
    let mut args = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        while i < chars.len() && (chars[i] == ' ' || chars[i] == '\t') {
            i += 1;
        }
        if i >= chars.len() {
            break;
        }
        let mut cur = String::new();
        let mut in_quotes = false;
        while i < chars.len() {
            let c = chars[i];
            if c == '\\' {
                let start = i;
                while i < chars.len() && chars[i] == '\\' {
                    i += 1;
                }
                let n = i - start;
                if i < chars.len() && chars[i] == '"' {
                    cur.extend(std::iter::repeat('\\').take(n / 2));
                    if n % 2 == 1 {
                        cur.push('"');
                        i += 1;
                    }
                } else {
                    cur.extend(std::iter::repeat('\\').take(n));
                }
                continue;
            }
            if c == '"' {
                in_quotes = !in_quotes;
                i += 1;
                continue;
            }
            if !in_quotes && (c == ' ' || c == '\t') {
                break;
            }
            cur.push(c);
            i += 1;
        }
        if cur.chars().count() > MAX_ARG_CHARS || args.len() >= MAX_ARGS {
            return None;
        }
        args.push(cur);
    }
    Some(args)
}

/// Tarefa primaria do jogo: `isPrimary`, `category = "game"`, `type = "FileTask"`. `URLTask` nunca
/// e executada.
fn primary_task(info: &GogInfoRaw) -> Option<&GogTaskRaw> {
    info.play_tasks.iter().find(|t| {
        t.is_primary
            && t.category.as_deref() == Some("game")
            && t.kind.as_deref() == Some("FileTask")
    })
}

pub fn gog_game(
    raw: &GogGameRaw,
    policy: &InstallPathPolicy,
    canon: &dyn Fn(&str) -> Option<String>,
) -> Result<DetectedGame, Reject> {
    let id = raw.game_id.as_deref().ok_or(Reject::MissingField)?;
    if !is_gog_id(id) {
        return Err(Reject::BadId);
    }
    if let Some(info_id) = raw.info.as_ref().and_then(|i| i.game_id.as_deref()) {
        if info_id != id {
            return Err(Reject::BadId);
        }
    }
    let dir = canon_dir(
        raw.path.as_deref().ok_or(Reject::MissingField)?,
        canon,
        policy,
    )?;

    let (exe_rel, args, work_rel, info_name) = match &raw.info {
        // Com `.info`: so a tarefa primaria vale; sem ela o item e ignorado.
        Some(info) => {
            let task = primary_task(info).ok_or(Reject::NoPlayTask)?;
            let args = match non_empty(&task.arguments) {
                Some(line) => split_arguments(line).ok_or(Reject::NoPlayTask)?,
                None => Vec::new(),
            };
            (
                task.path.as_deref().ok_or(Reject::MissingField)?,
                args,
                non_empty(&task.working_dir),
                info.name.as_deref(),
            )
        }
        // Sem `.info`: `exe` e `workingDir` do registro (mesmas politicas).
        None => (
            raw.exe.as_deref().ok_or(Reject::MissingField)?,
            match non_empty(&raw.launch_param) {
                Some(line) => split_arguments(line).ok_or(Reject::NoPlayTask)?,
                None => Vec::new(),
            },
            non_empty(&raw.working_dir),
            None,
        ),
    };

    let exe = confined_exe(&dir, exe_rel, canon, policy)?;
    let working_dir = match work_rel {
        Some(w) => {
            let confined = if w == "." || normalize_local(w).is_ok_and(|n| n == dir) {
                dir.clone()
            } else {
                resolve_confined(&dir, w).map_err(|_| Reject::BadPath)?
            };
            let real = canon(&confined).ok_or(Reject::Missing)?;
            if !path_is_within(&dir, &real) {
                return Err(Reject::BadPath);
            }
            real
        }
        None => dir.clone(),
    };
    let name = non_empty(&raw.name)
        .or_else(|| info_name.map(str::trim).filter(|s| !s.is_empty()))
        .unwrap_or(id);
    Ok(DetectedGame {
        id: GameId::parse(&format!("gog:{id}")).map_err(|_| Reject::BadId)?,
        name: name.to_owned(),
        install_dir: Some(dir),
        launch: LaunchSpec::Exe {
            path: exe.clone(),
            args,
            working_dir: Some(working_dir),
        },
        cover_source: None,
        icon_source: Some(exe),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn policy() -> InstallPathPolicy {
        InstallPathPolicy::new(
            r"C:\Windows",
            [
                r"C:\Program Files".to_owned(),
                r"C:\Program Files (x86)".to_owned(),
                r"C:\Program Files (x86)\Steam".to_owned(),
                r"C:\Program Files\Sofa".to_owned(),
                r"C:\ProgramData".to_owned(),
            ],
        )
    }

    /// Tudo existe e nada e link: o caminho canonico e o proprio (so a forma vale).
    fn everything(p: &str) -> Option<String> {
        Some(p.to_owned())
    }

    fn steam(appid: &str, flags: &str, installdir: &str) -> SteamManifestRaw {
        SteamManifestRaw {
            appid: Some(appid.into()),
            name: Some("Jogo".into()),
            state_flags: Some(flags.into()),
            installdir: Some(installdir.into()),
        }
    }

    #[test]
    fn steam_aceita_jogo_instalado_e_monta_a_pasta() {
        let g = steam_game(
            &steam("620", "4", "Portal 2"),
            r"C:\Program Files (x86)\Steam",
            &policy(),
            &everything,
        )
        .unwrap();
        assert_eq!(g.id.to_string(), "steam:620");
        assert_eq!(
            g.install_dir.as_deref(),
            Some(r"C:\Program Files (x86)\Steam\steamapps\common\Portal 2")
        );
        assert_eq!(g.launch, LaunchSpec::SteamUri { app_id: 620 });
    }

    #[test]
    fn steam_state_flags_testa_o_bit_nao_a_igualdade() {
        for (flags, ok) in [
            ("4", true),
            ("6", true),
            ("1030", true),
            ("1", false),
            ("2", false),
            ("1026", false),
            ("0", false),
        ] {
            let r = steam_game(
                &steam("620", flags, "X"),
                r"D:\SteamLibrary",
                &policy(),
                &everything,
            );
            assert_eq!(r.is_ok(), ok, "StateFlags {flags}");
            if !ok {
                assert_eq!(r.unwrap_err(), Reject::NotInstalled);
            }
        }
        let r = steam_game(
            &steam("620", "abc", "X"),
            r"D:\SteamLibrary",
            &policy(),
            &everything,
        );
        assert_eq!(r.unwrap_err(), Reject::NotInstalled);
    }

    #[test]
    fn steam_rejeita_appid_e_installdir_maliciosos() {
        let lib = r"D:\SteamLibrary";
        for appid in [
            "10//-console",
            "x?action=foo&y",
            "a b",
            "99999999999",
            "",
            "4294967296",
        ] {
            let r = steam_game(&steam(appid, "4", "X"), lib, &policy(), &everything);
            assert!(
                matches!(r, Err(Reject::BadId | Reject::MissingField)),
                "appid {appid:?}: {r:?}"
            );
        }
        for dir in [
            r"..\..\..\Windows",
            "a/b",
            r"C:\Windows",
            "",
            ".",
            "..",
            "x:ads",
            "a\\b",
            "con\u{7}",
        ] {
            let r = steam_game(&steam("620", "4", dir), lib, &policy(), &everything);
            assert!(
                matches!(r, Err(Reject::BadPath | Reject::MissingField)),
                "installdir {dir:?}: {r:?}"
            );
        }
    }

    #[test]
    fn steam_filtra_nao_jogos_e_exige_nome_e_pasta_existente() {
        let lib = r"D:\SteamLibrary";
        assert_eq!(
            steam_game(
                &steam("228980", "4", "Steamworks Shared"),
                lib,
                &policy(),
                &everything
            )
            .unwrap_err(),
            Reject::NotAGame
        );
        let mut sem_nome = steam("620", "4", "X");
        sem_nome.name = None;
        assert_eq!(
            steam_game(&sem_nome, lib, &policy(), &everything).unwrap_err(),
            Reject::MissingField
        );
        // Unidade desconectada / pasta apagada (E-18): `canon` nao acha.
        let nada = |_: &str| None;
        assert_eq!(
            steam_game(&steam("620", "4", "X"), lib, &policy(), &nada).unwrap_err(),
            Reject::Missing
        );
    }

    #[test]
    fn steam_junction_para_fora_de_common_e_recusada() {
        let para_fora = |p: &str| {
            if p.ends_with(r"common\Link") {
                Some(r"D:\Outro\Lugar\Jogo".to_owned())
            } else {
                Some(p.to_owned())
            }
        };
        let r = steam_game(
            &steam("620", "4", "Link"),
            r"D:\SteamLibrary",
            &policy(),
            &para_fora,
        );
        assert_eq!(r.unwrap_err(), Reject::BadPath);
    }

    fn epic(app: &str, rel: &str, loc: &str) -> EpicManifestRaw {
        EpicManifestRaw {
            app_name: Some(app.into()),
            main_game_app_name: Some(app.into()),
            catalog_namespace: Some("ns".into()),
            catalog_item_id: Some("item".into()),
            display_name: Some("Jogo Epic".into()),
            install_location: Some(loc.into()),
            launch_executable: Some(rel.into()),
            incomplete: false,
        }
    }

    #[test]
    fn epic_aceita_jogo_e_monta_uri_com_os_tres_componentes() {
        let g = epic_game(
            &epic(
                "SampleGameA",
                r"SampleGameA\Binaries\Win64\SampleGameA.exe",
                r"C:\Program Files\Epic Games\SampleGameA",
            ),
            &policy(),
            &everything,
        )
        .unwrap();
        assert_eq!(g.id.to_string(), "epic:SampleGameA");
        assert_eq!(g.name, "Jogo Epic");
        assert!(
            matches!(&g.launch, LaunchSpec::EpicUri { namespace, catalog_item_id, app_name }
            if namespace == "ns" && catalog_item_id == "item" && app_name == "SampleGameA")
        );
        assert_eq!(
            g.icon_source.as_deref(),
            Some(
                r"C:\Program Files\Epic Games\SampleGameA\SampleGameA\Binaries\Win64\SampleGameA.exe"
            )
        );
    }

    #[test]
    fn epic_rejeita_ids_e_caminhos_maliciosos() {
        let ok_dir = r"C:\Program Files\Epic Games\X";
        for app in ["x?action=foo&y", "a b", "a/b", "", "a\u{0}b"] {
            let r = epic_game(&epic(app, "X.exe", ok_dir), &policy(), &everything);
            assert!(
                matches!(r, Err(Reject::BadId | Reject::MissingField)),
                "AppName {app:?}: {r:?}"
            );
        }
        let casos: &[(&str, &str, Reject)] = &[
            (r"Windows\System32\cmd.exe", r"C:\", Reject::BadInstallDir),
            (
                r"..\..\..\Windows\System32\cmd.exe",
                ok_dir,
                Reject::BadPath,
            ),
            (r"C:\Windows\System32\cmd.exe", ok_dir, Reject::BadPath),
            ("X.exe", r"\\127.0.0.1\c$\Jogo", Reject::BadPath),
            ("explorer.exe", r"C:\Windows", Reject::BadInstallDir),
            ("sofa.exe", r"C:\Program Files\Sofa", Reject::BadInstallDir),
            ("run.bat", ok_dir, Reject::BadExe),
            ("X.exe:ads", ok_dir, Reject::BadPath),
            ("", ok_dir, Reject::BadPath),
        ];
        for (rel, loc, esperado) in casos {
            let r = epic_game(&epic("X", rel, loc), &policy(), &everything);
            assert_eq!(r.unwrap_err(), *esperado, "{rel} em {loc}");
        }
    }

    #[test]
    fn epic_ignora_incompleto_dlc_e_item_sem_catalogo_e_trata_main_ausente_como_jogo() {
        let dir = r"C:\Program Files\Epic Games\X";
        let mut inc = epic("X", "X.exe", dir);
        inc.incomplete = true;
        assert_eq!(
            epic_game(&inc, &policy(), &everything).unwrap_err(),
            Reject::Incomplete
        );

        let mut dlc = epic("XDlc", "X.exe", dir);
        dlc.main_game_app_name = Some("X".into());
        assert_eq!(
            epic_game(&dlc, &policy(), &everything).unwrap_err(),
            Reject::Dlc
        );

        let mut sem_main = epic("X", "X.exe", dir);
        sem_main.main_game_app_name = None;
        assert!(epic_game(&sem_main, &policy(), &everything).is_ok());
        sem_main.main_game_app_name = Some(String::new());
        assert!(epic_game(&sem_main, &policy(), &everything).is_ok());

        let mut sem_cat = epic("X", "X.exe", dir);
        sem_cat.catalog_item_id = Some(String::new());
        assert_eq!(
            epic_game(&sem_cat, &policy(), &everything).unwrap_err(),
            Reject::MissingField
        );

        let mut sem_nome = epic("X", "X.exe", dir);
        sem_nome.display_name = None;
        assert_eq!(
            epic_game(&sem_nome, &policy(), &everything).unwrap().name,
            "X"
        );
    }

    #[test]
    fn epic_link_que_escapa_da_pasta_e_recusado() {
        let dir = r"C:\Program Files\Epic Games\X";
        let escapa = |p: &str| {
            if p.ends_with("link.exe") {
                Some(r"C:\Outro\Lugar\evil.exe".to_owned())
            } else {
                Some(p.to_owned())
            }
        };
        let r = epic_game(&epic("X", "link.exe", dir), &policy(), &escapa);
        assert_eq!(r.unwrap_err(), Reject::BadExe);
    }

    fn task(path: &str) -> GogTaskRaw {
        GogTaskRaw {
            category: Some("game".into()),
            is_primary: true,
            kind: Some("FileTask".into()),
            path: Some(path.into()),
            arguments: None,
            working_dir: None,
        }
    }

    fn gog_with_task(t: GogTaskRaw) -> GogGameRaw {
        GogGameRaw {
            game_id: Some("1000000001".into()),
            name: Some("Sample GOG".into()),
            path: Some(r"C:\GOG Games\Sample".into()),
            info: Some(GogInfoRaw {
                game_id: Some("1000000001".into()),
                name: Some("Sample GOG".into()),
                play_tasks: vec![t],
            }),
            ..GogGameRaw::default()
        }
    }

    #[test]
    fn gog_usa_a_tarefa_primaria_com_argumentos_como_lista() {
        let mut t = task(r"bin\game.exe");
        t.arguments = Some(r#"-windowed -nointro "C:\Users\Public\x y""#.into());
        t.working_dir = Some("bin".into());
        let mut raw = gog_with_task(t);
        // As outras tarefas (manual, URL) nao contam.
        raw.info.as_mut().unwrap().play_tasks.insert(
            0,
            GogTaskRaw {
                category: Some("document".into()),
                kind: Some("URLTask".into()),
                path: Some("https://exemplo.com".into()),
                ..GogTaskRaw::default()
            },
        );
        let g = gog_game(&raw, &policy(), &everything).unwrap();
        assert_eq!(g.id.to_string(), "gog:1000000001");
        match g.launch {
            LaunchSpec::Exe {
                path,
                args,
                working_dir,
            } => {
                assert_eq!(path, r"C:\GOG Games\Sample\bin\game.exe");
                assert_eq!(args, ["-windowed", "-nointro", r"C:\Users\Public\x y"]);
                assert_eq!(working_dir.as_deref(), Some(r"C:\GOG Games\Sample\bin"));
            }
            other => panic!("esperava Exe, veio {other:?}"),
        }
    }

    #[test]
    fn gog_rejeita_script_cmd_ads_unc_lnk_e_so_url() {
        let casos: &[(&str, Reject)] = &[
            ("start.bat", Reject::BadExe),
            (r"C:\Windows\System32\cmd.exe", Reject::BadPath),
            (r"..\..\Windows\System32\cmd.exe", Reject::BadPath),
            ("jogo.exe:evil", Reject::BadPath),
            (r"\\srv\share\g.exe", Reject::BadPath),
            ("atalho.lnk", Reject::BadExe),
        ];
        for (path, esperado) in casos {
            let r = gog_game(&gog_with_task(task(path)), &policy(), &everything);
            assert_eq!(r.unwrap_err(), *esperado, "{path}");
        }
        let mut so_url = gog_with_task(task("x.exe"));
        so_url.info.as_mut().unwrap().play_tasks[0].kind = Some("URLTask".into());
        assert_eq!(
            gog_game(&so_url, &policy(), &everything).unwrap_err(),
            Reject::NoPlayTask
        );
        let mut dlc = gog_with_task(task("x.exe"));
        dlc.info.as_mut().unwrap().play_tasks.clear();
        assert_eq!(
            gog_game(&dlc, &policy(), &everything).unwrap_err(),
            Reject::NoPlayTask
        );
        let mut id_ruim = gog_with_task(task("x.exe"));
        id_ruim.game_id = Some("abc".into());
        assert_eq!(
            gog_game(&id_ruim, &policy(), &everything).unwrap_err(),
            Reject::BadId
        );
        let mut id_divergente = gog_with_task(task("x.exe"));
        id_divergente.info.as_mut().unwrap().game_id = Some("999".into());
        assert_eq!(
            gog_game(&id_divergente, &policy(), &everything).unwrap_err(),
            Reject::BadId
        );
    }

    #[test]
    fn gog_sem_info_usa_o_registro_com_as_mesmas_politicas() {
        let reg = |exe: &str, path: &str| GogGameRaw {
            game_id: Some("1000000002".into()),
            name: Some("Sem info".into()),
            path: Some(path.into()),
            exe: Some(exe.into()),
            working_dir: Some(path.into()),
            ..GogGameRaw::default()
        };
        let g = gog_game(
            &reg(r"C:\GOG Games\SemInfo\g.exe", r"C:\GOG Games\SemInfo"),
            &policy(),
            &everything,
        )
        .unwrap();
        assert!(
            matches!(g.launch, LaunchSpec::Exe { ref path, .. } if path == r"C:\GOG Games\SemInfo\g.exe")
        );
        assert_eq!(
            gog_game(&reg(r"C:\g.exe", r"C:\"), &policy(), &everything).unwrap_err(),
            Reject::BadInstallDir
        );
        assert_eq!(
            gog_game(
                &reg(r"C:\Windows\System32\cmd.exe", r"C:\GOG Games\Fora"),
                &policy(),
                &everything
            )
            .unwrap_err(),
            Reject::BadPath
        );
    }

    #[test]
    fn argumentos_seguem_as_regras_do_windows() {
        let casos: &[(&str, &[&str])] = &[
            ("", &[]),
            ("   ", &[]),
            ("-a -b", &["-a", "-b"]),
            (r#"a "b c" d"#, &["a", "b c", "d"]),
            (r#""C:\x y\z" w"#, &[r"C:\x y\z", "w"]),
            (r#"a\"b"#, &[r#"a"b"#]),
            (r#""a\\" b"#, &[r"a\", "b"]),
            (r#"a\\b"#, &[r"a\\b"]),
            (r#"x"y z"w"#, &["xy zw"]),
        ];
        for (line, esperado) in casos {
            let esperado: Vec<String> = esperado.iter().map(|s| (*s).to_owned()).collect();
            assert_eq!(split_arguments(line), Some(esperado), "{line:?}");
        }
        assert_eq!(split_arguments("a\nb"), None);
        assert_eq!(split_arguments(&"a ".repeat(40)), None);
        assert_eq!(split_arguments(&"a".repeat(1025)), None);
    }
}
