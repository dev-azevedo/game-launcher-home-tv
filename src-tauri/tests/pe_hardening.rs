//! SEC-BUG-06 (SEC-23, ADR-0006): o `sofa.exe` de release sai com Control Flow Guard e com as
//! defesas padrao do loader ligadas. O teste le o `DllCharacteristics` do cabecalho opcional do PE
//! (nao procura texto no binario).
//!
//! - MSVC: reprova sem `IMAGE_DLLCHARACTERISTICS_GUARD_CF`, sem ASLR (`DYNAMIC_BASE`), sem ASLR de
//!   alta entropia ou sem DEP (`NX_COMPAT`). A flag vem de `.cargo/config.toml`
//!   (`-C control-flow-guard`). E a trava do `scripts/ci.ps1 -Release`.
//! - `-gnu` (dev, nunca distribuido, o MinGW nao tem CFG): so avisa.
//!
//! `SOFA_MANIFEST_EXE=<caminho>` aponta o teste para outro `sofa.exe` (o do release MSVC, que o
//! `scripts/ci.ps1 -Release` passa, a mesma variavel do `manifest_resource`).

use std::fs;

const GUARD_CF: u32 = 0x4000;
const HIGH_ENTROPY_VA: u32 = 0x0020;
const DYNAMIC_BASE: u32 = 0x0040;
const NX_COMPAT: u32 = 0x0100;

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

/// `DllCharacteristics` do cabecalho opcional (deslocamento 70, igual em PE32 e PE32+).
fn dll_characteristics(exe: &[u8]) -> Result<u32, String> {
    if exe.get(0..2) != Some(b"MZ") {
        return Err("sem cabecalho MZ".into());
    }
    let pe = u32_at(exe, 0x3C)? as usize;
    if exe.get(pe..pe + 4) != Some(b"PE\0\0") {
        return Err("sem assinatura PE".into());
    }
    let opt = pe + 4 + 20;
    match u16_at(exe, opt)? {
        0x20B | 0x10B => u16_at(exe, opt + 70),
        m => Err(format!("magic do optional header desconhecido: {m:#x}")),
    }
}

/// Defesas exigidas no release, ou o que falta.
fn check_hardening(exe: &[u8]) -> Result<(), String> {
    let c = dll_characteristics(exe)?;
    let mut missing = Vec::new();
    for (bit, name) in [
        (GUARD_CF, "GUARD_CF (Control Flow Guard, SEC-BUG-06)"),
        (DYNAMIC_BASE, "DYNAMIC_BASE (ASLR)"),
        (HIGH_ENTROPY_VA, "HIGH_ENTROPY_VA (ASLR de 64 bits)"),
        (NX_COMPAT, "NX_COMPAT (DEP)"),
    ] {
        if c & bit == 0 {
            missing.push(name);
        }
    }
    if missing.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "DllCharacteristics {c:#06x} sem: {}",
            missing.join(", ")
        ))
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
fn sofa_exe_tem_control_flow_guard_e_as_defesas_do_loader() {
    let verdict = check_hardening(&exe_under_test());
    if cfg!(target_env = "msvc") {
        verdict.unwrap_or_else(|e| panic!("SEC-BUG-06 (MSVC): {e}"));
        return;
    }
    // `-gnu`: nunca distribuido e sem CFG no MinGW; so avisa.
    if let Err(e) = verdict {
        eprintln!("AVISO SEC-BUG-06 (-gnu, nao distribuido): {e}");
    }
}

// --- o proprio teste precisa conseguir reprovar: PE sintetico com cada combinacao ---

/// PE32+ minimo (so os cabecalhos) com o `DllCharacteristics` dado.
fn synthetic_pe(characteristics: u16) -> Vec<u8> {
    let mut b = vec![0u8; 0x200];
    b[0..2].copy_from_slice(b"MZ");
    b[0x3C..0x40].copy_from_slice(&0x80u32.to_le_bytes());
    b[0x80..0x84].copy_from_slice(b"PE\0\0");
    let opt = 0x80 + 4 + 20;
    b[opt..opt + 2].copy_from_slice(&0x20Bu16.to_le_bytes());
    b[opt + 70..opt + 72].copy_from_slice(&characteristics.to_le_bytes());
    b
}

const ALL: u16 = (GUARD_CF | DYNAMIC_BASE | HIGH_ENTROPY_VA | NX_COMPAT) as u16;

#[test]
fn pe_com_todas_as_defesas_passa() {
    check_hardening(&synthetic_pe(ALL)).expect("todas ligadas");
}

#[test]
fn pe_sem_control_flow_guard_reprova() {
    let err = check_hardening(&synthetic_pe(ALL & !(GUARD_CF as u16))).unwrap_err();
    assert!(err.contains("GUARD_CF"), "{err}");
}

#[test]
fn pe_sem_aslr_ou_dep_reprova() {
    for (bit, name) in [
        (DYNAMIC_BASE, "DYNAMIC_BASE"),
        (HIGH_ENTROPY_VA, "HIGH_ENTROPY_VA"),
        (NX_COMPAT, "NX_COMPAT"),
    ] {
        let err = check_hardening(&synthetic_pe(ALL & !(bit as u16))).unwrap_err();
        assert!(err.contains(name), "{err}");
    }
}

#[test]
fn arquivo_que_nao_e_pe_e_erro() {
    assert!(check_hardening(b"nao sou um PE").is_err());
    assert!(check_hardening(&[]).is_err());
}
