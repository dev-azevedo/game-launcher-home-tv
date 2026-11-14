//! Como o processo foi chamado (SEC-19, SEC-31, BE-35, BE-40, BE-41): a linha de comando e
//! tratada **so aqui**, so no inicio do proprio processo, e aceita no maximo uma bandeira, sem
//! valor. Qualquer outra coisa e ignorada. Nada vem de uma segunda instancia (o callback do
//! plugin de instancia unica descarta `argv` e `cwd`).

use super::autostart::AUTOSTART_ARG;

/// Bandeira do modo de remocao de dados (so o desinstalador a usa, SEC-31).
pub const PURGE_ARG: &str = "--purge-user-data";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StartMode {
    /// Abertura normal (atalho, menu Iniciar, argumentos desconhecidos ignorados).
    Normal,
    /// A entrada de inicio com o Windows (`--autostart`, unico argumento): so marca o log (M-02).
    Autostart,
    /// `--purge-user-data` como unico argumento: modo de remocao, sem janela nem WebView2.
    Purge,
    /// `--purge-user-data` junto com qualquer outro argumento: recusado, nada e apagado.
    PurgeRefused,
}

/// `args` sao os argumentos **depois** do nome do programa.
pub fn parse_start_mode<S: AsRef<str>>(args: &[S]) -> StartMode {
    match args {
        [] => StartMode::Normal,
        [only] if only.as_ref() == AUTOSTART_ARG => StartMode::Autostart,
        [only] if only.as_ref() == PURGE_ARG => StartMode::Purge,
        _ if args.iter().any(|a| a.as_ref() == PURGE_ARG) => StartMode::PurgeRefused,
        _ => StartMode::Normal,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mode(args: &[&str]) -> StartMode {
        parse_start_mode(args)
    }

    #[test]
    fn tabela_de_argumentos() {
        for (args, esperado) in [
            (&[][..], StartMode::Normal),
            (&["--autostart"], StartMode::Autostart),
            (&["--purge-user-data"], StartMode::Purge),
            // extras: purge recusado; o resto e ignorado e abre normal
            (&["--purge-user-data", "x"], StartMode::PurgeRefused),
            (&["x", "--purge-user-data"], StartMode::PurgeRefused),
            (
                &["--purge-user-data", "--purge-user-data"],
                StartMode::PurgeRefused,
            ),
            (
                &["--purge-user-data", "--autostart"],
                StartMode::PurgeRefused,
            ),
            (
                &["--autostart", "--purge-user-data"],
                StartMode::PurgeRefused,
            ),
            (&["--autostart", "x"], StartMode::Normal),
            (&["--autostart=1"], StartMode::Normal),
            (&["--Autostart"], StartMode::Normal),
            (&["--purge-user-data=C:\\"], StartMode::Normal),
            (&["--purge-user-data "], StartMode::Normal),
            (
                &["C:\\Windows\\System32\\calc.exe", "--launch", "x"],
                StartMode::Normal,
            ),
            (&["--launch", "steam:620"], StartMode::Normal),
            (&[""], StartMode::Normal),
        ] {
            assert_eq!(mode(args), esperado, "{args:?}");
        }
    }

    #[test]
    fn aceita_string_e_str() {
        assert_eq!(
            parse_start_mode(&["--autostart".to_owned()]),
            StartMode::Autostart
        );
    }
}
