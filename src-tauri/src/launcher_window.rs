//! `LauncherWindow` sobre a janela `main` do Tauri (BE-20, BE-21, resultado do SP-06): durante a
//! sessao a janela some e o WebView2 dorme; na volta acorda. **Nao** esvazia o DOM nem navega para
//! `about:blank` (a retomada iria de ~30 ms para ~555 ms sem ganho de memoria).
//!
//! Ordem do SP-06:
//! - esconder: `hide` -> `IsVisible(false)` -> `MemoryUsageTargetLevel(LOW)` -> `TrySuspend`
//!   (o `TrySuspend` exige `IsVisible = false` antes);
//! - mostrar: `Resume` -> `MemoryUsageTargetLevel(NORMAL)` -> `IsVisible(true)` -> `show`.
//!
//! O Tauri executa `with_webview` na thread da UI, em ordem; por isso as chamadas daqui nao
//! bloqueiam quem chama (a escada roda em thread de trabalho e a thread da janela nunca espera).

use launcher_core::application::ports::{LauncherPlacement, LauncherWindow};
use launcher_core::domain::display::Monitor;
use tauri::{AppHandle, Manager, PhysicalPosition, PhysicalSize, Runtime};

use crate::state::AppState;
use crate::window::MAIN_LABEL;

pub struct TauriLauncherWindow<R: Runtime> {
    app: AppHandle<R>,
}

impl<R: Runtime> TauriLauncherWindow<R> {
    pub fn new(app: AppHandle<R>) -> Self {
        Self { app }
    }
}

impl<R: Runtime> LauncherWindow for TauriLauncherWindow<R> {
    fn hide_and_suspend(&self) {
        let Some(window) = self.app.get_webview_window(MAIN_LABEL) else {
            return;
        };
        if let Some(display) = self.app.state::<AppState>().display.get() {
            display.on_launcher_hidden();
        }
        if let Err(error) = window.hide() {
            tracing::warn!(%error, "nao conseguiu esconder a janela");
        }
        let queued = window.with_webview(|webview| {
            use webview2_com::Microsoft::Web::WebView2::Win32::{
                ICoreWebView2_19, ICoreWebView2_3, COREWEBVIEW2_MEMORY_USAGE_TARGET_LEVEL_LOW,
            };
            use windows_core::Interface;

            // SAFETY: chamadas COM na thread da UI sobre os objetos da propria WebView.
            let result: windows_core::Result<()> = unsafe {
                (|| {
                    let controller = webview.controller();
                    controller.SetIsVisible(false)?;
                    let core = controller.CoreWebView2()?;
                    if let Ok(c19) = core.cast::<ICoreWebView2_19>() {
                        let _ = c19
                            .SetMemoryUsageTargetLevel(COREWEBVIEW2_MEMORY_USAGE_TARGET_LEVEL_LOW);
                    }
                    let c3: ICoreWebView2_3 = core.cast()?;
                    let handler = webview2_com::TrySuspendCompletedHandler::create(Box::new(
                        |result, suspended| {
                            tracing::debug!(ok = result.is_ok(), suspended, "WebView2 suspenso");
                            Ok(())
                        },
                    ));
                    c3.TrySuspend(&handler)?;
                    Ok(())
                })()
            };
            if let Err(error) = result {
                tracing::warn!(%error, "WebView2 nao suspendeu; segue so oculto");
            }
        });
        if let Err(error) = queued {
            tracing::warn!(%error, "with_webview indisponivel ao suspender");
        }
    }

    fn show_and_resume(&self) {
        let Some(window) = self.app.get_webview_window(MAIN_LABEL) else {
            return;
        };
        let queued = window.with_webview(|webview| {
            use webview2_com::Microsoft::Web::WebView2::Win32::{
                ICoreWebView2_19, ICoreWebView2_3, COREWEBVIEW2_MEMORY_USAGE_TARGET_LEVEL_NORMAL,
            };
            use windows_core::Interface;

            // SAFETY: chamadas COM na thread da UI sobre os objetos da propria WebView.
            let result: windows_core::Result<()> = unsafe {
                (|| {
                    let controller = webview.controller();
                    let core = controller.CoreWebView2()?;
                    if let Ok(c3) = core.cast::<ICoreWebView2_3>() {
                        let _ = c3.Resume();
                    }
                    if let Ok(c19) = core.cast::<ICoreWebView2_19>() {
                        let _ = c19.SetMemoryUsageTargetLevel(
                            COREWEBVIEW2_MEMORY_USAGE_TARGET_LEVEL_NORMAL,
                        );
                    }
                    controller.SetIsVisible(true)?;
                    Ok(())
                })()
            };
            if let Err(error) = result {
                tracing::warn!(%error, "WebView2 nao retomou como esperado");
            }
        });
        if let Err(error) = queued {
            tracing::warn!(%error, "with_webview indisponivel ao retomar");
        }
        // Mostra sem roubar o foco: quem traz o Sofa para a frente e a escada, nao o `show`.
        if let Err(error) = window.show() {
            tracing::warn!(%error, "nao conseguiu mostrar a janela");
        }
        // Monitor que mudou enquanto o jogo estava aberto (BE-34): cumpre agora.
        if let Some(display) = self.app.state::<AppState>().display.get() {
            display.on_launcher_shown();
        }
    }
}

/// Tela cheia sem borda no monitor dado (RN-20, CA-66). A janela e tirada da tela cheia, levada
/// ao centro do monitor de destino com metade do tamanho (assim o Windows a atribui a ele sem
/// duvida) e posta em tela cheia de novo. Janela oculta (jogo aberto): o `DisplayService` ja nao
/// chama (guarda a visibilidade em vez de perguntar ao Tauri, ARQ-BUG-02).
pub struct TauriLauncherPlacement<R: Runtime> {
    app: AppHandle<R>,
}

impl<R: Runtime> TauriLauncherPlacement<R> {
    pub fn new(app: AppHandle<R>) -> Self {
        Self { app }
    }
}

impl<R: Runtime> LauncherPlacement for TauriLauncherPlacement<R> {
    fn place(&self, monitor: &Monitor) -> bool {
        let Some(window) = self.app.get_webview_window(MAIN_LABEL) else {
            return false;
        };
        let r = monitor.rect;
        let (w, h) = (r.width / 2, r.height / 2);
        let steps = [
            window.set_fullscreen(false),
            window.set_size(PhysicalSize::new(w.max(200), h.max(200))),
            window.set_position(PhysicalPosition::new(
                r.x + i32::try_from(r.width / 4).unwrap_or(0),
                r.y + i32::try_from(r.height / 4).unwrap_or(0),
            )),
            window.set_fullscreen(true),
        ];
        for step in steps {
            if let Err(error) = step {
                tracing::warn!(%error, "nao conseguiu mover o launcher para o monitor");
            }
        }
        true
    }
}
