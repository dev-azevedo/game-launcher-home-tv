//! Quais processos podem entrar no conjunto de uma sessao (SEC-15, SEC-17, RN-10).
//!
//! Regra pura sobre dados ja lidos pelo adapter (`SessionProbe`, BE-17): caminhos canonicos,
//! sessao e usuario do Windows. Nunca entram: o Sofa e seus filhos (WebView2), processos e pasta
//! das lojas, `explorer.exe`, outra sessao ou outro usuario, e processos que nao abrem com
//! `QUERY_LIMITED | SYNCHRONIZE` (protegidos). A associacao por pasta compara **por componente**
//! (`C:\Jogos2` nao esta em `C:\Jogos`).

use crate::domain::library::InstallPathPolicy;
use crate::domain::path::{path_is_within, path_key};

/// Fotografia de um processo candidato.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProcessInfo {
    pub pid: u32,
    pub parent_pid: Option<u32>,
    /// Caminho canonico da imagem (`QueryFullProcessImageNameW`), ja resolvido pelo adapter.
    pub image_path: String,
    pub session_id: u32,
    /// O dono do processo e o mesmo usuario do Sofa.
    pub same_user: bool,
    /// Abriu com `PROCESS_QUERY_LIMITED_INFORMATION | SYNCHRONIZE`.
    pub openable: bool,
}

/// Onde a imagem de um processo do jogo precisa estar para entrar no conjunto.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GameScope {
    /// Dentro da pasta de instalacao (canonica, ja validada por SEC-16), por componente.
    Dir(String),
    /// So o executavel do jogo, pelo caminho exato: a pasta dele nao passou em SEC-16 (raiz de
    /// unidade, pasta rasa ou proibida), e associar por ela pegaria processos alheios (SEC-BUG-04).
    ExactExe(String),
}

impl GameScope {
    /// Escopo de um jogo iniciado por `.exe` (manual, GOG ou elevado): a pasta so vale se passar
    /// em `check_install_dir` (e entra na forma canonica de texto); senao, so o caminho exato do
    /// exe.
    pub fn for_exe(exe: &str, install_dir: Option<&str>, policy: &InstallPathPolicy) -> Self {
        match install_dir.and_then(|d| policy.check_install_dir(d).ok()) {
            Some(dir) => Self::Dir(dir),
            None => Self::ExactExe(exe.to_owned()),
        }
    }

    /// `true` se a imagem esta no escopo.
    pub fn contains(&self, image_path: &str) -> bool {
        match self {
            Self::Dir(dir) => path_is_within(dir, image_path),
            Self::ExactExe(exe) => {
                let key = path_key(exe);
                !key.is_empty() && key == path_key(image_path)
            }
        }
    }
}

/// O que o Sofa sabe de si e do jogo no momento da varredura.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionContext {
    pub own_pid: u32,
    pub own_session_id: u32,
    /// PIDs dos `msedgewebview2.exe` do proprio Sofa (a arvore de WebView2 a partir de
    /// `own_pid`). **Nao** e "todo descendente": a Steam iniciada pelo Sofa e os jogos dela sao
    /// descendentes e precisam entrar no conjunto pela pasta de instalacao.
    pub own_child_pids: Vec<u32>,
    /// Onde a imagem de um processo do jogo pode estar (SEC-16, SEC-BUG-04).
    pub scope: GameScope,
    /// Pastas das lojas (Steam, Epic, GOG Galaxy) cujos processos nunca contam.
    pub store_dirs: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Exclusion {
    /// O proprio Sofa.
    Sofa,
    /// Filho do Sofa (WebView2).
    SofaChild,
    /// Processo na pasta de uma loja, ou executavel de loja.
    Store,
    /// `explorer.exe` (shell do usuario).
    Explorer,
    /// Sessao do Windows diferente da do Sofa.
    OtherSession,
    /// Outro usuario.
    OtherUser,
    /// Nao abre com os direitos minimos (protegido ou de sistema).
    NotOpenable,
    /// Imagem fora da pasta de instalacao do jogo.
    OutsideInstallDir,
}

/// Executaveis de loja que nunca contam, mesmo fora das pastas conhecidas (RN-10).
const STORE_IMAGES: &[&str] = &[
    "steam.exe",
    "steamwebhelper.exe",
    "steamservice.exe",
    "epicgameslauncher.exe",
    "epicwebhelper.exe",
    "galaxyclient.exe",
    "galaxyclienthelper.exe",
    "galaxycommunication.exe",
];

fn file_name(path: &str) -> String {
    path.rsplit(['\\', '/']).next().unwrap_or("").to_lowercase()
}

/// `Ok(())` se o processo pode entrar no conjunto da sessao; senao, o motivo da exclusao.
/// A ordem das checagens e a da gravidade: identidade do Sofa, escopo (sessao/usuario), direitos,
/// lojas e shell, e por fim a pasta de instalacao.
pub fn classify(p: &ProcessInfo, ctx: &SessionContext) -> Result<(), Exclusion> {
    if p.pid == ctx.own_pid {
        return Err(Exclusion::Sofa);
    }
    if ctx.own_child_pids.contains(&p.pid) {
        return Err(Exclusion::SofaChild);
    }
    if p.session_id != ctx.own_session_id {
        return Err(Exclusion::OtherSession);
    }
    if !p.same_user {
        return Err(Exclusion::OtherUser);
    }
    if !p.openable {
        return Err(Exclusion::NotOpenable);
    }
    let image = file_name(&p.image_path);
    if image == "explorer.exe" {
        return Err(Exclusion::Explorer);
    }
    if STORE_IMAGES.contains(&image.as_str())
        || ctx
            .store_dirs
            .iter()
            .any(|d| path_is_within(d, &p.image_path))
    {
        return Err(Exclusion::Store);
    }
    if !ctx.scope.contains(&p.image_path) {
        return Err(Exclusion::OutsideInstallDir);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx() -> SessionContext {
        SessionContext {
            own_pid: 100,
            own_session_id: 1,
            own_child_pids: vec![101, 102],
            scope: GameScope::Dir(r"C:\Jogos\Portal 2".into()),
            store_dirs: vec![r"C:\Program Files (x86)\Steam".into()],
        }
    }

    fn game_proc() -> ProcessInfo {
        ProcessInfo {
            pid: 500,
            parent_pid: Some(400),
            image_path: r"C:\Jogos\Portal 2\bin\portal2.exe".into(),
            session_id: 1,
            same_user: true,
            openable: true,
        }
    }

    #[test]
    fn processo_do_jogo_na_pasta_de_instalacao_entra() {
        assert_eq!(classify(&game_proc(), &ctx()), Ok(()));
    }

    #[test]
    fn filho_direto_do_sofa_na_pasta_do_jogo_entra() {
        // Steam iniciada pelo Sofa: o jogo e descendente do Sofa, mas nao e WebView2.
        let p = ProcessInfo {
            parent_pid: Some(100),
            ..game_proc()
        };
        assert_eq!(classify(&p, &ctx()), Ok(()));
    }

    #[test]
    fn cada_exclusao_de_sec_17_tem_o_seu_motivo() {
        let casos: Vec<(ProcessInfo, Exclusion)> = vec![
            (
                ProcessInfo {
                    pid: 100,
                    ..game_proc()
                },
                Exclusion::Sofa,
            ),
            (
                ProcessInfo {
                    pid: 101,
                    ..game_proc()
                },
                Exclusion::SofaChild,
            ),
            (
                ProcessInfo {
                    session_id: 2,
                    ..game_proc()
                },
                Exclusion::OtherSession,
            ),
            (
                ProcessInfo {
                    same_user: false,
                    ..game_proc()
                },
                Exclusion::OtherUser,
            ),
            (
                ProcessInfo {
                    openable: false,
                    ..game_proc()
                },
                Exclusion::NotOpenable,
            ),
            (
                ProcessInfo {
                    image_path: r"C:\Windows\explorer.exe".into(),
                    ..game_proc()
                },
                Exclusion::Explorer,
            ),
            (
                ProcessInfo {
                    image_path: r"C:\Jogos\Portal 2\steam.exe".into(),
                    ..game_proc()
                },
                Exclusion::Store,
            ),
            (
                ProcessInfo {
                    image_path: r"C:\Jogos\Portal 2\EpicGamesLauncher.exe".into(),
                    ..game_proc()
                },
                Exclusion::Store,
            ),
            (
                ProcessInfo {
                    image_path: r"C:\Program Files (x86)\Steam\bin\cef\helper.exe".into(),
                    ..game_proc()
                },
                Exclusion::Store,
            ),
            (
                ProcessInfo {
                    image_path: r"C:\Outros\Jogo\game.exe".into(),
                    ..game_proc()
                },
                Exclusion::OutsideInstallDir,
            ),
        ];
        for (p, want) in casos {
            assert_eq!(classify(&p, &ctx()), Err(want), "{p:?}");
        }
    }

    fn policy() -> InstallPathPolicy {
        InstallPathPolicy::new(
            r"C:\Windows",
            vec![r"C:\Program Files".to_owned(), r"C:\Users\ana".to_owned()],
        )
    }

    #[test]
    fn exe_na_raiz_da_unidade_associa_so_pelo_exe_exato() {
        // SEC-BUG-04: `D:\jogo.exe` daria a pasta `D:`; outro processo do usuario em `D:\` nao entra.
        let scope = GameScope::for_exe(r"D:\jogo.exe", Some("D:"), &policy());
        assert_eq!(scope, GameScope::ExactExe(r"D:\jogo.exe".into()));
        let c = SessionContext { scope, ..ctx() };
        let jogo = ProcessInfo {
            image_path: r"D:\jogo.exe".into(),
            ..game_proc()
        };
        let outro = ProcessInfo {
            image_path: r"D:\Navegador\browser.exe".into(),
            ..game_proc()
        };
        let vizinho = ProcessInfo {
            image_path: r"D:\outro.exe".into(),
            ..game_proc()
        };
        assert_eq!(classify(&jogo, &c), Ok(()));
        assert_eq!(classify(&outro, &c), Err(Exclusion::OutsideInstallDir));
        assert_eq!(classify(&vizinho, &c), Err(Exclusion::OutsideInstallDir));
    }

    #[test]
    fn pasta_reprovada_por_sec_16_cai_no_exe_exato() {
        // Cada motivo de SEC-16: rasa, proibida, ancestral de proibida, dentro de %SystemRoot%.
        for dir in [
            r"D:\Jogos",
            r"C:\Program Files",
            r"C:\Users",
            r"C:\Windows\System32",
        ] {
            let exe = format!(r"{dir}\g.exe");
            assert_eq!(
                GameScope::for_exe(&exe, Some(dir), &policy()),
                GameScope::ExactExe(exe.clone()),
                "{dir}"
            );
        }
        // Sem pasta nenhuma tambem cai no exe exato.
        assert_eq!(
            GameScope::for_exe(r"D:\Jogos\g.exe", None, &policy()),
            GameScope::ExactExe(r"D:\Jogos\g.exe".into())
        );
    }

    #[test]
    fn pasta_valida_segue_associando_pela_pasta() {
        let scope = GameScope::for_exe(r"D:\Jogos\X\x.exe", Some(r"D:\Jogos\X"), &policy());
        assert_eq!(scope, GameScope::Dir(r"D:\Jogos\X".into()));
        assert!(scope.contains(r"D:\Jogos\X\bin\helper.exe"));
        assert!(!scope.contains(r"D:\Jogos\X2\a.exe"));
    }

    #[test]
    fn exe_exato_ignora_caixa_barras_e_prefixo_verbatim() {
        let scope = GameScope::ExactExe(r"D:\Jogo.exe".into());
        assert!(scope.contains(r"d:/jogo.EXE"));
        assert!(scope.contains(r"\\?\D:\Jogo.exe"));
        assert!(!scope.contains(r"D:\Jogo.exe.bak"));
        assert!(!scope.contains(r"D:\Sub\Jogo.exe"));
        assert!(!GameScope::ExactExe(String::new()).contains(""));
    }

    #[test]
    fn processo_homonimo_fora_da_pasta_nao_entra() {
        // SP-02, S4: mesmo nome de exe, pasta diferente.
        let p = ProcessInfo {
            image_path: r"D:\Copia\Portal 2\bin\portal2.exe".into(),
            ..game_proc()
        };
        assert_eq!(classify(&p, &ctx()), Err(Exclusion::OutsideInstallDir));
    }
}
