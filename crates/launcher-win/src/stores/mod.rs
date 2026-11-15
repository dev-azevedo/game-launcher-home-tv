//! Detectores das lojas (BE-25 a BE-27): so leem arquivo e registro e extraem campos crus. Toda
//! validacao (ID, caminho, executavel, DLC, nao-jogo) e do dominio (`launcher-core::domain::
//! library::detect`). Arquivo de terceiro tem limite de 4 MB checado antes de ler e nenhum
//! `unwrap` em dado de fora (SEC-13). Nunca escreve em arquivo ou chave de loja.

pub mod epic;
pub mod gog;
pub mod registry;
pub mod steam;

use std::fs::{self, File};
use std::io::{self, Read};
use std::path::Path;
use std::sync::Arc;

use launcher_core::domain::library::{InstallPathPolicy, Reject};

use crate::path_canon::{known, FsPathResolver, PathResolver};

pub use epic::EpicScanner;
pub use gog::GogScanner;
pub use registry::{Hive, RegistryReader, WinRegistry};
pub use steam::SteamScanner;

/// Limite de leitura de qualquer arquivo de terceiro (SEC-13).
pub const MAX_MANIFEST_BYTES: u64 = 4 * 1024 * 1024;

/// Ambiente comum dos detectores: a politica de pastas e como achar o disco.
#[derive(Clone)]
pub struct StoreEnv {
    pub policy: InstallPathPolicy,
    pub resolver: Arc<dyn PathResolver>,
}

impl StoreEnv {
    pub fn new(policy: InstallPathPolicy, resolver: Arc<dyn PathResolver>) -> Self {
        Self { policy, resolver }
    }

    /// Politica do Windows de verdade (SEC-16): `%SystemRoot%` e as pastas proibidas, as lojas
    /// (cliente) e a pasta do proprio Sofa.
    pub fn system() -> Self {
        // Pasta do cliente da Steam (SteamPath do HKCU): proibida para TODAS as lojas, nao so para
        // o scanner da Steam (SEC-BUG-02). Sem ela, um `.item` da Epic com `InstallLocation` na
        // pasta da Steam associaria processos de outros jogos e do overlay da Steam.
        let steam_dir = WinRegistry
            .read_string(Hive::CurrentUser, r"Software\Valve\Steam", "SteamPath")
            .and_then(|raw| steam::normalize_steam_path(&raw))
            .and_then(|p| FsPathResolver.canonicalize(&p));
        let system_root = known::windows_dir().unwrap_or_else(|| r"C:\Windows".to_owned());
        Self::new(
            InstallPathPolicy::new(&system_root, Self::forbidden_dirs(steam_dir)),
            Arc::new(FsPathResolver),
        )
    }

    /// Pastas que nunca sao pasta de instalacao (SEC-16): sistema, perfil, clientes das lojas
    /// (inclui a da Steam, se instalada) e a do proprio Sofa.
    fn forbidden_dirs(steam_dir: Option<String>) -> Vec<String> {
        let sofa_dir = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|d| d.to_string_lossy().into_owned()));
        let pf86 = known::program_files_x86();
        [
            known::program_files(),
            pf86.clone(),
            known::program_data(),
            known::profile(),
            known::local_app_data(),
            known::roaming_app_data(),
            sofa_dir,
            // Clientes das lojas (as pastas dos jogos ficam em outro lugar).
            pf86.as_ref().map(|p| format!(r"{p}\Epic Games\Launcher")),
            pf86.as_ref().map(|p| format!(r"{p}\GOG Galaxy")),
            steam_dir,
        ]
        .into_iter()
        .flatten()
        .map(|p| crate::path_canon::strip_verbatim(&p))
        .collect()
    }

    pub fn canon(&self, path: &str) -> Option<String> {
        self.resolver.canonicalize(path)
    }
}

/// Le o arquivo inteiro se ele tem no maximo 4 MB; senao `InvalidData`. O tamanho e checado antes
/// de ler e a leitura em si e limitada (o arquivo pode crescer entre as duas coisas).
pub fn read_limited(path: &Path) -> io::Result<Vec<u8>> {
    let file = File::open(path)?;
    if file.metadata()?.len() > MAX_MANIFEST_BYTES {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "arquivo acima de 4 MB",
        ));
    }
    let mut buf = Vec::new();
    file.take(MAX_MANIFEST_BYTES + 1).read_to_end(&mut buf)?;
    if buf.len() as u64 > MAX_MANIFEST_BYTES {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "arquivo acima de 4 MB",
        ));
    }
    Ok(buf)
}

/// `true` se e arquivo comum (nao link, nao pasta).
pub fn is_plain_file(path: &Path) -> bool {
    fs::symlink_metadata(path).is_ok_and(|m| m.is_file())
}

/// Registra o motivo de um item recusado: filtro esperado em depuracao, defeito em aviso.
pub fn log_reject(store: &str, item: &str, reason: Reject) {
    if reason.is_expected() {
        tracing::debug!(
            loja = store,
            item,
            motivo = reason.as_str(),
            "item ignorado"
        );
    } else {
        tracing::warn!(
            loja = store,
            item,
            motivo = reason.as_str(),
            "item recusado"
        );
    }
}

#[cfg(test)]
pub(crate) mod testing {
    //! Apoio dos testes dos detectores.

    use std::fs;
    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::sync::Arc;

    use launcher_core::domain::library::InstallPathPolicy;

    use super::StoreEnv;
    use crate::path_canon::FsPathResolver;

    pub struct TempDir(PathBuf);

    impl TempDir {
        pub fn new(tag: &str) -> Self {
            static N: AtomicU32 = AtomicU32::new(0);
            let p = std::env::temp_dir().join(format!(
                "sofa-{tag}-{}-{}",
                std::process::id(),
                N.fetch_add(1, Ordering::SeqCst)
            ));
            fs::create_dir_all(&p).unwrap();
            Self(p)
        }
        pub fn path(&self) -> &Path {
            &self.0
        }
        pub fn text(&self) -> String {
            // Forma canonica (o temp pode ter nome curto 8.3): e a que os detectores devolvem.
            crate::path_canon::canonicalize(self.0.to_str().unwrap()).unwrap()
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    /// Disco de verdade, politica com `%SystemRoot%` e sem pastas proibidas (o temp fica dentro do
    /// perfil, que a politica real proibiria como ancestral).
    pub fn env() -> Arc<StoreEnv> {
        let root = crate::path_canon::known::windows_dir().unwrap();
        Arc::new(StoreEnv::new(
            InstallPathPolicy::new(&root, Vec::new()),
            Arc::new(FsPathResolver),
        ))
    }

    pub fn fixtures() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join("fixtures")
    }

    pub fn write(path: &Path, bytes: &[u8]) {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, bytes).unwrap();
    }

    /// Escapa `\` como o VDF.
    pub fn vdf_escape(p: &str) -> String {
        p.replace('\\', "\\\\")
    }
}
