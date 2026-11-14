//! Listagem do navegador de pastas (CA-19, CT-04): so unidades, pastas e `.exe`; pastas antes de
//! arquivos, ordem sem acento dentro de cada grupo, no maximo 2.000 entradas. O adapter lista o
//! disco; esta regra e pura.

use super::model::sort_key;

/// Limite de entradas da resposta (CT-04).
pub const MAX_DIR_ENTRIES: usize = 2_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum DirEntryKind {
    Drive,
    Dir,
    Exe,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DirEntry {
    /// Nome cru do arquivo ou pasta: a navegacao precisa dele de volta (CT-03).
    pub name: String,
    pub kind: DirEntryKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DirListing {
    /// `None` = lista de unidades.
    pub path: Option<String>,
    /// `None` na lista de unidades e na raiz de uma unidade.
    pub parent: Option<String>,
    pub entries: Vec<DirEntry>,
    pub truncated: bool,
}

/// Item lido do disco, ainda sem classificar.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawEntry {
    pub name: String,
    pub is_dir: bool,
}

fn is_exe_name(name: &str) -> bool {
    name.len() > 4 && name.to_ascii_lowercase().ends_with(".exe")
}

/// Pasta pai de um caminho `X:\a\b`: `X:\a`; `X:\a` -> `X:\`; raiz -> `None`.
pub fn parent_dir(path: &str) -> Option<String> {
    let trimmed = path.trim_end_matches('\\');
    let (head, _) = trimmed.rsplit_once('\\')?;
    if head.len() == 2 {
        Some(format!("{head}\\"))
    } else {
        Some(head.to_owned())
    }
}

/// Monta a listagem de uma pasta: descarta o que nao e pasta nem `.exe`, ordena e corta.
/// Nunca recebe nem devolve conteudo de arquivo.
pub fn dir_listing(path: &str, raw: impl IntoIterator<Item = RawEntry>) -> DirListing {
    let mut entries: Vec<(String, DirEntry)> = raw
        .into_iter()
        .filter_map(|r| {
            let kind = if r.is_dir {
                DirEntryKind::Dir
            } else if is_exe_name(&r.name) {
                DirEntryKind::Exe
            } else {
                return None;
            };
            Some((sort_key(&r.name), DirEntry { name: r.name, kind }))
        })
        .collect();
    entries.sort_by(|(ka, a), (kb, b)| {
        a.kind
            .cmp(&b.kind)
            .then_with(|| ka.cmp(kb))
            .then_with(|| a.name.cmp(&b.name))
    });
    let truncated = entries.len() > MAX_DIR_ENTRIES;
    entries.truncate(MAX_DIR_ENTRIES);
    DirListing {
        path: Some(path.to_owned()),
        parent: parent_dir(path),
        entries: entries.into_iter().map(|(_, e)| e).collect(),
        truncated,
    }
}

/// Lista de unidades locais (`C:\`, `D:\`...), ja filtradas pelo adapter por `DRIVE_FIXED` e
/// `DRIVE_REMOVABLE`.
pub fn drives_listing(mut drives: Vec<String>) -> DirListing {
    drives.sort();
    drives.dedup();
    DirListing {
        path: None,
        parent: None,
        entries: drives
            .into_iter()
            .map(|name| DirEntry {
                name,
                kind: DirEntryKind::Drive,
            })
            .collect(),
        truncated: false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn raw(name: &str, is_dir: bool) -> RawEntry {
        RawEntry {
            name: name.to_owned(),
            is_dir,
        }
    }

    #[test]
    fn so_pastas_e_exe_pastas_antes_ordem_sem_acento() {
        let l = dir_listing(
            r"D:\Jogos",
            vec![
                raw("zeta.exe", false),
                raw("leia.txt", false),
                raw("Atalho.lnk", false),
                raw("run.bat", false),
                raw("\u{c1}gua", true),
                raw("beta", true),
                raw("Alfa.EXE", false),
                raw(".exe", false), // sem nome: nao e um executavel
            ],
        );
        let nomes: Vec<(&str, DirEntryKind)> = l
            .entries
            .iter()
            .map(|e| (e.name.as_str(), e.kind))
            .collect();
        assert_eq!(
            nomes,
            [
                ("\u{c1}gua", DirEntryKind::Dir),
                ("beta", DirEntryKind::Dir),
                ("Alfa.EXE", DirEntryKind::Exe),
                ("zeta.exe", DirEntryKind::Exe),
            ]
        );
        assert_eq!(l.path.as_deref(), Some(r"D:\Jogos"));
        assert_eq!(l.parent.as_deref(), Some(r"D:\"));
        assert!(!l.truncated);
    }

    #[test]
    fn corta_em_2000_com_truncated_e_mantem_pastas_primeiro() {
        let mut v: Vec<RawEntry> = (0..2500)
            .map(|i| raw(&format!("f{i:04}.exe"), false))
            .collect();
        v.push(raw("zzz", true));
        let l = dir_listing(r"C:\Grande", v);
        assert_eq!(l.entries.len(), MAX_DIR_ENTRIES);
        assert!(l.truncated);
        assert_eq!(l.entries[0].kind, DirEntryKind::Dir); // a pasta sobrevive ao corte
        let exato = dir_listing(
            r"C:\Exato",
            (0..MAX_DIR_ENTRIES).map(|i| raw(&format!("f{i:04}.exe"), false)),
        );
        assert!(!exato.truncated);
    }

    #[test]
    fn pai_da_raiz_e_nulo_e_o_de_primeiro_nivel_e_a_raiz() {
        assert_eq!(parent_dir(r"C:\"), None);
        assert_eq!(parent_dir(r"C:\Jogos").as_deref(), Some(r"C:\"));
        assert_eq!(parent_dir(r"C:\Jogos\X\").as_deref(), Some(r"C:\Jogos"));
        assert_eq!(parent_dir(r"C:\Jogos\X").as_deref(), Some(r"C:\Jogos"));
    }

    #[test]
    fn unidades_sem_repeticao() {
        let l = drives_listing(vec![r"D:\".into(), r"C:\".into(), r"D:\".into()]);
        assert_eq!(l.path, None);
        assert_eq!(l.parent, None);
        let nomes: Vec<&str> = l.entries.iter().map(|e| e.name.as_str()).collect();
        assert_eq!(nomes, [r"C:\", r"D:\"]);
        assert!(l.entries.iter().all(|e| e.kind == DirEntryKind::Drive));
    }
}
