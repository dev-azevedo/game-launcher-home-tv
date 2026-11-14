//! "Iniciar com o Windows" (RN-17, SEC-22, DR-04): regras puras. Nome da tarefa, linha de comando,
//! confinamento do executavel em `Program Files`, o que o Agendador chama de falha e como o
//! instalador deixa o padrao (`InstallDefaults`). Nada de Win32 aqui.

/// Argumento que a entrada de autostart passa ao Sofa (BE-40).
pub const AUTOSTART_ARG: &str = "--autostart";
/// Prefixo do nome da tarefa; o resto e o SID do usuario (SEC-22, SEC-32).
pub const TASK_NAME_PREFIX: &str = "Sofa Autostart (";
/// Nome do valor em `HKCU\...\Run` (SEC-22).
pub const RUN_VALUE_NAME: &str = "Sofa";
/// Nome do executavel instalado.
pub const EXE_FILE_NAME: &str = "sofa.exe";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mechanism {
    Task,
    Run,
    None,
}

/// SID em forma de texto: `S-1-<autoridade>-<sub>...`, so digitos e hifens, ate 15 subautoridades.
/// Cobre conta local, de dominio, da Microsoft e do Azure AD.
pub fn is_valid_sid(sid: &str) -> bool {
    let Some(rest) = sid.strip_prefix("S-1-") else {
        return false;
    };
    let parts: Vec<&str> = rest.split('-').collect();
    (2..=16).contains(&parts.len())
        && parts
            .iter()
            .all(|p| !p.is_empty() && p.len() <= 10 && p.bytes().all(|b| b.is_ascii_digit()))
}

/// `Sofa Autostart (<SID>)`. `None` se o SID nao tiver a forma esperada (nome nunca e montado com
/// texto solto).
pub fn task_name(sid: &str) -> Option<String> {
    is_valid_sid(sid).then(|| format!("{TASK_NAME_PREFIX}{sid})"))
}

/// Caminho do exe entre aspas, como vai em `Run` e no `Path` da acao da tarefa (SEC-22).
pub fn quoted_exe(exe: &str) -> String {
    format!("\"{exe}\"")
}

/// Linha de comando do `Run`: `"C:\Program Files\Sofa\sofa.exe" --autostart`.
pub fn run_command_line(exe: &str) -> String {
    format!("{} {AUTOSTART_ARG}", quoted_exe(exe))
}

/// Chave de comparacao de caminho: sem aspas, sem prefixo `\\?\`, barras iguais, sem barra final,
/// sem diferenca de caixa (o NTFS nao diferencia).
pub fn path_key(path: &str) -> String {
    let t = path.trim().trim_matches('"');
    let t = t.strip_prefix(r"\\?\").unwrap_or(t);
    t.replace('/', "\\").trim_end_matches('\\').to_lowercase()
}

pub fn same_path(a: &str, b: &str) -> bool {
    path_key(a) == path_key(b)
}

/// DR-04: o exe esta dentro de uma das pastas `Program Files` (comparacao por componente, nunca
/// por prefixo de texto: `C:\Program Files Evil\sofa.exe` nao passa). Rejeita `..`.
pub fn is_under_program_files(exe: &str, roots: &[String]) -> bool {
    let exe_key = path_key(exe);
    if exe_key.split('\\').any(|c| c == ".." || c == ".") {
        return false;
    }
    roots.iter().any(|root| {
        let root_key = path_key(root);
        !root_key.is_empty()
            && exe_key.len() > root_key.len() + 1
            && exe_key.starts_with(&root_key)
            && exe_key.as_bytes()[root_key.len()] == b'\\'
    })
}

/// O `Path` de uma acao ou de um valor `Run` aponta para o exe esperado. Com `expected`, so o
/// mesmo caminho vale; sem ele (Sofa fora de `Program Files`), vale um caminho absoluto de
/// unidade que termine em `sofa.exe`.
pub fn exe_matches(path: &str, expected: Option<&str>) -> bool {
    match expected {
        Some(e) => !path_key(e).is_empty() && same_path(path, e),
        None => {
            let key = path_key(path);
            let b = key.as_bytes();
            b.len() > 3
                && b[0].is_ascii_alphabetic()
                && b[1] == b':'
                && b[2] == b'\\'
                && key.rsplit('\\').next() == Some(EXE_FILE_NAME)
        }
    }
}

/// O valor de `Run` e exatamente `"<exe>" --autostart` (aspas em volta do caminho, SEC-22).
pub fn run_value_matches(value: &str, expected: Option<&str>) -> bool {
    let Some(rest) = value.trim().strip_prefix('"') else {
        return false;
    };
    let Some((path, args)) = rest.split_once('"') else {
        return false;
    };
    args.trim() == AUTOSTART_ARG && !path.contains('"') && exe_matches(path, expected)
}

/// `StartupApproved\Run`: o primeiro byte par (02, 06) e "ligado"; impar (03) e o usuario ter
/// desligado no Gerenciador de Tarefas. Sem valor, vale o `Run` como esta.
pub fn startup_approved_disabled(bytes: &[u8]) -> bool {
    bytes.first().is_some_and(|b| b & 1 == 1)
}

/// `LastTaskResult` que **nao** e falha (SEC-22 rev. 4 (c)): sucesso, e os avisos do Agendador
/// (`SCHED_S_*`, 0x00041300 a 0x0004132F: pronto, em execucao, desabilitada, ainda nao rodou,
/// sem gatilho, encerrada pelo usuario, na fila...). Qualquer outro valor e falha.
pub fn is_task_failure(last_result: i32) -> bool {
    let code = last_result as u32;
    !(code == 0 || (0x0004_1300..=0x0004_132F).contains(&code))
}

/// `InstallDefaults` gravado pelo instalador: DWORD 1 liga; 0, outro numero, ausente ou de outro
/// tipo (`None`) deixa desligado (SEC-21).
pub fn install_default_enabled(dword: Option<u32>) -> bool {
    dword == Some(1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sid_e_nome_da_tarefa() {
        let sid = "S-1-5-21-1004336348-1177238915-682003330-512";
        assert!(is_valid_sid(sid));
        assert_eq!(
            task_name(sid).as_deref(),
            Some("Sofa Autostart (S-1-5-21-1004336348-1177238915-682003330-512)")
        );
        assert!(is_valid_sid("S-1-12-1-1234-5678-9012-3456")); // Azure AD
        for ruim in [
            "",
            "S-1",
            "S-1-",
            "S-1-5",
            "s-1-5-21-1",
            "S-2-5-21-1-2-3-4",
            "S-1-5-21-1-2-3-4)\" /f & calc",
            "S-1-5-21--1-2",
            "S-1-5-21-12345678901-1",
            "S-1-5-21-a-b",
        ] {
            assert!(!is_valid_sid(ruim), "{ruim}");
            assert_eq!(task_name(ruim), None, "{ruim}");
        }
    }

    #[test]
    fn linha_de_comando_leva_o_caminho_entre_aspas() {
        let exe = r"C:\Program Files\Sofa\sofa.exe";
        assert_eq!(quoted_exe(exe), "\"C:\\Program Files\\Sofa\\sofa.exe\"");
        assert_eq!(
            run_command_line(exe),
            "\"C:\\Program Files\\Sofa\\sofa.exe\" --autostart"
        );
    }

    #[test]
    fn comparacao_de_caminho_ignora_aspas_caixa_barra_e_prefixo() {
        let exe = r"C:\Program Files\Sofa\sofa.exe";
        for igual in [
            "\"C:\\Program Files\\Sofa\\sofa.exe\"",
            r"c:\program files\sofa\SOFA.EXE",
            r"C:/Program Files/Sofa/sofa.exe",
            r"\\?\C:\Program Files\Sofa\sofa.exe",
            "  C:\\Program Files\\Sofa\\sofa.exe  ",
        ] {
            assert!(same_path(exe, igual), "{igual}");
        }
        for diferente in [
            r"C:\Windows\System32\calc.exe",
            r"C:\Program Files\Sofa\sofa2.exe",
            r"C:\Program Files\Sofa\sub\sofa.exe",
            "",
        ] {
            assert!(!same_path(exe, diferente), "{diferente}");
        }
    }

    #[test]
    fn so_aceita_exe_dentro_de_program_files() {
        let roots = vec![
            r"C:\Program Files".to_owned(),
            r"C:\Program Files (x86)".to_owned(),
        ];
        for ok in [
            r"C:\Program Files\Sofa\sofa.exe",
            r"c:\program files\sofa\sofa.exe",
            r"C:\Program Files (x86)\Sofa\sofa.exe",
            r"\\?\C:\Program Files\Sofa\sofa.exe",
        ] {
            assert!(is_under_program_files(ok, &roots), "{ok}");
        }
        for ruim in [
            r"C:\Program Files Evil\sofa.exe",
            r"C:\Program Files\..\Users\x\sofa.exe",
            r"C:\Users\x\Program Files\Sofa\sofa.exe",
            r"D:\GIT\STD\gamelaucher\target\debug\sofa.exe",
            r"C:\Program Files",
            r"C:\Program Files\",
            r"C:\Program Files\.\Sofa\sofa.exe",
            r"\\servidor\C$\Program Files\Sofa\sofa.exe",
            "",
        ] {
            assert!(!is_under_program_files(ruim, &roots), "{ruim}");
        }
        assert!(!is_under_program_files(
            r"C:\Program Files\Sofa\sofa.exe",
            &[]
        ));
        assert!(!is_under_program_files(
            r"C:\Program Files\Sofa\sofa.exe",
            &[String::new()]
        ));
    }

    /// SEC-22 rev. 4 (c): 0x41301, 0x41303 e 0x41325 nao disparam a troca de mecanismo.
    #[test]
    fn codigos_do_agendador_que_nao_sao_falha() {
        for nao_falha in [
            0,
            0x0004_1300, // SCHED_S_TASK_READY
            0x0004_1301, // SCHED_S_TASK_RUNNING
            0x0004_1302, // SCHED_S_TASK_DISABLED
            0x0004_1303, // SCHED_S_TASK_HAS_NOT_RUN
            0x0004_1304, // SCHED_S_TASK_NO_MORE_RUNS
            0x0004_1305, // SCHED_S_TASK_NOT_SCHEDULED
            0x0004_1306, // SCHED_S_TASK_TERMINATED
            0x0004_1325, // SCHED_S_TASK_QUEUED
        ] {
            assert!(!is_task_failure(nao_falha), "{nao_falha:#x}");
        }
        for falha in [
            0x8007_0005_u32, // E_ACCESSDENIED
            0x8004_1309,     // SCHED_E_TASK_NOT_READY
            0x8007_0002,     // arquivo nao encontrado
            0xC000_0005,     // violacao de acesso do processo
            1,               // codigo de saida 1
            0x0004_1330,     // fora da faixa de avisos
            0x0004_12FF,
        ] {
            assert!(is_task_failure(falha as i32), "{falha:#x}");
        }
    }

    #[test]
    fn exe_e_valor_run_conferidos_por_caminho_e_forma() {
        let exe = r"C:\Program Files\Sofa\sofa.exe";
        assert!(exe_matches(
            r#""C:\Program Files\Sofa\sofa.exe""#,
            Some(exe)
        ));
        assert!(!exe_matches(r"C:\Windows\System32\calc.exe", Some(exe)));
        assert!(!exe_matches("", Some("")));
        // Sem esperado (copia de desenvolvimento): so "<unidade>:\...\sofa.exe".
        assert!(exe_matches(r"D:\GIT\target\debug\sofa.exe", None));
        assert!(!exe_matches(r"C:\Windows\System32\calc.exe", None));
        assert!(!exe_matches(r"sofa.exe", None));
        assert!(!exe_matches(r"\\srv\share\sofa.exe", None));

        let ok = r#""C:\Program Files\Sofa\sofa.exe" --autostart"#;
        assert!(run_value_matches(ok, Some(exe)));
        assert!(run_value_matches(ok, None));
        for ruim in [
            r"C:\Program Files\Sofa\sofa.exe --autostart", // sem aspas
            r#""C:\Program Files\Sofa\sofa.exe""#,         // sem argumento
            r#""C:\Program Files\Sofa\sofa.exe" --autostart --x"#,
            r#""C:\Windows\System32\calc.exe" --autostart"#,
            r#""C:\Program Files\Sofa\sofa.exe --autostart"#,
            "",
        ] {
            assert!(!run_value_matches(ruim, Some(exe)), "{ruim}");
        }
    }

    #[test]
    fn startup_approved_impar_e_desligado() {
        assert!(!startup_approved_disabled(&[0x02, 0, 0, 0]));
        assert!(!startup_approved_disabled(&[0x06, 0, 0, 0]));
        assert!(startup_approved_disabled(&[0x03, 0, 0, 0]));
        assert!(!startup_approved_disabled(&[]));
    }

    /// SEC-21: so o DWORD 1 liga.
    #[test]
    fn install_defaults_so_liga_com_dword_1() {
        assert!(install_default_enabled(Some(1)));
        for desligado in [Some(0), Some(7), Some(2), Some(u32::MAX), None] {
            assert!(!install_default_enabled(desligado), "{desligado:?}");
        }
    }
}
