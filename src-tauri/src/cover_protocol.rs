//! Protocolo `cover://` (BE-28, SEC-08, CT-06). O handler so aceita um `GameId` **existente na
//! biblioteca em memoria**: qualquer outra coisa (`..`, id inexistente, id malformado) responde
//! 404 e a UI mostra o placeholder (CA-24). O `GameId` nunca compoe caminho em disco: e so chave
//! de busca. Os bytes servidos saem do `CoverCache` (reencodados pelo nucleo); o WebView nunca
//! recebe o arquivo de terceiro. Nao ha `assetProtocol`.

use launcher_core::domain::library::GameId;
use tauri::http::{header, Request, Response, StatusCode};

use crate::state::AppState;

pub const SCHEME: &str = "cover";

fn hex(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}

/// `%3A` -> `:`. Sequencia invalida ou resultado que nao e UTF-8 = `None`.
fn percent_decode(s: &str) -> Option<String> {
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%' {
            let hi = hex(*b.get(i + 1)?)?;
            let lo = hex(*b.get(i + 2)?)?;
            out.push(hi * 16 + lo);
            i += 3;
        } else {
            out.push(b[i]);
            i += 1;
        }
    }
    String::from_utf8(out).ok()
}

/// O `GameId` do pedido. No Windows o WebView2 entrega `http://cover.localhost/<id>` (o wry
/// converte de/para `cover://localhost/<id>`); o id vem no caminho, com `:` possivelmente
/// codificado. Consulta e fragmento sao ignorados.
pub fn game_id_of(path: &str) -> Option<GameId> {
    let raw = path.strip_prefix('/')?;
    let decoded = percent_decode(raw)?;
    GameId::parse(&decoded).ok()
}

fn status(code: StatusCode) -> Response<Vec<u8>> {
    Response::builder()
        .status(code)
        .header(header::CONTENT_LENGTH, "0")
        .body(Vec::new())
        .unwrap_or_default()
}

/// Atende um pedido de `cover://`. Roda fora da thread da janela (decodificar e reencodar pode
/// demorar na primeira vez).
pub fn respond(state: &AppState, request: &Request<Vec<u8>>) -> Response<Vec<u8>> {
    if request.method() != tauri::http::Method::GET {
        return status(StatusCode::NOT_FOUND);
    }
    let Some(id) = game_id_of(request.uri().path()) else {
        return status(StatusCode::NOT_FOUND);
    };
    let (Some(library), Some(covers)) = (state.library.get(), state.covers.get()) else {
        return status(StatusCode::NOT_FOUND);
    };
    // Id fora da biblioteca = 404, sem tocar em disco.
    let Some(source) = library.cover_source(&id) else {
        return status(StatusCode::NOT_FOUND);
    };
    let Some(image) = covers.image(&source) else {
        return status(StatusCode::NOT_FOUND);
    };
    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, image.mime)
        .header(header::CONTENT_LENGTH, image.bytes.len().to_string())
        .header(header::CACHE_CONTROL, "private, max-age=600")
        .header("X-Content-Type-Options", "nosniff")
        .body(image.bytes)
        .unwrap_or_default()
}
