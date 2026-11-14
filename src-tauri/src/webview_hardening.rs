//! Endurecimento da WebView (SEC-02, INV-02): navegacao so para a origem do app, nenhuma janela
//! nova, e no release sem DevTools, aceleradores do navegador, menu de contexto e barra de status.

use tauri::webview::NewWindowResponse;
use tauri::{Runtime, Url, WebviewWindow, WebviewWindowBuilder};

/// Origens que a WebView pode navegar. Comparacao exata de esquema, host e porta: nada de
/// prefixo, `userinfo` ou subdominio.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NavigationPolicy {
    origins: Vec<(String, String, Option<u16>)>,
}

impl NavigationPolicy {
    /// Origem do app empacotado no Windows: `http://tauri.localhost`.
    pub const APP_ORIGIN: &'static str = "http://tauri.localhost";
    /// Servidor de desenvolvimento (so em build de debug).
    pub const DEV_ORIGIN: &'static str = "http://127.0.0.1:5173";

    pub fn new(allowed: &[&str]) -> Self {
        let origins = allowed
            .iter()
            .filter_map(|s| Url::parse(s).ok())
            .filter_map(|u| {
                Some((
                    u.scheme().to_owned(),
                    u.host_str()?.to_owned(),
                    u.port_or_known_default(),
                ))
            })
            .collect();
        Self { origins }
    }

    /// Politica da build atual: so o app no release; app + dev server no debug.
    pub fn for_build() -> Self {
        if cfg!(debug_assertions) {
            Self::new(&[Self::APP_ORIGIN, Self::DEV_ORIGIN])
        } else {
            Self::new(&[Self::APP_ORIGIN])
        }
    }

    pub fn allows(&self, url: &Url) -> bool {
        if !url.username().is_empty() || url.password().is_some() {
            return false; // `http://tauri.localhost@evil.com`, mesmo que o host confira
        }
        let Some(host) = url.host_str() else {
            return false; // `data:`, `javascript:`, `file:` sem host, `about:`
        };
        let key = (
            url.scheme().to_owned(),
            host.to_owned(),
            url.port_or_known_default(),
        );
        self.origins.contains(&key)
    }
}

/// Liga os handlers de navegacao e de nova janela no builder (antes de `build`).
pub fn lock_navigation<'a, R: Runtime, M: tauri::Manager<R>>(
    builder: WebviewWindowBuilder<'a, R, M>,
) -> WebviewWindowBuilder<'a, R, M> {
    let policy = NavigationPolicy::for_build();
    builder
        .on_navigation(move |url| {
            let ok = policy.allows(url);
            if !ok {
                tracing::warn!(
                    scheme = url.scheme(),
                    "navegacao recusada pela politica de origem"
                );
            }
            ok
        })
        // `window.open` e `target=_blank` nunca abrem janela (SEC-02).
        .on_new_window(|_url, _features| NewWindowResponse::Deny)
}

/// Desliga os recursos do navegador na WebView2. So no release: no debug o desenvolvedor mantem
/// DevTools e F5/F12.
pub fn disable_browser_features<R: Runtime>(window: &WebviewWindow<R>) {
    if cfg!(debug_assertions) {
        return;
    }
    let result = window.with_webview(|webview| {
        use webview2_com::Microsoft::Web::WebView2::Win32::{
            ICoreWebView2Settings, ICoreWebView2Settings3,
        };
        use windows_core::Interface;

        // SAFETY: chamadas COM no thread da UI (o Tauri executa `with_webview` la), sobre os
        // objetos da propria WebView; nenhum ponteiro bruto de fora.
        let applied: windows_core::Result<()> = unsafe {
            (|| {
                let core = webview.controller().CoreWebView2()?;
                let settings: ICoreWebView2Settings = core.Settings()?;
                settings.SetAreDevToolsEnabled(false)?;
                settings.SetAreDefaultContextMenusEnabled(false)?;
                settings.SetIsStatusBarEnabled(false)?;
                let settings3: ICoreWebView2Settings3 = settings.cast()?;
                settings3.SetAreBrowserAcceleratorKeysEnabled(false)?;
                Ok(())
            })()
        };
        if let Err(error) = applied {
            // Falhar aqui nao deve derrubar o launcher, mas precisa aparecer no log.
            tracing::error!(%error, "falha ao desligar recursos do navegador na WebView2");
        }
    });
    if let Err(error) = result {
        tracing::error!(%error, "with_webview indisponivel; WebView sem endurecimento");
    }
}
