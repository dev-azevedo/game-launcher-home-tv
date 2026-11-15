//! BE-46 / ARQ-BUG-08 (SEC-20, SEC-23): o `sofa.exe` precisa sair com **exatamente um**
//! `RT_MANIFEST` (ID 1), com `asInvoker`, `uiAccess="false"` e Common Controls v6.
//!
//! O teste le o diretorio de recursos do PE (tipo 24 -> ID -> idioma), em vez de procurar texto
//! no binario: procurar texto passa mesmo com dois manifestos, inclusive se o ativo fosse o
//! padrao do MinGW (sem Common Controls v6, onde `TaskDialogIndirect` nao carrega).
//!
//! - MSVC: dois manifestos (ou qualquer desvio) reprova. E a trava do BE-39 (release).
//! - `-gnu`: o linker do MinGW soma o manifesto padrao dele ao nosso (aviso ".rsrc merge failure").
//!   O build `-gnu` nunca e distribuido (ADR-0001), entao so avisa; os manifestos presentes ainda
//!   nao podem pedir elevacao nem `uiAccess`.
//!
//! `SOFA_MANIFEST_EXE=<caminho>` aponta o teste para outro `sofa.exe` (o do release MSVC, que o
//! `scripts/ci.ps1 -Release` passa).

use std::fs;

/// Um recurso `RT_MANIFEST` achado no diretorio de recursos do PE.
#[derive(Debug, Clone)]
struct ManifestResource {
    id: u32,
    lang: u32,
    text: String,
}

const RT_MANIFEST: u32 = 24;

fn u16_at(b: &[u8], o: usize) -> Result<u32, String> {
    b.get(o..o + 2)
        .map(|s| u32::from(u16::from_le_bytes([s[0], s[1]])))
        .ok_or_else(|| format!("PE truncado em {o:#x}"))
}

fn u32_at(b: &[u8], o: usize) -> Result<u32, String> {
    b.get(o..o + 4)
        .map(|s| u32::from_le_bytes([s[0], s[1], s[2], s[3]]))
        .ok_or_else(|| format!("PE truncado em {o:#x}"))
}

struct Section {
    va: u32,
    vsize: u32,
    raw_ptr: u32,
    raw_size: u32,
}

fn rva_to_offset(sections: &[Section], rva: u32) -> Result<usize, String> {
    for s in sections {
        let span = s.vsize.max(s.raw_size);
        if rva >= s.va && rva < s.va + span {
            return Ok((rva - s.va + s.raw_ptr) as usize);
        }
    }
    Err(format!("RVA {rva:#x} fora das secoes"))
}

/// Entradas de um `IMAGE_RESOURCE_DIRECTORY` em `base + dir_off`: `(nome_ou_id, destino, e_pasta)`.
/// Entradas por nome (bit 31 do primeiro campo) vem com o id `u32::MAX`.
fn dir_entries(rsrc: &[u8], dir_off: usize) -> Result<Vec<(u32, usize, bool)>, String> {
    let named = u16_at(rsrc, dir_off + 12)? as usize;
    let ids = u16_at(rsrc, dir_off + 14)? as usize;
    let mut out = Vec::new();
    for i in 0..(named + ids) {
        let e = dir_off + 16 + i * 8;
        let name = u32_at(rsrc, e)?;
        let target = u32_at(rsrc, e + 4)?;
        let id = if name & 0x8000_0000 != 0 {
            u32::MAX
        } else {
            name
        };
        out.push((
            id,
            (target & 0x7FFF_FFFF) as usize,
            target & 0x8000_0000 != 0,
        ));
    }
    Ok(out)
}

/// Todos os `RT_MANIFEST` do PE (o conteudo, como texto).
fn manifests(exe: &[u8]) -> Result<Vec<ManifestResource>, String> {
    if exe.get(0..2) != Some(b"MZ") {
        return Err("sem cabecalho MZ".into());
    }
    let pe = u32_at(exe, 0x3C)? as usize;
    if exe.get(pe..pe + 4) != Some(b"PE\0\0") {
        return Err("sem assinatura PE".into());
    }
    let coff = pe + 4;
    let nsections = u16_at(exe, coff + 2)? as usize;
    let opt_size = u16_at(exe, coff + 16)? as usize;
    let opt = coff + 20;
    let dd_off = match u16_at(exe, opt)? {
        0x20B => opt + 112, // PE32+
        0x10B => opt + 96,  // PE32
        m => return Err(format!("magic do optional header desconhecido: {m:#x}")),
    };
    let rsrc_rva = u32_at(exe, dd_off + 2 * 8)?;
    let rsrc_size = u32_at(exe, dd_off + 2 * 8 + 4)?;
    if rsrc_rva == 0 || rsrc_size == 0 {
        return Ok(Vec::new());
    }
    let sh = opt + opt_size;
    let mut sections = Vec::new();
    for i in 0..nsections {
        let s = sh + i * 40;
        sections.push(Section {
            vsize: u32_at(exe, s + 8)?,
            va: u32_at(exe, s + 12)?,
            raw_size: u32_at(exe, s + 16)?,
            raw_ptr: u32_at(exe, s + 20)?,
        });
    }
    let base = rva_to_offset(&sections, rsrc_rva)?;
    let rsrc = exe
        .get(base..base + rsrc_size as usize)
        .ok_or("diretorio de recursos fora do arquivo")?;

    let mut found = Vec::new();
    for (ty, ty_off, ty_dir) in dir_entries(rsrc, 0)? {
        if ty != RT_MANIFEST || !ty_dir {
            continue;
        }
        for (id, id_off, id_dir) in dir_entries(rsrc, ty_off)? {
            if !id_dir {
                return Err("RT_MANIFEST sem nivel de idioma".into());
            }
            for (lang, data_off, is_dir) in dir_entries(rsrc, id_off)? {
                if is_dir {
                    return Err("RT_MANIFEST com nivel extra".into());
                }
                let data_rva = u32_at(rsrc, data_off)?;
                let size = u32_at(rsrc, data_off + 4)? as usize;
                let off = rva_to_offset(&sections, data_rva)?;
                let raw = exe
                    .get(off..off + size)
                    .ok_or("dados do manifesto fora do arquivo")?;
                found.push(ManifestResource {
                    id,
                    lang,
                    text: String::from_utf8_lossy(raw).into_owned(),
                });
            }
        }
    }
    Ok(found)
}

/// Um manifesto isolado serve ao Sofa? (`asInvoker`, sem `uiAccess`, Common Controls v6).
fn check_content(m: &ManifestResource) -> Result<(), String> {
    let t = &m.text;
    if t.contains("requireAdministrator") || t.contains("highestAvailable") {
        return Err("pede elevacao (SEC-20)".into());
    }
    if t.contains(r#"uiAccess="true""#) {
        return Err("uiAccess=true (SEC-20, SEC-28)".into());
    }
    if !t.contains(r#"requestedExecutionLevel level="asInvoker" uiAccess="false""#) {
        return Err(r#"sem requestedExecutionLevel level="asInvoker" uiAccess="false""#.into());
    }
    if !t.contains("Microsoft.Windows.Common-Controls") || !t.contains(r#"version="6.0.0.0""#) {
        return Err("sem Common Controls v6 (TaskDialogIndirect nao carrega)".into());
    }
    Ok(())
}

/// A regra do ARQ-BUG-08: exatamente um `RT_MANIFEST`, ID 1, e ele serve.
fn check_single(found: &[ManifestResource]) -> Result<(), String> {
    match found {
        [] => Err("nenhum RT_MANIFEST no exe".into()),
        [one] => {
            if one.id != 1 {
                return Err(format!("o RT_MANIFEST tem ID {}, esperado 1", one.id));
            }
            check_content(one)
        }
        many => Err(format!(
            "{} RT_MANIFEST no exe (esperado exatamente 1): {}",
            many.len(),
            many.iter()
                .map(|m| format!("id {} idioma {} ({} bytes)", m.id, m.lang, m.text.len()))
                .collect::<Vec<_>>()
                .join("; ")
        )),
    }
}

fn exe_under_test() -> Vec<u8> {
    let path = std::env::var("SOFA_MANIFEST_EXE")
        .ok()
        .filter(|p| !p.trim().is_empty())
        .unwrap_or_else(|| env!("CARGO_BIN_EXE_sofa").to_string());
    fs::read(&path).unwrap_or_else(|e| panic!("{path}: {e}"))
}

#[test]
fn sofa_exe_tem_exatamente_um_manifesto_valido() {
    let found = manifests(&exe_under_test()).expect("ler o diretorio de recursos do sofa.exe");
    let verdict = check_single(&found);
    if cfg!(target_env = "msvc") {
        // Trava do BE-39: no MSVC o manifesto do MinGW nao entra; dois manifestos = reprovado.
        verdict.unwrap_or_else(|e| panic!("ARQ-BUG-08 (MSVC): {e}"));
        return;
    }
    // `-gnu`: so avisa do que e artefato do MinGW, mas nenhum manifesto presente pode elevar.
    if let Err(e) = &verdict {
        eprintln!("AVISO ARQ-BUG-08 (-gnu, nao distribuido): {e}");
    }
    assert!(!found.is_empty(), "nenhum RT_MANIFEST no sofa.exe");
    assert!(
        found.iter().any(|m| check_content(m).is_ok()),
        "nenhum RT_MANIFEST do sofa.exe passa em asInvoker + uiAccess=false + Common Controls v6"
    );
    for m in &found {
        assert!(
            !m.text.contains("requireAdministrator")
                && !m.text.contains("highestAvailable")
                && !m.text.contains(r#"uiAccess="true""#),
            "RT_MANIFEST id {} pede elevacao ou uiAccess (SEC-20)",
            m.id
        );
    }
}

// --- o proprio teste precisa conseguir reprovar: PE sintetico com 1, 2 e manifestos ruins ---

const OURS: &str = r#"<assembly><dependency><dependentAssembly><assemblyIdentity name="Microsoft.Windows.Common-Controls" version="6.0.0.0"/></dependentAssembly></dependency><trustInfo><security><requestedPrivileges><requestedExecutionLevel level="asInvoker" uiAccess="false"/></requestedPrivileges></security></trustInfo></assembly>"#;
const MINGW: &str = r#"<assembly><trustInfo><security><requestedPrivileges><requestedExecutionLevel level="asInvoker" uiAccess="false"/></requestedPrivileges></security></trustInfo></assembly>"#;

/// PE32+ minimo com uma secao `.rsrc` (RVA 0x1000, arquivo em 0x400) com os manifestos
/// `(id, idioma, texto)` sob o tipo 24.
fn synthetic_pe(items: &[(u32, u32, &str)]) -> Vec<u8> {
    // .rsrc: raiz -> tipo 24 -> ids -> idiomas -> entradas de dados -> textos.
    let mut ids: Vec<u32> = items.iter().map(|i| i.0).collect();
    ids.dedup();
    let mut rs: Vec<u8> = Vec::new();
    let put32 = |v: &mut Vec<u8>, x: u32| v.extend_from_slice(&x.to_le_bytes());
    let dir = |v: &mut Vec<u8>, n_ids: u16| {
        v.extend_from_slice(&[0; 12]);
        v.extend_from_slice(&0u16.to_le_bytes());
        v.extend_from_slice(&n_ids.to_le_bytes());
    };
    // tamanhos para calcular os deslocamentos
    let root = 0usize;
    let ty_dir = root + 16 + 8;
    let id_dirs_start = ty_dir + 16 + 8 * ids.len();
    let mut id_dir_off = Vec::new();
    let mut cur = id_dirs_start;
    for id in &ids {
        id_dir_off.push(cur);
        let n = items.iter().filter(|i| i.0 == *id).count();
        cur += 16 + 8 * n;
    }
    let data_entries_start = cur;
    let texts_start = data_entries_start + 16 * items.len();

    dir(&mut rs, 1);
    put32(&mut rs, RT_MANIFEST);
    put32(&mut rs, 0x8000_0000 | ty_dir as u32);
    dir(&mut rs, ids.len() as u16);
    for (i, id) in ids.iter().enumerate() {
        put32(&mut rs, *id);
        put32(&mut rs, 0x8000_0000 | id_dir_off[i] as u32);
    }
    let mut data_idx = 0usize;
    for id in &ids {
        let mine: Vec<_> = items.iter().filter(|i| i.0 == *id).collect();
        dir(&mut rs, mine.len() as u16);
        for it in mine {
            put32(&mut rs, it.1);
            put32(&mut rs, (data_entries_start + 16 * data_idx) as u32);
            data_idx += 1;
        }
    }
    let mut text_off = texts_start;
    let mut texts = Vec::new();
    for it in items {
        put32(&mut rs, 0x1000 + text_off as u32);
        put32(&mut rs, it.2.len() as u32);
        put32(&mut rs, 0);
        put32(&mut rs, 0);
        texts.extend_from_slice(it.2.as_bytes());
        text_off += it.2.len();
    }
    rs.extend_from_slice(&texts);

    let mut pe = vec![0u8; 0x400];
    pe[0..2].copy_from_slice(b"MZ");
    pe[0x3C..0x40].copy_from_slice(&0x80u32.to_le_bytes());
    pe[0x80..0x84].copy_from_slice(b"PE\0\0");
    let coff = 0x84;
    pe[coff + 2..coff + 4].copy_from_slice(&1u16.to_le_bytes()); // 1 secao
    pe[coff + 16..coff + 18].copy_from_slice(&240u16.to_le_bytes()); // optional header PE32+
    let opt = coff + 20;
    pe[opt..opt + 2].copy_from_slice(&0x20Bu16.to_le_bytes());
    let dd = opt + 112 + 2 * 8;
    pe[dd..dd + 4].copy_from_slice(&0x1000u32.to_le_bytes());
    pe[dd + 4..dd + 8].copy_from_slice(&(rs.len() as u32).to_le_bytes());
    let sh = opt + 240;
    pe[sh..sh + 6].copy_from_slice(b".rsrc\0");
    pe[sh + 8..sh + 12].copy_from_slice(&(rs.len() as u32).to_le_bytes());
    pe[sh + 12..sh + 16].copy_from_slice(&0x1000u32.to_le_bytes());
    pe[sh + 16..sh + 20].copy_from_slice(&(rs.len() as u32).to_le_bytes());
    pe[sh + 20..sh + 24].copy_from_slice(&0x400u32.to_le_bytes());
    pe.extend_from_slice(&rs);
    pe
}

#[test]
fn um_manifesto_id_1_valido_passa() {
    let found = manifests(&synthetic_pe(&[(1, 1033, OURS)])).unwrap();
    assert_eq!(found.len(), 1);
    assert_eq!((found[0].id, found[0].lang), (1, 1033));
    check_single(&found).unwrap();
}

#[test]
fn dois_manifestos_reprovam_como_no_gnu() {
    // O caso real do `-gnu`: o nosso (Common Controls v6) mais o padrao do MinGW, ID 1 e 1033.
    // Dois idiomas sob o mesmo ID, que e a forma que o linker deixa.
    let found = manifests(&synthetic_pe(&[(1, 1033, OURS), (1, 0, MINGW)])).unwrap();
    assert_eq!(found.len(), 2);
    let err = check_single(&found).unwrap_err();
    assert!(err.contains("2 RT_MANIFEST"), "{err}");
    // e com IDs diferentes
    let found = manifests(&synthetic_pe(&[(1, 1033, OURS), (2, 1033, MINGW)])).unwrap();
    assert!(check_single(&found).is_err());
}

#[test]
fn manifesto_unico_ruim_reprova() {
    let bad = |text: &str, id: u32| {
        check_single(&manifests(&synthetic_pe(&[(id, 1033, text)])).unwrap()).unwrap_err()
    };
    assert!(bad(MINGW, 1).contains("Common Controls"));
    assert!(bad(&OURS.replace("asInvoker", "requireAdministrator"), 1).contains("elevacao"));
    assert!(bad(
        &OURS.replace(r#"uiAccess="false""#, r#"uiAccess="true""#),
        1
    )
    .contains("uiAccess"));
    assert!(bad(OURS, 2).contains("ID 2"));
}

#[test]
fn pe_sem_manifesto_reprova() {
    let mut pe = synthetic_pe(&[(1, 1033, OURS)]);
    // zera o diretorio de recursos
    let dd = 0x84 + 20 + 112 + 2 * 8;
    pe[dd..dd + 8].copy_from_slice(&[0; 8]);
    let found = manifests(&pe).unwrap();
    assert!(found.is_empty());
    assert!(check_single(&found).unwrap_err().contains("nenhum"));
}

#[test]
fn arquivo_que_nao_e_pe_e_erro() {
    assert!(manifests(b"nao sou um exe").is_err());
}
