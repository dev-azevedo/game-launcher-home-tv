//! Monitor do launcher (RN-20, CA-66, CA-67, E-08, E-09): o escolhido nas configuracoes, se estiver
//! conectado; senao o principal do Windows. A preferencia nunca e apagada por o monitor sumir.
//! Funcoes puras sobre a lista enumerada pelo adapter (`DisplayTopology`).

const MAX_MONITOR_ID_CHARS: usize = 256;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

/// Um monitor ligado. `id` e estavel entre sessoes (`monitorDevicePath` do Windows) e opaco para
/// a UI; `name` ainda nao esta normalizado (quem monta o DTO normaliza, SEC-07).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Monitor {
    pub id: String,
    pub name: String,
    pub primary: bool,
    pub rect: Rect,
}

/// `MonitorId` aceito: 1 a 256 caracteres, sem controle.
pub fn is_valid_monitor_id(id: &str) -> bool {
    let n = id.chars().count();
    (1..=MAX_MONITOR_ID_CHARS).contains(&n) && !id.chars().any(char::is_control)
}

/// RN-20: monitor preferido se conectado; senao o principal; senao (lista sem principal, caso
/// defensivo) o primeiro. Lista vazia: `None`.
pub fn choose_monitor<'a>(preferred: Option<&str>, monitors: &'a [Monitor]) -> Option<&'a Monitor> {
    preferred
        .and_then(|id| monitors.iter().find(|m| m.id == id))
        .or_else(|| monitors.iter().find(|m| m.primary))
        .or_else(|| monitors.first())
}

/// Depois de uma mudanca de topologia: o launcher **fica** onde esta enquanto aquele monitor
/// existir (E-09: a TV volta e ele continua na mesma tela); so se o monitor atual sumiu vale
/// `choose_monitor` (CA-67). O monitor preferido que reconecta so volta a valer na proxima
/// abertura.
pub fn reconcile<'a>(
    current: Option<&str>,
    preferred: Option<&str>,
    monitors: &'a [Monitor],
) -> Option<&'a Monitor> {
    current
        .and_then(|id| monitors.iter().find(|m| m.id == id))
        .or_else(|| choose_monitor(preferred, monitors))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn m(id: &str, primary: bool) -> Monitor {
        Monitor {
            id: id.into(),
            name: format!("Tela {id}"),
            primary,
            rect: Rect {
                x: if primary { 0 } else { 1920 },
                y: 0,
                width: 1920,
                height: 1080,
            },
        }
    }

    fn id_of(m: Option<&Monitor>) -> Option<&str> {
        m.map(|m| m.id.as_str())
    }

    #[test]
    fn preferido_conectado_vence_o_principal() {
        let list = [m("A", true), m("TV", false)];
        assert_eq!(id_of(choose_monitor(Some("TV"), &list)), Some("TV"));
    }

    #[test]
    fn sem_preferencia_ou_preferido_ausente_cai_no_principal() {
        let list = [m("A", true), m("TV", false)];
        assert_eq!(id_of(choose_monitor(None, &list)), Some("A"));
        assert_eq!(id_of(choose_monitor(Some("SUMIU"), &list)), Some("A")); // CA-67
        let outra_ordem = [m("TV", false), m("A", true)];
        assert_eq!(
            id_of(choose_monitor(Some("SUMIU"), &outra_ordem)),
            Some("A")
        );
    }

    #[test]
    fn lista_sem_principal_usa_o_primeiro_e_vazia_nao_tem_monitor() {
        let list = [m("X", false), m("Y", false)];
        assert_eq!(id_of(choose_monitor(None, &list)), Some("X"));
        assert_eq!(choose_monitor(Some("Y"), &[]), None);
        assert_eq!(choose_monitor(None, &[]), None);
    }

    /// CA-66 e CA-67 em sequencia, o roteiro do teste manual com duas telas.
    #[test]
    fn roteiro_de_duas_telas() {
        let preferido = Some("TV");
        let duas = [m("A", true), m("TV", false)];
        let so_a = [m("A", true)];

        // Abre com as duas ligadas: vai para a TV.
        let atual = choose_monitor(preferido, &duas).map(|x| x.id.clone());
        assert_eq!(atual.as_deref(), Some("TV"));
        // Desplugar a TV: cai no principal.
        let atual = reconcile(atual.as_deref(), preferido, &so_a).map(|x| x.id.clone());
        assert_eq!(atual.as_deref(), Some("A"));
        // Religar a TV com o Sofa aberto: continua no principal (E-09), nao pula sozinho.
        let atual = reconcile(atual.as_deref(), preferido, &duas).map(|x| x.id.clone());
        assert_eq!(atual.as_deref(), Some("A"));
        // Fechar e abrir de novo: volta ao escolhido, porque a preferencia nunca saiu.
        assert_eq!(id_of(choose_monitor(preferido, &duas)), Some("TV"));
    }

    #[test]
    fn monitor_atual_que_continua_ligado_nao_muda() {
        let duas = [m("A", true), m("TV", false)];
        assert_eq!(id_of(reconcile(Some("TV"), None, &duas)), Some("TV"));
        assert_eq!(id_of(reconcile(None, None, &duas)), Some("A"));
    }

    #[test]
    fn forma_do_monitor_id() {
        assert!(is_valid_monitor_id(
            "\\\\?\\DISPLAY#GSM5B09#4&1234#{e6f07b5f}"
        ));
        assert!(is_valid_monitor_id(&"m".repeat(256)));
        assert!(!is_valid_monitor_id(""));
        assert!(!is_valid_monitor_id(&"m".repeat(257)));
        assert!(!is_valid_monitor_id("a\u{0}b"));
        assert!(!is_valid_monitor_id("a\nb"));
    }
}
