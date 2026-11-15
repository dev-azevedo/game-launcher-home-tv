//! Comparacao de caminhos Windows, num lugar so (ARQ-BUG-04). Tres regras de seguranca dependiam
//! de "este caminho esta dentro daquela raiz, por componente, sem diferenca de caixa": a politica
//! de sessao (SEC-17), o alvo do autostart (DR-04) e as travas do purge (SEC-25). Uma correcao
//! (prefixo `\\?\`, barra final, aspas) agora vale para as tres.
//!
//! Tudo aqui e texto puro: nao toca o disco, nao resolve `..` nem links. Quem chama entrega
//! caminho canonico; `.` e `..` em qualquer componente tornam o caminho "fora" (falha fechada).

/// Chave de comparacao: sem aspas nas pontas, sem prefixo `\\?\`, barras iguais, sem barra final
/// e sem diferenca de caixa (o NTFS nao diferencia).
pub fn path_key(path: &str) -> String {
    let t = path.trim().trim_matches('"');
    let t = t.strip_prefix(r"\\?\").unwrap_or(t);
    t.replace('/', "\\").trim_end_matches('\\').to_lowercase()
}

/// Componentes em minusculas, ignorando separadores repetidos, aspas nas pontas e o prefixo
/// `\\?\`. `None` se algum componente e `.` ou `..`.
fn components(path: &str) -> Option<Vec<String>> {
    let t = path.trim().trim_matches('"');
    let t = t.strip_prefix(r"\\?\").unwrap_or(t);
    let mut out = Vec::new();
    for c in t.split(['\\', '/']).filter(|c| !c.is_empty()) {
        if c == "." || c == ".." {
            return None;
        }
        out.push(c.to_lowercase());
    }
    Some(out)
}

/// `true` se `path` e `root` ou esta dentro dele, componente a componente (`C:\Jogos2` nao esta
/// em `C:\Jogos`). Raiz vazia nunca confina.
pub fn path_is_within(root: &str, path: &str) -> bool {
    match (components(root), components(path)) {
        (Some(r), Some(p)) => !r.is_empty() && p.len() >= r.len() && p[..r.len()] == r[..],
        _ => false,
    }
}

/// Como `path_is_within`, mas `path` precisa estar **abaixo** de `root` (igual nao vale).
pub fn path_is_strictly_within(root: &str, path: &str) -> bool {
    match (components(root), components(path)) {
        (Some(r), Some(p)) => !r.is_empty() && p.len() > r.len() && p[..r.len()] == r[..],
        _ => false,
    }
}

/// `child` e `parent\<um componente>`.
pub fn is_direct_child(parent: &str, child: &str) -> bool {
    match (components(parent), components(child)) {
        (Some(p), Some(c)) => !p.is_empty() && c.len() == p.len() + 1 && c[..p.len()] == p[..],
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dentro_da_raiz_por_componente() {
        // (raiz, caminho, esperado)
        let tabela: &[(&str, &str, bool)] = &[
            (r"C:\Jogos", r"C:\Jogos\A\a.exe", true),
            (r"C:\Jogos", r"C:\Jogos", true),
            (r"c:\JOGOS\", r"C:/jogos/a.exe", true), // caixa, barra e barra final
            (r"C:\Jogos", r"C:\\Jogos\\a.exe", true), // barras duplicadas
            (r"C:\Jogos", r"\\?\C:\Jogos\a.exe", true), // prefixo verbatim
            (r"\\?\C:\Jogos", r"C:\Jogos\a.exe", true), // prefixo na raiz
            (r"C:\Jogos", r#""C:\Jogos\a.exe""#, true), // aspas nas pontas
            (r"C:\Jogos", r"C:\Jogos2\a.exe", false), // por componente, nao por prefixo
            (r"C:\Jogos", r"C:\JogosExtra", false),
            (r"C:\Jogos\A", r"C:\Jogos", false), // pai nao esta no filho
            (r"C:\Jogos", r"C:\Jogos\..\Windows\x.exe", false), // sem resolver ..
            (r"C:\Jogos", r"C:\Jogos\.\a.exe", false),
            (r"C:\Jogos\..", r"C:\Jogos\a.exe", false),
            ("", r"C:\Jogos\a.exe", false), // raiz vazia nunca confina
            (r"\\", r"C:\a.exe", false),
            (r"\\?\", r"C:\a.exe", false),
            (r"C:\Jogos", "", false),
            (r"D:\Jogos", r"C:\Jogos\a.exe", false),
        ];
        for (root, path, esperado) in tabela {
            assert_eq!(path_is_within(root, path), *esperado, "{root:?} / {path:?}");
        }
    }

    #[test]
    fn estritamente_abaixo_nao_aceita_a_propria_raiz() {
        assert!(path_is_strictly_within(
            r"C:\Program Files",
            r"C:\Program Files\Sofa\sofa.exe"
        ));
        assert!(path_is_strictly_within(r"c:\pf\", r"\\?\C:\PF\x"));
        assert!(!path_is_strictly_within(
            r"C:\Program Files",
            r"C:\Program Files"
        ));
        assert!(!path_is_strictly_within(
            r"C:\Program Files",
            r"C:\Program Files\"
        ));
        assert!(!path_is_strictly_within(
            r"C:\Program Files",
            r"C:\Program Files Evil\sofa.exe"
        ));
        assert!(!path_is_strictly_within(
            r"C:\Program Files",
            r"C:\Program Files\..\Evil\sofa.exe"
        ));
        assert!(!path_is_strictly_within("", r"C:\x"));
    }

    #[test]
    fn um_nivel_abaixo_por_componente() {
        let lad = r"C:\Users\ana\AppData\Local";
        for ok in [
            r"C:\Users\ana\AppData\Local\app.sofa.launcher",
            r"c:\users\ANA\appdata\local\App.Sofa.Launcher",
            r"\\?\C:\Users\ana\AppData\Local\app.sofa.launcher",
            r"C:\Users\ana\AppData\Local\app.sofa.launcher\",
        ] {
            assert!(is_direct_child(lad, ok), "{ok}");
        }
        for ruim in [
            r"C:\Users\ana\AppData\Local",
            r"C:\Users\ana\AppData\Local\a\b",
            r"C:\Users\ana\AppData\LocalLow\app.sofa.launcher",
            r"C:\Users\ana\AppData\Local\..",
            r"C:\Users\ana\AppData",
            r"D:\Outro\app.sofa.launcher",
            r"C:\Users\ana\AppData\Local\app.sofa.launcher\cache",
            "",
        ] {
            assert!(!is_direct_child(lad, ruim), "{ruim}");
        }
        assert!(!is_direct_child("", r"C:\x"));
    }

    #[test]
    fn chave_de_comparacao() {
        assert_eq!(path_key(r#" "C:/Pasta/A.EXE" "#), r"c:\pasta\a.exe");
        assert_eq!(path_key(r"\\?\C:\Pasta\"), r"c:\pasta");
        assert_eq!(path_key(""), "");
    }
}
