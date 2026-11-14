//! Energia (RN-14, RN-15, SEC-27, CT-07): a acao so executa com um ticket emitido por
//! `power_prepare`, valido por 30 s, de uso unico e preso a `PowerAction`. Um `prepare` novo
//! invalida o anterior. O dominio nao gera aleatoriedade nem le relogio: o token chega pronto (port
//! `RandomSource`) e o tempo e parametro.

use super::time::Millis;

/// Validade do ticket (SEC-27).
pub const TICKET_TTL_MS: u64 = 30_000;
/// Bytes aleatorios de cada ticket: 256 bits (o contrato exige >= 128).
pub const TICKET_RANDOM_BYTES: usize = 32;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PowerAction {
    Shutdown,
    Restart,
    Suspend,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TicketError {
    /// Expirado, reutilizado, desconhecido ou invalidado por um `prepare` mais novo.
    Invalid,
}

#[derive(Debug)]
struct Issued {
    token: String,
    action: PowerAction,
    issued_at: Millis,
}

/// No maximo um ticket em aberto: emitir outro substitui o anterior (CT-07).
#[derive(Debug, Default)]
pub struct TicketBook {
    current: Option<Issued>,
}

impl TicketBook {
    pub fn new() -> Self {
        Self::default()
    }

    /// Registra o ticket e invalida qualquer anterior.
    pub fn issue(&mut self, token: String, action: PowerAction, now: Millis) {
        self.current = Some(Issued {
            token,
            action,
            issued_at: now,
        });
    }

    /// Resgata o ticket. Valido: devolve a acao **presa a ele** e o consome (uso unico). Vencido:
    /// consome e recusa. Token diferente do vigente: recusa sem tocar no vigente (um chute errado
    /// nao cancela a confirmacao em andamento).
    pub fn redeem(&mut self, token: &str, now: Millis) -> Result<PowerAction, TicketError> {
        let Some(issued) = &self.current else {
            return Err(TicketError::Invalid);
        };
        if !constant_time_eq(issued.token.as_bytes(), token.as_bytes()) {
            return Err(TicketError::Invalid);
        }
        let issued = self.current.take().ok_or(TicketError::Invalid)?;
        if now.since(issued.issued_at) > TICKET_TTL_MS {
            return Err(TicketError::Invalid);
        }
        Ok(issued.action)
    }
}

/// Comparacao sem saida antecipada (o ticket e segredo de curta duracao).
fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    a.iter().zip(b).fold(0u8, |acc, (x, y)| acc | (x ^ y)) == 0
}

/// `^[A-Za-z0-9_-]{22,64}$` (forma do ticket no contrato).
pub fn is_well_formed(token: &str) -> bool {
    (22..=64).contains(&token.len())
        && token
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
}

/// base64url sem preenchimento: 32 bytes viram 43 caracteres do alfabeto do ticket.
pub fn encode_ticket(bytes: &[u8]) -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let b = [
            chunk[0],
            chunk.get(1).copied().unwrap_or(0),
            chunk.get(2).copied().unwrap_or(0),
        ];
        let n = (u32::from(b[0]) << 16) | (u32::from(b[1]) << 8) | u32::from(b[2]);
        let take = chunk.len() + 1; // 1 byte = 2 chars, 2 = 3, 3 = 4
        for i in 0..take {
            out.push(char::from(ALPHABET[((n >> (18 - 6 * i)) & 0x3f) as usize]));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const T0: Millis = Millis(1_000);

    #[test]
    fn ticket_vale_uma_vez_e_devolve_a_acao_presa_a_ele() {
        let mut book = TicketBook::new();
        book.issue("A".repeat(43), PowerAction::Restart, T0);
        assert_eq!(
            book.redeem(&"A".repeat(43), T0 + 5_000),
            Ok(PowerAction::Restart)
        );
        assert_eq!(
            book.redeem(&"A".repeat(43), T0 + 5_001),
            Err(TicketError::Invalid),
            "reutilizado"
        );
    }

    #[test]
    fn expira_depois_de_30_segundos() {
        for (decorrido, ok) in [(0, true), (29_999, true), (30_000, true), (30_001, false)] {
            let mut book = TicketBook::new();
            book.issue("B".repeat(43), PowerAction::Shutdown, T0);
            let r = book.redeem(&"B".repeat(43), T0 + decorrido);
            assert_eq!(r.is_ok(), ok, "{decorrido} ms");
        }
    }

    #[test]
    fn expirado_e_consumido() {
        let mut book = TicketBook::new();
        book.issue("C".repeat(43), PowerAction::Suspend, T0);
        assert!(book.redeem(&"C".repeat(43), T0 + 31_000).is_err());
        // Nem dentro do prazo (relogio andando para tras) volta a valer.
        assert!(book.redeem(&"C".repeat(43), T0).is_err());
    }

    /// CT-07: um `prepare` novo invalida o anterior.
    #[test]
    fn novo_prepare_invalida_o_anterior() {
        let mut book = TicketBook::new();
        book.issue("D".repeat(43), PowerAction::Shutdown, T0);
        book.issue("E".repeat(43), PowerAction::Suspend, T0 + 1_000);
        assert_eq!(
            book.redeem(&"D".repeat(43), T0 + 2_000),
            Err(TicketError::Invalid)
        );
        assert_eq!(
            book.redeem(&"E".repeat(43), T0 + 2_000),
            Ok(PowerAction::Suspend)
        );
    }

    #[test]
    fn token_desconhecido_nao_cancela_o_vigente() {
        let mut book = TicketBook::new();
        assert_eq!(
            book.redeem(&"F".repeat(43), T0),
            Err(TicketError::Invalid),
            "sem ticket nenhum"
        );
        book.issue("G".repeat(43), PowerAction::Restart, T0);
        assert_eq!(book.redeem(&"H".repeat(43), T0), Err(TicketError::Invalid));
        assert_eq!(book.redeem("G", T0), Err(TicketError::Invalid), "prefixo");
        assert_eq!(
            book.redeem(&"G".repeat(43), T0 + 1),
            Ok(PowerAction::Restart)
        );
    }

    #[test]
    fn forma_do_ticket() {
        assert!(is_well_formed(&"a".repeat(22)));
        assert!(is_well_formed(&"Z_-9".repeat(16)));
        assert!(!is_well_formed(&"a".repeat(21)));
        assert!(!is_well_formed(&"a".repeat(65)));
        assert!(!is_well_formed(&format!("{}!", "a".repeat(30))));
        assert!(!is_well_formed(&format!("{}=", "a".repeat(30))));
        assert!(!is_well_formed(""));
    }

    /// Vetores do RFC 4648 secao 10 (sem preenchimento), mais o alfabeto url-safe.
    #[test]
    fn base64url_confere_com_o_rfc() {
        for (entrada, saida) in [
            (&b""[..], ""),
            (b"f", "Zg"),
            (b"fo", "Zm8"),
            (b"foo", "Zm9v"),
            (b"foob", "Zm9vYg"),
            (b"fooba", "Zm9vYmE"),
            (b"foobar", "Zm9vYmFy"),
            (&[0xfb, 0xff, 0xbf], "-_-_"),
            (&[0xff, 0xff, 0xff], "____"),
        ] {
            assert_eq!(encode_ticket(entrada), saida);
        }
        let t = encode_ticket(&[0xAB; TICKET_RANDOM_BYTES]);
        assert_eq!(t.len(), 43);
        assert!(is_well_formed(&t));
    }
}
