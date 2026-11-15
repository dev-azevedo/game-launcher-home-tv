//! Logs do Sofa em `<pasta de dados>\logs\`: arquivo com rotacao por tamanho, 5 MB x 3 (SEC-18).
//! Os registros so levam IDs, nomes e caminhos de jogo, nunca linha de comando, ambiente, titulos
//! de janela de outros apps nem conteudo de arquivos (SEC-18 b): isso e disciplina de quem chama
//! `tracing`, e o nivel `debug` so liga mais eventos desses mesmos tipos.
//!
//! **Quem registra nunca espera o disco** (INV-04, ARQ-BUG-07): o `tracing` formata o evento na
//! thread de quem o emitiu e entrega a linha pronta a uma fila limitada; uma thread propria grava
//! no `RotatingFile`. Fila cheia descarta a linha (conta e avisa no proximo registro gravado), em
//! vez de segurar a thread de entrada, que le o XInput e fala com o dominio sem I/O.

use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{self, SyncSender, TrySendError};
use std::sync::{Arc, OnceLock};
use std::time::{Duration, Instant};

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

/// Linhas pendentes que a fila aguenta antes de descartar. Um evento tem centenas de bytes: a fila
/// cheia custa poucos MB e so enche se o disco parar.
const QUEUE_CAPACITY: usize = 4096;

enum Msg {
    Line(Vec<u8>),
    /// Confirma, pelo canal dado, que tudo que veio antes ja foi gravado.
    Flush(SyncSender<()>),
}

/// Escritor assincrono para o `tracing-subscriber`: `make_writer` e a escrita nunca bloqueiam.
#[derive(Clone)]
pub struct AsyncLog {
    tx: SyncSender<Msg>,
    dropped: Arc<AtomicU64>,
}

impl AsyncLog {
    /// Sobe a thread de gravacao sobre `sink` e devolve a ponta que registra. A thread termina
    /// quando a ultima ponta some.
    pub fn spawn(sink: impl Write + Send + 'static, capacity: usize) -> io::Result<Self> {
        let (tx, rx) = mpsc::sync_channel::<Msg>(capacity.max(1));
        let dropped = Arc::new(AtomicU64::new(0));
        let lost = Arc::clone(&dropped);
        std::thread::Builder::new()
            .name("log-writer".into())
            .spawn(move || {
                let mut sink = sink;
                for msg in rx {
                    match msg {
                        Msg::Line(line) => {
                            let n = lost.swap(0, Ordering::Relaxed);
                            if n > 0 {
                                let _ = writeln!(
                                    sink,
                                    "WARN log: {n} registro(s) descartado(s), fila cheia"
                                );
                            }
                            // Erro de disco nao derruba nada: o proximo registro tenta de novo.
                            let _ = sink.write_all(&line);
                        }
                        Msg::Flush(ack) => {
                            let _ = sink.flush();
                            let _ = ack.send(());
                        }
                    }
                }
            })?;
        Ok(Self { tx, dropped })
    }

    /// Espera, ate `timeout`, a thread de gravacao esvaziar o que ja estava na fila. Para o
    /// encerramento do app; nunca chamar da thread de entrada.
    pub fn flush(&self, timeout: Duration) -> bool {
        let deadline = Instant::now() + timeout;
        let (ack_tx, ack_rx) = mpsc::sync_channel(1);
        let mut msg = Msg::Flush(ack_tx);
        // Fila cheia: espera abrir vaga, sem passar do prazo.
        loop {
            match self.tx.try_send(msg) {
                Ok(()) => break,
                Err(TrySendError::Full(back)) if Instant::now() < deadline => {
                    msg = back;
                    std::thread::sleep(Duration::from_millis(1));
                }
                Err(_) => return false,
            }
        }
        ack_rx
            .recv_timeout(deadline.saturating_duration_since(Instant::now()))
            .is_ok()
    }

    /// Linhas descartadas desde o ultimo aviso gravado (teste e diagnostico).
    pub fn dropped(&self) -> u64 {
        self.dropped.load(Ordering::Relaxed)
    }
}

/// Uma linha de log em montagem: o `tracing` pode escrever em partes; ao soltar, a linha inteira
/// vai para a fila, sem esperar.
pub struct LogLine<'a> {
    log: &'a AsyncLog,
    buf: Vec<u8>,
}

impl Write for LogLine<'_> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.buf.extend_from_slice(buf);
        Ok(buf.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl Drop for LogLine<'_> {
    fn drop(&mut self) {
        if self.buf.is_empty() {
            return;
        }
        match self
            .log
            .tx
            .try_send(Msg::Line(std::mem::take(&mut self.buf)))
        {
            Ok(()) => {}
            Err(TrySendError::Full(_)) => {
                self.log.dropped.fetch_add(1, Ordering::Relaxed);
            }
            Err(TrySendError::Disconnected(_)) => {}
        }
    }
}

impl<'a> MakeWriter<'a> for AsyncLog {
    type Writer = LogLine<'a>;
    fn make_writer(&'a self) -> Self::Writer {
        LogLine {
            log: self,
            buf: Vec::with_capacity(256),
        }
    }
}

static LOG: OnceLock<AsyncLog> = OnceLock::new();

/// Espera a gravacao do que ja foi registrado (ate `timeout`). Chamar ao sair do app, fora da
/// thread de entrada. `false` se o log nao esta ligado ou a fila nao esvaziou a tempo.
pub fn flush(timeout: Duration) -> bool {
    LOG.get().is_some_and(|log| log.flush(timeout))
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
    let writer = AsyncLog::spawn(
        RotatingFile::open(log_dir, MAX_LOG_BYTES, LOG_FILES)?,
        QUEUE_CAPACITY,
    )?;
    let _ = LOG.set(writer.clone());
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

    /// Destino que trava ate o teste soltar: simula o disco parado.
    struct Gated {
        open: Arc<(std::sync::Mutex<bool>, std::sync::Condvar)>,
        out: Arc<std::sync::Mutex<Vec<u8>>>,
        /// A thread de gravacao ja entrou numa escrita (e esta presa nela).
        entered: Arc<std::sync::atomic::AtomicBool>,
    }
    impl Write for Gated {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            self.entered.store(true, Ordering::SeqCst);
            let (lock, cv) = &*self.open;
            let mut open = lock.lock().unwrap();
            while !*open {
                open = cv.wait(open).unwrap();
            }
            self.out.lock().unwrap().extend_from_slice(buf);
            Ok(buf.len())
        }
        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    #[test]
    fn registrar_nao_espera_o_disco_e_fila_cheia_descarta_e_avisa() {
        let open = Arc::new((std::sync::Mutex::new(false), std::sync::Condvar::new()));
        let out = Arc::new(std::sync::Mutex::new(Vec::new()));
        let entered = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let log = AsyncLog::spawn(
            Gated {
                open: Arc::clone(&open),
                out: Arc::clone(&out),
                entered: Arc::clone(&entered),
            },
            4,
        )
        .unwrap();

        // A primeira linha prende a thread de gravacao no "disco" parado.
        writeln!(log.make_writer(), "linha 0").unwrap();
        while !entered.load(Ordering::SeqCst) {
            std::thread::yield_now();
        }

        // Disco parado: 999 registros voltam na hora (INV-04), sem esperar a thread de gravacao.
        let t0 = std::time::Instant::now();
        for i in 1..1000 {
            let mut w = log.make_writer();
            writeln!(w, "linha {i}").unwrap();
        }
        assert!(
            t0.elapsed() < Duration::from_millis(500),
            "registrar bloqueou: {:?}",
            t0.elapsed()
        );
        assert_eq!(
            log.dropped(),
            995,
            "fila de 4: as outras 995 sao descartadas"
        );

        // Disco volta: o que coube e gravado, e o proximo registro avisa quantos se perderam.
        {
            let (lock, cv) = &*open;
            *lock.lock().unwrap() = true;
            cv.notify_all();
        }
        assert!(log.flush(Duration::from_secs(5)), "esvaziou a fila");
        let mut w = log.make_writer();
        writeln!(w, "depois").unwrap();
        drop(w);
        assert!(log.flush(Duration::from_secs(5)));
        let text = String::from_utf8(out.lock().unwrap().clone()).unwrap();
        assert_eq!(
            text,
            "linha 0\nWARN log: 995 registro(s) descartado(s), fila cheia\n\
             linha 1\nlinha 2\nlinha 3\nlinha 4\ndepois\n"
        );
        assert_eq!(log.dropped(), 0, "o aviso zera a contagem");
    }

    #[test]
    fn linha_escrita_em_partes_chega_inteira_e_em_ordem() {
        let d = TempDir::new();
        let log = AsyncLog::spawn(
            RotatingFile::open(&d.0, MAX_LOG_BYTES, LOG_FILES).unwrap(),
            64,
        )
        .unwrap();
        for i in 0..20 {
            let mut w = log.make_writer();
            w.write_all(b"parte um, ").unwrap();
            writeln!(w, "parte dois {i}").unwrap();
        }
        assert!(log.flush(Duration::from_secs(5)));
        let text = fs::read_to_string(d.0.join("sofa.log")).unwrap();
        let lines: Vec<&str> = text.lines().collect();
        assert_eq!(lines.len(), 20);
        for (i, l) in lines.iter().enumerate() {
            assert_eq!(*l, format!("parte um, parte dois {i}"));
        }
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
