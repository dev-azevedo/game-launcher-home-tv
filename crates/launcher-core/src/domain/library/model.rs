//! Jogo da biblioteca: fusao do que as lojas detectam com a camada do usuario (ADR-0004),
//! unicidade (RN-03), ordem (RN-05) e nome exibido (RN-22). Puro.

use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};

use unicode_normalization::char::is_combining_mark;
use unicode_normalization::UnicodeNormalization;

use super::display_name::normalize_display_name;
use super::id::GameId;
use super::launch::LaunchSpec;
use super::stored::{ManualGameRecord, OverlayEntry};

/// Jogo que uma loja detectou agora (ja validado: IDs, caminhos e executavel passaram pelas
/// politicas). Nunca e persistido como verdade (SEC-18).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DetectedGame {
    pub id: GameId,
    /// Nome da loja, como veio (a normalizacao e da fusao).
    pub name: String,
    /// Pasta de instalacao canonica (SEC-16).
    pub install_dir: Option<String>,
    pub launch: LaunchSpec,
    /// Capa local da loja (arquivo de terceiro: so o `CoverCache` le).
    pub cover_source: Option<String>,
    /// Icone do `.exe` quando nao ha capa (CA-56, CA-57).
    pub icon_source: Option<String>,
}

/// Jogo pronto para a UI e para o lancamento.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LibraryEntry {
    pub id: GameId,
    /// Nome da origem (loja ou arquivo), normalizado (SEC-07).
    pub original_name: String,
    /// Nome personalizado, normalizado e nao vazio (RN-22).
    pub custom_name: Option<String>,
    pub hidden: bool,
    pub last_played_at: Option<i64>,
    pub launch: LaunchSpec,
    pub install_dir: Option<String>,
    pub cover_source: Option<String>,
    pub icon_source: Option<String>,
}

impl LibraryEntry {
    /// RN-22: o nome personalizado vence o da origem.
    pub fn display_name(&self) -> &str {
        self.custom_name.as_deref().unwrap_or(&self.original_name)
    }
}

/// Chave de ordem alfabetica sem diferenciar caixa nem acento (RN-05): NFD, sem marcas
/// combinantes, em minusculas.
pub fn sort_key(name: &str) -> String {
    name.nfd()
        .filter(|c| !is_combining_mark(*c))
        .flat_map(char::to_lowercase)
        .collect()
}

fn compare(a: &LibraryEntry, b: &LibraryEntry) -> Ordering {
    // Jogados primeiro, do mais recente ao mais antigo; depois os nunca jogados, A-Z.
    match (a.last_played_at, b.last_played_at) {
        (Some(x), Some(y)) => y.cmp(&x),
        (Some(_), None) => Ordering::Less,
        (None, Some(_)) => Ordering::Greater,
        (None, None) => Ordering::Equal,
    }
    .then_with(|| sort_key(a.display_name()).cmp(&sort_key(b.display_name())))
    .then_with(|| a.id.cmp(&b.id))
}

/// Ordena pela RN-05 (estavel e deterministica: empate cai no `GameId`).
pub fn sort_entries(entries: &mut [LibraryEntry]) {
    entries.sort_by(compare);
}

fn manual_entry(record: &ManualGameRecord) -> LibraryEntry {
    let dir = record
        .path
        .rsplit_once('\\')
        .map(|(d, _)| d.to_owned())
        .filter(|d| !d.is_empty());
    LibraryEntry {
        id: record.id.clone(),
        original_name: fallback_name(normalize_display_name(&record.name), &record.id),
        custom_name: None,
        hidden: false,
        last_played_at: None,
        launch: LaunchSpec::Exe {
            path: record.path.clone(),
            args: Vec::new(), // AF-03: jogo manual nunca leva argumento
            working_dir: dir.clone(),
        },
        install_dir: dir,
        cover_source: None,
        icon_source: Some(record.path.clone()),
    }
}

/// Nome vazio depois de normalizar nao pode chegar a UI (`name` e 1..256): cai na chave do ID.
fn fallback_name(name: String, id: &GameId) -> String {
    if name.is_empty() {
        id.key().to_owned()
    } else {
        name
    }
}

/// Une o detectado agora, os jogos manuais e a camada do usuario. Um item por `GameId` (RN-03,
/// E-12: a primeira ocorrencia vale); o mesmo jogo em lojas diferentes sao itens separados
/// (RN-25). Oculto e nome personalizado vem do overlay e sobrevivem a jogo que sumiu e voltou
/// (E-18): o overlay de quem nao foi detectado fica no arquivo e nao entra na lista. Devolve
/// todos (inclusive ocultos) na ordem da RN-05.
pub fn merge_library(
    detected: Vec<DetectedGame>,
    manual: &[ManualGameRecord],
    overlay: &BTreeMap<GameId, OverlayEntry>,
) -> Vec<LibraryEntry> {
    let mut seen: BTreeSet<GameId> = BTreeSet::new();
    let mut out = Vec::with_capacity(detected.len() + manual.len());

    for d in detected {
        if !seen.insert(d.id.clone()) {
            continue;
        }
        let original_name = fallback_name(normalize_display_name(&d.name), &d.id);
        out.push(LibraryEntry {
            id: d.id,
            original_name,
            custom_name: None,
            hidden: false,
            last_played_at: None,
            launch: d.launch,
            install_dir: d.install_dir,
            cover_source: d.cover_source,
            icon_source: d.icon_source,
        });
    }
    for m in manual {
        if seen.insert(m.id.clone()) {
            out.push(manual_entry(m));
        }
    }

    for e in &mut out {
        if let Some(o) = overlay.get(&e.id) {
            e.custom_name = o
                .custom_name
                .as_deref()
                .map(normalize_display_name)
                .filter(|n| !n.is_empty());
            e.hidden = o.hidden;
            e.last_played_at = o.last_played_at;
        }
    }
    sort_entries(&mut out);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id(s: &str) -> GameId {
        GameId::parse(s).unwrap()
    }

    fn det(game: &str, name: &str) -> DetectedGame {
        let id = id(game);
        let launch = match id.origin() {
            super::super::Origin::Steam => LaunchSpec::SteamUri {
                app_id: id.key().parse().unwrap(),
            },
            _ => LaunchSpec::Exe {
                path: format!(r"C:\Jogos\{name}\g.exe"),
                args: Vec::new(),
                working_dir: None,
            },
        };
        DetectedGame {
            id,
            name: name.to_owned(),
            install_dir: Some(format!(r"C:\Jogos\{name}")),
            launch,
            cover_source: None,
            icon_source: None,
        }
    }

    fn manual(path: &str, name: &str) -> ManualGameRecord {
        ManualGameRecord {
            id: GameId::manual_from_canonical_path(path),
            name: name.to_owned(),
            path: path.to_owned(),
        }
    }

    fn ids(v: &[LibraryEntry]) -> Vec<String> {
        v.iter().map(|e| e.id.to_string()).collect()
    }

    #[test]
    fn mesmo_jogo_em_duas_pastas_da_steam_e_um_item() {
        let lib = merge_library(
            vec![
                det("steam:620", "Portal 2"),
                det("steam:620", "Portal 2 (copia)"),
            ],
            &[],
            &BTreeMap::new(),
        );
        assert_eq!(ids(&lib), ["steam:620"]);
        assert_eq!(lib[0].original_name, "Portal 2"); // a primeira ocorrencia vale
    }

    #[test]
    fn steam_e_epic_do_mesmo_jogo_sao_dois_itens() {
        let lib = merge_library(
            vec![
                det("steam:620", "Portal 2"),
                det("epic:Portal2", "Portal 2"),
            ],
            &[],
            &BTreeMap::new(),
        );
        assert_eq!(lib.len(), 2);
        assert_eq!(lib[0].display_name(), lib[1].display_name());
    }

    #[test]
    fn manual_entra_com_nome_icone_e_sem_argumentos() {
        let m = manual(r"C:\Jogos\Foo\foo.exe", "foo");
        let lib = merge_library(Vec::new(), std::slice::from_ref(&m), &BTreeMap::new());
        assert_eq!(lib.len(), 1);
        assert_eq!(lib[0].install_dir.as_deref(), Some(r"C:\Jogos\Foo"));
        assert_eq!(lib[0].icon_source.as_deref(), Some(r"C:\Jogos\Foo\foo.exe"));
        match &lib[0].launch {
            LaunchSpec::Exe { args, path, .. } => {
                assert!(args.is_empty());
                assert_eq!(path, r"C:\Jogos\Foo\foo.exe");
            }
            other => panic!("esperava Exe, veio {other:?}"),
        }
        // Dois manuais com o mesmo caminho (ID igual) viram um.
        let lib = merge_library(Vec::new(), &[m.clone(), m], &BTreeMap::new());
        assert_eq!(lib.len(), 1);
    }

    #[test]
    fn oculto_nao_volta_apos_novo_scan_e_nome_personalizado_vence() {
        let mut overlay = BTreeMap::new();
        overlay.insert(
            id("steam:620"),
            OverlayEntry {
                custom_name: Some("  Portal 2 (coop)\u{202E} ".into()),
                hidden: true,
                last_played_at: Some(10),
            },
        );
        // Duas varreduras seguidas com o mesmo overlay: continua oculto, nome mantido (RN-22).
        for _ in 0..2 {
            let lib = merge_library(vec![det("steam:620", "Portal 2")], &[], &overlay);
            assert!(lib[0].hidden);
            assert_eq!(lib[0].display_name(), "Portal 2 (coop)");
            assert_eq!(lib[0].original_name, "Portal 2");
            assert_eq!(lib[0].last_played_at, Some(10));
        }
    }

    #[test]
    fn nome_personalizado_vazio_depois_de_normalizar_volta_ao_original() {
        let mut overlay = BTreeMap::new();
        overlay.insert(
            id("steam:620"),
            OverlayEntry {
                custom_name: Some("\u{202E}\u{0}".into()),
                ..OverlayEntry::default()
            },
        );
        let lib = merge_library(vec![det("steam:620", "Portal 2")], &[], &overlay);
        assert_eq!(lib[0].custom_name, None);
        assert_eq!(lib[0].display_name(), "Portal 2");
    }

    #[test]
    fn jogo_de_unidade_desconectada_some_e_volta_com_o_estado_preservado() {
        let mut overlay = BTreeMap::new();
        overlay.insert(
            id("steam:440"),
            OverlayEntry {
                custom_name: Some("TF2".into()),
                hidden: true,
                last_played_at: Some(99),
            },
        );
        // Unidade desconectada: a loja nao detecta o jogo. Some (E-18); o overlay fica no arquivo.
        let sem = merge_library(vec![det("steam:620", "Portal 2")], &[], &overlay);
        assert_eq!(ids(&sem), ["steam:620"]);
        assert!(overlay.contains_key(&id("steam:440")));
        // Reconectou: volta com nome, oculto e data.
        let com = merge_library(
            vec![
                det("steam:620", "Portal 2"),
                det("steam:440", "Team Fortress 2"),
            ],
            &[],
            &overlay,
        );
        let tf2 = com.iter().find(|e| e.id == id("steam:440")).unwrap();
        assert!(tf2.hidden);
        assert_eq!(tf2.display_name(), "TF2");
        assert_eq!(tf2.last_played_at, Some(99));
    }

    #[test]
    fn ordem_recentes_primeiro_depois_a_z_sem_acento_nem_caixa() {
        let mut overlay = BTreeMap::new();
        overlay.insert(
            id("steam:3"),
            OverlayEntry {
                last_played_at: Some(100),
                ..OverlayEntry::default()
            },
        );
        overlay.insert(
            id("steam:4"),
            OverlayEntry {
                last_played_at: Some(200),
                ..OverlayEntry::default()
            },
        );
        overlay.insert(
            id("steam:5"),
            OverlayEntry {
                custom_name: Some("Zeta".into()),
                ..OverlayEntry::default()
            },
        );
        let lib = merge_library(
            vec![
                det("steam:1", "\u{c9}pico"), // Epico
                det("steam:2", "abacaxi"),
                det("steam:3", "Mais antigo"),
                det("steam:4", "Mais novo"),
                det("steam:5", "Aaa (renomeado para Zeta)"),
                det("steam:6", "Eclipse"),
                det("steam:7", "\u{c1}rvore"), // Arvore
            ],
            &[],
            &overlay,
        );
        // Jogados: 4 (200) e 3 (100). Nunca jogados: abacaxi, Arvore, Eclipse, Epico, Zeta.
        assert_eq!(
            ids(&lib),
            ["steam:4", "steam:3", "steam:2", "steam:7", "steam:6", "steam:1", "steam:5"]
        );
    }

    #[test]
    fn empate_de_nome_desempata_pelo_id() {
        let lib = merge_library(
            vec![
                det("steam:9", "Igual"),
                det("steam:2", "igual"),
                det("epic:X", "IGUAL"),
            ],
            &[],
            &BTreeMap::new(),
        );
        // Ordem deterministica: sort por (chave do nome, GameId). Origin: Steam < Epic.
        assert_eq!(ids(&lib), ["steam:2", "steam:9", "epic:X"]);
    }

    #[test]
    fn nome_vazio_da_loja_cai_na_chave() {
        let lib = merge_library(
            vec![det("steam:620", "\u{202E}\u{0}")],
            &[],
            &BTreeMap::new(),
        );
        assert_eq!(lib[0].display_name(), "620");
    }
}
