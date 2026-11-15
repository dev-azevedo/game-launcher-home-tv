//! QA-11 (rodada 1): entradas hostis nas politicas puras do dominio, vistas de fora (API publica).
//! Os casos vem de SEC-07, SEC-09, SEC-11, SEC-12, SEC-14 e SEC-16 e dos nomes de arquivo que o
//! Windows trata de forma especial. Testes marcados `#[ignore = "QA-BUG-xx"]` descrevem um defeito
//! aberto: retirar o `ignore` quando o bug for corrigido.

use launcher_core::domain::library::exe_policy::{
    check_manual_exe_path, check_store_exe, parse_browse_path, ManualPathError,
};
use launcher_core::domain::library::id_formats::{
    is_epic_component, is_gog_id, parse_steam_app_id,
};
use launcher_core::domain::library::install_path_policy::{
    resolve_confined, InstallPathPolicy, PathPolicyError,
};
use launcher_core::domain::library::launch::{check_launch_exe_path, epic_uri, steam_uri};
use launcher_core::domain::library::normalize_display_name;

fn policy() -> InstallPathPolicy {
    InstallPathPolicy::new(
        r"C:\Windows",
        [
            r"C:\Program Files".to_owned(),
            r"C:\Program Files (x86)".to_owned(),
            r"C:\ProgramData".to_owned(),
            r"C:\Users\ana".to_owned(),
            r"C:\Users\ana\AppData\Local".to_owned(),
        ],
    )
}

#[test]
fn sec09_formas_hostis_de_exe_manual_sao_recusadas() {
    let recusados = [
        r"C:\Jogos\x.lnk",
        r"C:\Jogos\x.bat",
        r"C:\Jogos\x.cmd",
        r"C:\Jogos\x.ps1",
        r"C:\Jogos\x.txt",
        r"C:\Jogos\x.exe.txt",
        r"C:\Jogos\x.exe:ads",
        r"C:\Jogos\x.exe::$DATA",
        r"C:\Jogos\x.exe.",
        r"C:\Jogos\x.exe ",
        r"C:\Jogos\..\Windows\notepad.exe",
        r"C:\Jogos\.\x.exe",
        r"C:\Jogos\CON.exe",
        r"C:\Jogos\nul.exe",
        r"C:\Jogos\LPT1.exe",
        r"C:\Jogos\aux.tmp.exe",
        r"C:\Jogos\a?b.exe",
        r"C:\Jogos\a*b.exe",
        r"C:\Jogos\a|b.exe",
        "C:\\Jogos\\x.exe\0",
        "C:\\Jogos\\x\n.exe",
        r"\\127.0.0.1\c$\Jogos\x.exe",
        r"\\servidor\jogos\x.exe",
        r"\\?\C:\Jogos\x.exe",
        r"\\?\UNC\servidor\jogos\x.exe",
        r"\\.\C:\Jogos\x.exe",
        "//servidor/jogos/x.exe",
        r"x.exe",
        r"Jogos\x.exe",
        r"C:x.exe",
        r"C:/Jogos/x.exe",
        r"",
        r"C:\",
    ];
    for p in recusados {
        assert!(check_manual_exe_path(p).is_err(), "deveria recusar {p:?}");
        assert!(
            check_launch_exe_path(p).is_err(),
            "iniciar deveria recusar {p:?}"
        );
    }
}

#[test]
fn sec09_rede_e_distinguida_de_forma_invalida() {
    assert_eq!(
        check_manual_exe_path(r"\\servidor\jogos\x.exe"),
        Err(ManualPathError::NotAllowed)
    );
    assert_eq!(
        check_manual_exe_path(r"\\?\UNC\servidor\jogos\x.exe"),
        Err(ManualPathError::NotAllowed)
    );
    assert_eq!(
        check_manual_exe_path(r"C:\Jogos\x.exe:ads"),
        Err(ManualPathError::NotExecutable)
    );
}

#[test]
fn sec09_exe_comum_com_e_comercial_aspas_espacos_e_maiuscula_e_aceito() {
    for p in [
        r"C:\Jogos\A & B\x.exe",
        r"D:\Meus Jogos\Aventura (2024)\Aventura.EXE",
        r"E:\Jogos\Ação Ágil\ação.exe",
        r"C:\Jogos\x.exe.exe",
        r"C:\Jogos\con-sole.exe",
        r"C:\Jogos\comx.exe",
    ] {
        assert_eq!(check_manual_exe_path(p), Ok(()), "{p:?}");
    }
}

#[test]
fn sec14_exe_de_loja_nao_pode_ser_interprete_nem_estar_no_windows() {
    let p = policy();
    for bad in [
        r"C:\Jogos\X\start.bat",
        r"C:\Jogos\X\run.cmd",
        r"C:\Jogos\X\run.ps1",
        r"C:\Jogos\X\atalho.lnk",
        r"C:\Windows\System32\cmd.exe",
        r"C:\Windows\System32\WindowsPowerShell\v1.0\powershell.exe",
        r"c:\WINDOWS\system32\calc.exe",
        r"\\srv\x\jogo.exe",
    ] {
        assert!(check_store_exe(bad, &p).is_err(), "{bad:?}");
    }
    assert_eq!(check_store_exe(r"C:\Jogos\X\x.exe", &p), Ok(()));
}

#[test]
fn sec11_ids_de_loja_fora_do_formato_nao_sao_sanitizados() {
    for bad in [
        "",
        "0x10",
        "10//-console",
        "-1",
        "+620",
        " 620",
        "620 ",
        "620\n",
        "62 0",
        "٦٢٠",
        "４２",
        "4294967296",
        "99999999999",
    ] {
        assert_eq!(parse_steam_app_id(bad), None, "steam {bad:?}");
    }
    assert_eq!(parse_steam_app_id("620"), Some(620));
    for bad in [
        "",
        "x?action=foo&y",
        "a b",
        "a/b",
        "a:b",
        "a%3Ab",
        "ação",
        "a\n",
        &"a".repeat(129),
    ] {
        assert!(!is_epic_component(bad), "epic {bad:?}");
    }
    for bad in ["", "12a", "-1", " 1", "1\n", &"9".repeat(21)] {
        assert!(!is_gog_id(bad), "gog {bad:?}");
    }
}

#[test]
fn sec11_uri_da_epic_codifica_tudo_que_nao_e_seguro() {
    // Mesmo que um componente hostil chegasse aqui, nada vira separador de URI.
    let uri = epic_uri("ns", "it?em", "a&b=c#d/e%f g");
    let corpo = uri
        .strip_prefix("com.epicgames.launcher://apps/")
        .expect("esquema fixo");
    let (caminho, consulta) = corpo.split_once('?').expect("uma consulta");
    assert_eq!(consulta, "action=launch&silent=true");
    assert!(
        !caminho.contains(['?', '&', '#', '/', ' ', '=']),
        "{caminho}"
    );
    assert_eq!(steam_uri(u32::MAX), "steam://rungameid/4294967295");
}

#[test]
fn sec12_resolve_confined_recusa_escape_por_componente() {
    let base = r"C:\Jogos\X";
    for rel in [
        r"..\Y\a.exe",
        r"bin\..\..\Y\a.exe",
        r"..",
        r"C:\Windows\System32\cmd.exe",
        r"\Windows\x.exe",
        r"D:\Jogos\X\a.exe",
        r"C:\Jogos\X2\a.exe",
        r"C:\Jogos\X",
        r"bin\a.exe:ads",
        r"bin\a.exe.",
        r"bin\ a.exe ",
        "",
    ] {
        assert!(resolve_confined(base, rel).is_err(), "{rel:?}");
    }
    assert_eq!(
        resolve_confined(base, r"bin/a.exe").as_deref(),
        Ok(r"C:\Jogos\X\bin\a.exe")
    );
}

#[test]
fn sec16_pasta_de_instalacao_rasa_ou_proibida_e_recusada() {
    let p = policy();
    for dir in [
        r"C:\",
        r"C:\Jogos",
        r"C:\Program Files",
        r"C:\ProgramData",
        r"C:\Users\ana",
        r"C:\Users\ana\AppData\Local",
        r"C:\Users",
        r"C:\Windows\Temp\X",
        r"C:\Windows",
        r"C:\Jogos\..",
        r"\\srv\jogos\X",
    ] {
        assert!(p.check_install_dir(dir).is_err(), "{dir:?}");
    }
    assert!(p.check_install_dir(r"C:\Jogos\X").is_ok());
    assert!(p
        .check_install_dir(r"C:\Program Files\Epic Games\X")
        .is_ok());
    assert_eq!(
        p.check_install_dir(r"C:\Jogos"),
        Err(PathPolicyError::TooShallow)
    );
}

#[test]
fn navegador_de_pastas_nao_aceita_rede_nem_prefixos() {
    for p in [
        r"\\servidor\jogos",
        r"\\?\C:\",
        r"\\.\C:\",
        r"C:\Jogos\..",
        r"C:\Jogos\x.exe:ads",
        r"Jogos",
        "C:\\Jogos\0",
    ] {
        assert!(parse_browse_path(Some(p)).is_err(), "{p:?}");
    }
    assert!(parse_browse_path(None).is_ok());
    assert!(parse_browse_path(Some("")).is_ok());
}

#[test]
fn sec07_nome_exibido_nao_tem_controle_nem_override_bidi_e_nunca_passa_de_256() {
    let n = normalize_display_name("\u{202E}exe.txt\u{0}\u{7}");
    assert_eq!(n, "exe.txt");
    for entrada in [
        "a".repeat(100_000),
        "🎮".repeat(1000),
        "e\u{301}".repeat(1000),
        "\u{202E}".repeat(300) + "x",
    ] {
        assert!(normalize_display_name(&entrada).chars().count() <= 256);
    }
    // Emoji composto nao pode ser partido ao meio e virar caractere invalido (so chars inteiros).
    assert!(normalize_display_name(&"👨‍👩‍👧".repeat(200)).is_char_boundary(0));
}

/// CA-61: "Nome vazio não é aceito". Um nome feito só de caracteres invisíveis passa pela
/// normalização, vira o nome do card (em branco) e nada o distingue na tela.
#[test]
#[ignore = "QA-BUG-01: nome so com caracteres invisiveis (ZWSP, BOM, filler) nao e tratado como vazio"]
fn ca61_nome_so_com_invisiveis_vira_vazio() {
    for invisivel in [
        "\u{200B}",
        "\u{200B}\u{200C}\u{200D}",
        "\u{2060}",
        "\u{FEFF}",
        "\u{3164}",
        "\u{2800}",
        "\u{00AD}",
        "\u{115F}",
    ] {
        assert_eq!(normalize_display_name(invisivel), "", "{invisivel:?}");
    }
}
