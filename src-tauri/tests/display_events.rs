//! BE-19 + BE-34: `DisplayChanged` do Windows chega ao `DisplayService` (rajadas viram uma so
//! reavaliacao) e a nova lista sai em `display://changed` (CA-66, CA-67). Topologia falsa: nenhuma
//! janela real se mexe.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use launcher_core::application::ports::{DisplayTopology, LauncherPlacement, SettingsStore};
use launcher_core::application::{AppError, DisplayService};
use launcher_core::domain::display::{Monitor, Rect};
use serde_json::Value;
use sofa_lib::events::{TauriDisplayEvents, TauriSystemEvents};
use sofa_lib::state::AppState;
use tauri::test::{mock_builder, mock_context, noop_assets};
use tauri::{Listener, Manager};

fn mon(id: &str, primary: bool, x: i32) -> Monitor {
    Monitor {
        id: id.into(),
        name: id.into(),
        primary,
        rect: Rect {
            x,
            y: 0,
            width: 1920,
            height: 1080,
        },
    }
}

struct Topology {
    list: Mutex<Vec<Monitor>>,
    enumerations: AtomicUsize,
}
impl DisplayTopology for Topology {
    fn monitors(&self) -> Vec<Monitor> {
        self.enumerations.fetch_add(1, Ordering::SeqCst);
        self.list.lock().unwrap().clone()
    }
}

#[derive(Default)]
struct Placement(Mutex<Vec<String>>);
impl LauncherPlacement for Placement {
    fn place(&self, m: &Monitor) -> bool {
        self.0.lock().unwrap().push(m.id.clone());
        true
    }
}

struct Store(Mutex<Option<String>>);
impl SettingsStore for Store {
    fn monitor_id(&self) -> Option<String> {
        self.0.lock().unwrap().clone()
    }
    fn set_monitor_id(&self, id: Option<String>) -> Result<(), AppError> {
        *self.0.lock().unwrap() = id;
        Ok(())
    }
    fn install_defaults_applied(&self) -> bool {
        false
    }
    fn mark_install_defaults_applied(&self) -> Result<(), AppError> {
        Ok(())
    }
}

#[test]
fn rajada_de_display_changed_vira_uma_reavaliacao_e_um_evento() {
    use launcher_win::system_events::{SystemEvent, SystemEventSink};

    let app = sofa_lib::configure(mock_builder())
        .build(mock_context(noop_assets()))
        .expect("app de teste");
    let handle = app.handle().clone();
    let topology = Arc::new(Topology {
        list: Mutex::new(vec![mon("PC", true, 0), mon("TV", false, 1920)]),
        enumerations: AtomicUsize::new(0),
    });
    let placement = Arc::new(Placement::default());
    let display = DisplayService::new(
        topology.clone(),
        placement.clone(),
        Arc::new(Store(Mutex::new(Some("TV".into())))),
        Arc::new(TauriDisplayEvents::new(handle.clone())),
    );
    display.start(); // abre na TV
    assert_eq!(*placement.0.lock().unwrap(), vec!["TV"]);
    app.state::<AppState>()
        .display
        .set(Arc::new(display))
        .ok()
        .unwrap();

    let seen: Arc<Mutex<Vec<Value>>> = Arc::default();
    let sink = seen.clone();
    app.listen("display://changed", move |e| {
        sink.lock()
            .unwrap()
            .push(serde_json::from_str(e.payload()).unwrap());
    });

    // A TV some (CA-67): tres avisos seguidos do Windows.
    *topology.list.lock().unwrap() = vec![mon("PC", true, 0)];
    let before = topology.enumerations.load(Ordering::SeqCst);
    let events = TauriSystemEvents::new(handle);
    for _ in 0..3 {
        events.event(SystemEvent::DisplayChanged);
    }

    let deadline = Instant::now() + Duration::from_secs(3);
    while seen.lock().unwrap().is_empty() && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(20));
    }
    std::thread::sleep(Duration::from_millis(700)); // tempo de sobra para um evento a mais
    let got = seen.lock().unwrap().clone();
    assert_eq!(got.len(), 1, "{got:?}");
    assert_eq!(got[0].as_array().unwrap().len(), 1);
    assert_eq!(got[0][0]["id"], "PC");
    assert_eq!(got[0][0]["current"], true);
    assert_eq!(
        topology.enumerations.load(Ordering::SeqCst) - before,
        1,
        "uma enumeracao para a rajada inteira"
    );
    // O launcher cai no principal; a preferencia segue sendo a TV.
    assert_eq!(placement.0.lock().unwrap().last().unwrap(), "PC");
}
