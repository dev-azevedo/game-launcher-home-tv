//! Logs do Sofa em `<pasta de dados>\logs\`: arquivo com rotacao por tamanho, 5 MB x 3 (SEC-18).
//! Os registros so levam IDs, nomes e caminhos de jogo, nunca linha de comando, ambiente, titulos
//! de janela de outros apps nem conteudo de arquivos (SEC-18 b): isso e disciplina de quem chama
//! `tracing`, e o nivel `debug` so liga mais eventos desses mesmos tipos.

use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use tracing_subscriber::fmt::MakeWriter;

/// Tamanho maximo de cada arquivo e quantidade de arquivos (atual + 2 antigos).
pub const MAX_LOG_BYTES: u64 = 5 * 1024 * 1024;
pub const LOG_FILES: usize = 3;
const BASE_NAME: &str = "sofa";

/// Escritor com rotacao: `sofa.log` (atual), `sofa.1.log`, `sofa.2.log`.
pub struct RotatingFile {
    dir: PathBuf,
    max_bytes: u64,
    files: usize,
    current: Option<File>,
    written: u64,
}

fn name(index: usize) -> String {
    if index == 0 {
        format!("{BASE_NAME}.log")
    } else {
        format!("{BASE_NAME}.{index}.log")
    }
}

impl RotatingFile {
    pub fn open(dir: impl Into<PathBuf>, max_bytes: u64, files: usize) -> io::Result<Self> {
        let dir = dir.into();
        fs::create_dir_all(&dir)?;
        let mut me = Self {
            dir,
            max_bytes,
            files: files.max(1),
            current: None,
            written: 0,
        };
        me.open_current()?;
        Ok(me)
    }

    fn open_current(&mut self) -> io::Result<()> {
        let path = self.dir.join(name(0));
        let file = OpenOptions::new().create(true).append(true).open(&path)?;
        self.written = file.metadata()?.len();
        self.current = Some(file);
        Ok(())
    }

    /// `sofa.(n-2).log` -> `sofa.(n-1).log` ... `sofa.log` -> `sofa.1.log`; o mais antigo sai.
    fn rotate(&mut self) -> io::Result<()> {
        self.current = None; // fecha o handle antes de renomear
        let oldest = self.files - 1;
        if oldest >= 1 {
            let _ = fs::remove_file(self.dir.join(name(oldest)));
            for i in (0..oldest).rev() {
                let from = self.dir.join(name(i));
                if from.exists() {
                    fs::rename(&from, self.dir.join(name(i + 1)))?;
                }
            }
        } else {
            fs::remove_file(self.dir.join(name(0)))?;
        }
        self.open_current()
    }
}

impl Write for RotatingFile {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        if self.written + buf.len() as u64 > self.max_bytes && self.written > 0 {
            self.rotate()?;
        }
        let file = self
            .current
            .as_mut()
            .ok_or_else(|| io::Error::other("log fechado"))?;
        let n = file.write(buf)?;
        self.written += n as u64;
        Ok(n)
    }

    fn flush(&mut self) -> io::Result<()> {
        match self.current.as_mut() {
            Some(f) => f.flush(),
            None => Ok(()),
        }
    }
}

/// Entrega o escritor compartilhado ao `tracing-subscriber`.
#[derive(Clone)]
pub struct SharedLog(Arc<Mutex<RotatingFile>>);

pub struct SharedLogGuard<'a>(std::sync::MutexGuard<'a, RotatingFile>);

impl Write for SharedLogGuard<'_> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.0.write(buf)
    }
    fn flush(&mut self) -> io::Result<()> {
        self.0.flush()
    }
}

impl<'a> MakeWriter<'a> for SharedLog {
    type Writer = SharedLogGuard<'a>;
    fn make_writer(&'a self) -> Self::Writer {
        // Mutex envenenado so acontece se um panico ocorreu escrevendo log: segue com o que ha.
        SharedLogGuard(
            self.0
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner),
        )
    }
}

/// Nivel de log pedido (`advanced.diagnostics.level`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Level {
    Info,
    Debug,
}

/// Instala o `tracing` global gravando em `log_dir`. Chamar uma vez, o mais cedo possivel.
/// Falha de E/S nao derruba o launcher: devolve o erro e o app segue sem arquivo de log.
pub fn init(log_dir: &Path, level: Level) -> io::Result<()> {
    let writer = SharedLog(Arc::new(Mutex::new(RotatingFile::open(
        log_dir,
        MAX_LOG_BYTES,
        LOG_FILES,
    )?)));
    let max = match level {
        Level::Info => tracing::Level::INFO,
        Level::Debug => tracing::Level::DEBUG,
    };
    tracing_subscriber::fmt()
        .with_writer(writer)
        .with_ansi(false)
        .with_max_level(max)
        .with_target(false)
        .try_init()
        .map_err(io::Error::other)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};

    struct TempDir(PathBuf);
    impl TempDir {
        fn new() -> Self {
            static N: AtomicU32 = AtomicU32::new(0);
            let p = std::env::temp_dir().join(format!(
                "sofa-log-{}-{}",
                std::process::id(),
                N.fetch_add(1, Ordering::SeqCst)
            ));
            fs::create_dir_all(&p).unwrap();
            Self(p)
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
    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn limites_de_producao_sao_5_mb_por_3_arquivos() {
        assert_eq!(MAX_LOG_BYTES, 5 * 1024 * 1024);
        assert_eq!(LOG_FILES, 3);
    }

    #[test]
    fn rotaciona_e_mantem_no_maximo_tres_arquivos_dentro_do_limite() {
        let d = TempDir::new();
        let mut w = RotatingFile::open(&d.0, 1000, 3).unwrap();
        // 10 linhas de 300 bytes = 3000 bytes: passa do limite varias vezes.
        for i in 0..10u8 {
            let line = vec![b'a' + i; 299];
            w.write_all(&line).unwrap();
            w.write_all(b"\n").unwrap();
        }
        w.flush().unwrap();
        assert_eq!(
            d.names(),
            vec![
                "sofa.1.log".to_owned(),
                "sofa.2.log".to_owned(),
                "sofa.log".to_owned()
            ]
        );
        for n in d.names() {
            assert!(
                fs::metadata(d.0.join(&n)).unwrap().len() <= 1000,
                "{n} passou do limite"
            );
        }
        // O mais recente fica no arquivo atual e o conteudo mais antigo foi descartado.
        let current = fs::read(d.0.join("sofa.log")).unwrap();
        assert!(current.contains(&b'j'), "ultima linha esta no atual");
        let all: Vec<u8> = d
            .names()
            .iter()
            .flat_map(|n| fs::read(d.0.join(n)).unwrap())
            .collect();
        assert!(
            !all.contains(&b'a'),
            "a linha mais antiga foi rotacionada para fora"
        );
    }

    #[test]
    fn reabrir_continua_o_arquivo_atual() {
        let d = TempDir::new();
        {
            let mut w = RotatingFile::open(&d.0, 1000, 3).unwrap();
            w.write_all(b"primeira\n").unwrap();
        }
        let mut w = RotatingFile::open(&d.0, 1000, 3).unwrap();
        w.write_all(b"segunda\n").unwrap();
        assert_eq!(
            fs::read_to_string(d.0.join("sofa.log")).unwrap(),
            "primeira\nsegunda\n"
        );
    }

    #[test]
    fn um_unico_arquivo_apaga_e_recomeca() {
        let d = TempDir::new();
        let mut w = RotatingFile::open(&d.0, 100, 1).unwrap();
        w.write_all(&[b'x'; 90]).unwrap();
        w.write_all(&[b'y'; 50]).unwrap();
        assert_eq!(d.names(), vec!["sofa.log".to_owned()]);
        assert_eq!(fs::read(d.0.join("sofa.log")).unwrap(), vec![b'y'; 50]);
    }
}
