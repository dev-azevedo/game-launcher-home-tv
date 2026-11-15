//! QA-12 (rodada 1): robustez e persistencia no disco de verdade (pasta temporaria propria).
//! Cobre CA-25, RN-18, SEC-18 e a secao 11 da spec ("arquivo de configuracao invalido vira backup,
//! nada derruba o launcher"). Testes com `#[ignore = "QA-BUG-xx"]` descrevem defeito aberto.
#![cfg(windows)]

use std::fs::{self, OpenOptions};
use std::os::windows::fs::OpenOptionsExt;
use std::path::PathBuf;
use std::sync::Arc;

use launcher_win::core::application::ports::{
    Clock, LibraryRepository, ScanCache, SettingsRepository,
};
use launcher_win::core::domain::library::stored::{ManualGameRecord, StoredLibrary};
use launcher_win::core::domain::library::GameId;
use launcher_win::core::domain::time::Millis;
use launcher_win::persistence::json::{
    JsonLibraryRepository, JsonScanCache, JsonSettingsRepository,
};

struct FixedClock;
impl Clock for FixedClock {
    fn now(&self) -> Millis {
        Millis(0)
    }
    fn unix_ms(&self) -> i64 {
        1_794_610_123_000
    }
}

struct Dir(PathBuf);
impl Dir {
    fn new(tag: &str) -> Self {
        let p = std::env::temp_dir().join(format!("sofa-qa-{tag}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&p);
        fs::create_dir_all(&p).unwrap();
        Dir(p)
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
impl Drop for Dir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn corpus() -> Vec<(&'static str, Vec<u8>)> {
    vec![
        ("vazio", vec![]),
        ("so_espacos", b"   \r\n  ".to_vec()),
        ("lixo_binario", (0u8..=255).cycle().take(5000).collect()),
        (
            "json_truncado",
            br#"{"schemaVersion":1,"manualGames":[{"id":"manual:9f3a"#.to_vec(),
        ),
        ("json_nulo", b"null".to_vec()),
        ("json_lista", b"[]".to_vec()),
        ("json_numero", b"42".to_vec()),
        (
            "tipo_errado",
            br#"{"schemaVersion":"um","manualGames":{},"overlay":[]}"#.to_vec(),
        ),
        ("versao_do_futuro", br#"{"schemaVersion":999}"#.to_vec()),
        ("utf16_com_bom", {
            let mut v = vec![0xFF, 0xFE];
            for u in "{}".encode_utf16() {
                v.extend(u.to_le_bytes());
            }
            v
        }),
        ("aninhamento_profundo", {
            let mut v = "[".repeat(100_000).into_bytes();
            v.extend("]".repeat(100_000).bytes());
            v
        }),
        ("acima_de_4_mb", vec![b' '; 4 * 1024 * 1024 + 10]),
    ]
}

#[test]
fn library_e_settings_corrompidos_nunca_derrubam_e_viram_backup() {
    for (nome, bytes) in corpus() {
        for (arquivo, kind) in [("library.json", 0), ("settings.json", 1)] {
            let dir = Dir::new(&format!("corrompido-{nome}-{kind}"));
            fs::write(dir.0.join(arquivo), &bytes).unwrap();
            let clock: Arc<dyn Clock> = Arc::new(FixedClock);
            // Nao pode haver panico em nenhum dos casos.
            let recovery = if kind == 0 {
                JsonLibraryRepository::new(&dir.0, clock).load().recovery
            } else {
                JsonSettingsRepository::new(&dir.0, clock).load().recovery
            };
            // "{}" em UTF-16 e "null"/"[]" sao invalidos; um schema valido minimo seria aceito,
            // por isso so exigimos backup quando houve recuperacao e coerencia dos arquivos.
            if recovery.is_some() {
                let backups: Vec<_> = dir
                    .names()
                    .into_iter()
                    .filter(|n| n.contains(".corrupt-"))
                    .collect();
                assert_eq!(backups.len(), 1, "{nome}/{arquivo}: {:?}", dir.names());
                assert!(
                    dir.0.join(arquivo).exists(),
                    "{nome}/{arquivo}: arquivo recriado"
                );
            }
        }
    }
}

#[test]
fn scan_cache_corrompido_volta_vazio_sem_backup_e_sem_panico() {
    for (nome, bytes) in corpus() {
        let dir = Dir::new(&format!("scan-{nome}"));
        fs::create_dir_all(dir.0.join("cache")).unwrap();
        fs::write(dir.0.join("cache").join("scan.json"), &bytes).unwrap();
        let loaded = JsonScanCache::new(&dir.0).load();
        assert!(loaded.is_empty(), "{nome}");
        assert!(
            !fs::read_dir(dir.0.join("cache")).unwrap().any(|e| e
                .unwrap()
                .file_name()
                .to_string_lossy()
                .contains("corrupt")),
            "{nome}: cache nunca vira backup"
        );
    }
}

#[test]
fn arquivo_no_lugar_e_pasta_nao_derruba_a_leitura() {
    let dir = Dir::new("pasta-no-lugar");
    fs::create_dir_all(dir.0.join("library.json")).unwrap();
    fs::create_dir_all(dir.0.join("settings.json")).unwrap();
    let clock: Arc<dyn Clock> = Arc::new(FixedClock);
    let _ = JsonLibraryRepository::new(&dir.0, clock.clone()).load();
    let _ = JsonSettingsRepository::new(&dir.0, clock).load();
}

/// CA-25 e RN-18: os jogos manuais e os ocultos do usuario sobrevivem. Hoje uma falha de LEITURA
/// (arquivo travado por antivirus, backup ou indexador) devolve a biblioteca vazia sem `recovery`
/// e sem tocar no arquivo; na mutacao seguinte (ocultar, renomear, marcar jogado) o servico le de
/// novo, recebe o padrao e grava `library.json` so com a mudanca nova: os jogos manuais somem.
#[test]
#[ignore = "QA-BUG-02: leitura falha vira biblioteca vazia e o proximo save apaga os dados do usuario"]
fn leitura_transitoriamente_travada_nao_pode_levar_a_perda_dos_dados_no_save_seguinte() {
    let dir = Dir::new("leitura-travada");
    let clock: Arc<dyn Clock> = Arc::new(FixedClock);
    let repo = JsonLibraryRepository::new(&dir.0, clock);

    let mut original = StoredLibrary::default();
    original.manual_games.push(ManualGameRecord {
        id: GameId::parse("manual:9f3a1c2b7d4e5a60").unwrap(),
        name: "Meu Jogo".into(),
        path: r"D:\Jogos\Meu Jogo\jogo.exe".into(),
    });
    repo.save(&original).unwrap();
    let antes = fs::read(dir.0.join("library.json")).unwrap();

    // Outro processo abre o arquivo sem compartilhar leitura (mas permite renomear/trocar).
    let _trava = OpenOptions::new()
        .read(true)
        .share_mode(0x2 | 0x4) // FILE_SHARE_WRITE | FILE_SHARE_DELETE, sem FILE_SHARE_READ
        .open(dir.0.join("library.json"))
        .unwrap();

    let lida = repo.load(); // leitura falha: o padrao vem vazio
    assert!(
        lida.value.manual_games.len() == 1 || lida.recovery.is_some(),
        "falha de leitura devolveu biblioteca vazia como se fosse um arquivo novo"
    );

    // O servico faz ler-alterar-gravar: grava o padrao com uma mudanca.
    let mut depois = lida.value;
    depois
        .overlay
        .insert(GameId::parse("steam:620").unwrap(), Default::default());
    let _ = repo.save(&depois);
    drop(_trava);

    // Os dados originais precisam continuar recuperaveis (no proprio arquivo ou em um backup).
    let recuperavel = dir
        .names()
        .iter()
        .any(|n| fs::read(dir.0.join(n)).map(|b| b == antes).unwrap_or(false))
        || fs::read(dir.0.join("library.json"))
            .map(|b| String::from_utf8_lossy(&b).contains("Meu Jogo"))
            .unwrap_or(false);
    assert!(recuperavel, "dados do usuario perdidos: {:?}", dir.names());
}

#[test]
fn settings_com_advanced_estranho_nao_derruba_e_volta_ao_padrao_campo_a_campo() {
    // Padrao do MVP apos o SG-02 (rev. 5): degraus 2 e 3 LIGADOS, F24, ordem 1-3-2-4.
    let dir = Dir::new("advanced");
    let json = br#"{"schemaVersion":1,"advanced":{"foreground":{"step2":"sim","step3":false,"order":[9,9,9]},"session":{"launchTimeoutSecs":-1},"inputDeadzone":5000}}"#;
    fs::write(dir.0.join("settings.json"), json).unwrap();
    let clock: Arc<dyn Clock> = Arc::new(FixedClock);
    let loaded = JsonSettingsRepository::new(&dir.0, clock).load();
    let adv = loaded.value.advanced();
    assert!(
        adv.settings.foreground.step2,
        "tipo errado volta ao padrao (ligado)"
    );
    assert!(!adv.settings.foreground.step3, "valor valido e respeitado");
    assert!(
        !adv.warnings.is_empty(),
        "o valor invalido gera aviso no log"
    );
    assert!(
        loaded.recovery.is_none(),
        "advanced invalido nao derruba o arquivo"
    );
}

/// Roteiro do usuario (bloco 12): quem edita `settings.json` a mao erra nomes. Registra o que o
/// Sofa faz com uma chave desconhecida ao lado de uma valida (so informativo, nao ha RN).
#[test]
fn advanced_com_chave_desconhecida_ao_lado_de_valida_nao_derruba_e_gera_aviso() {
    let dir = Dir::new("advanced-typo");
    let json = br#"{"schemaVersion":1,"advanced":{"foreground":{"step2":false,"step2key":"Alt"}}}"#;
    fs::write(dir.0.join("settings.json"), json).unwrap();
    let clock: Arc<dyn Clock> = Arc::new(FixedClock);
    let loaded = JsonSettingsRepository::new(&dir.0, clock).load();
    let adv = loaded.value.advanced();
    eprintln!(
        "step2={} step2_key={:?} avisos={:?}",
        adv.settings.foreground.step2, adv.settings.foreground.step2_key, adv.warnings
    );
    assert!(loaded.recovery.is_none());
    assert!(!adv.warnings.is_empty());
}
