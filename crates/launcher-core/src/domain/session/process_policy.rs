//! Quais processos podem entrar no conjunto de uma sessao (SEC-15, SEC-17, RN-10).
//!
//! Regra pura sobre dados ja lidos pelo adapter (`SessionProbe`, BE-17): caminhos canonicos,
//! sessao e usuario do Windows. Nunca entram: o Sofa e seus filhos (WebView2), processos e pasta
//! das lojas, `explorer.exe`, outra sessao ou outro usuario, e processos que nao abrem com
//! `QUERY_LIMITED | SYNCHRONIZE` (protegidos). A associacao por pasta compara **por componente**
//! (`C:\Jogos2` nao esta em `C:\Jogos`).

use crate::domain::path::path_is_within;

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

/// O que o Sofa sabe de si e do jogo no momento da varredura.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionContext {
    pub own_pid: u32,
    pub own_session_id: u32,
    /// PIDs dos `msedgewebview2.exe` do proprio Sofa (a arvore de WebView2 a partir de
    /// `own_pid`). **Nao** e "todo descendente": a Steam iniciada pelo Sofa e os jogos dela sao
    /// descendentes e precisam entrar no conjunto pela pasta de instalacao.
    pub own_child_pids: Vec<u32>,
    /// Pasta de instalacao do jogo (canonica, ja validada por SEC-16).
    pub install_dir: String,
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
    if !path_is_within(&ctx.install_dir, &p.image_path) {
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
            install_dir: r"C:\Jogos\Portal 2".into(),
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
