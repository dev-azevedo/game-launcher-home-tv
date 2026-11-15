//! Casos de uso de monitor (BE-34, RN-20, CA-66, CA-67, E-08, E-09): onde o launcher fica em tela
//! cheia, a lista para a UI, a escolha do usuario e a reacao a monitor ligado ou desligado.
//! A regra de escolha e de `domain::display`; aqui ficam o estado ("onde estou agora"), a
//! persistencia da preferencia e a publicacao de `display://changed`.

use std::sync::{Arc, Mutex, PoisonError};

use super::error::AppError;
use super::ports::{DisplayEvents, DisplayTopology, LauncherPlacement, SettingsStore};
use crate::domain::display::{choose_monitor, reconcile, Monitor, Rect};

/// Item de `display_monitors` e de `display://changed`. `current` = onde o launcher esta agora.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MonitorView {
    pub monitor: Monitor,
    pub current: bool,
}

struct State {
    /// Monitor onde o launcher esta (nao e a preferencia: pode ser o principal por fallback).
    current: Option<String>,
    /// Retangulos conhecidos, para perceber mudanca de resolucao no monitor atual.
    rects: Vec<(String, Rect)>,
    /// A janela esta visivel? Guardado aqui (recebe `hide`/`show` do `LauncherWindow`) para nao
    /// perguntar ao Tauri com getter que bloqueia a thread da UI (ARQ-BUG-02).
    visible: bool,
    /// A janela estava oculta (jogo aberto) e o movimento ficou para quando ela voltar.
    pending: bool,
    last_emitted: Option<Vec<MonitorView>>,
}

impl Default for State {
    fn default() -> Self {
        Self {
            current: None,
            rects: Vec::new(),
            visible: true, // a janela nasce visivel
            pending: false,
            last_emitted: None,
        }
    }
}

pub struct DisplayService {
    topology: Arc<dyn DisplayTopology>,
    placement: Arc<dyn LauncherPlacement>,
    store: Arc<dyn SettingsStore>,
    events: Arc<dyn DisplayEvents>,
    state: Mutex<State>,
}

impl DisplayService {
    pub fn new(
        topology: Arc<dyn DisplayTopology>,
        placement: Arc<dyn LauncherPlacement>,
        store: Arc<dyn SettingsStore>,
        events: Arc<dyn DisplayEvents>,
    ) -> Self {
        Self {
            topology,
            placement,
            store,
            events,
            state: Mutex::new(State::default()),
        }
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }

    fn views(list: &[Monitor], current: Option<&str>) -> Vec<MonitorView> {
        list.iter()
            .map(|m| MonitorView {
                monitor: m.clone(),
                current: current == Some(m.id.as_str()),
            })
            .collect()
    }

    fn remember(state: &mut State, list: &[Monitor]) {
        state.rects = list.iter().map(|m| (m.id.clone(), m.rect)).collect();
    }

    /// Move o launcher; se a janela esta oculta, deixa pendente para `on_launcher_shown`.
    fn place(&self, state: &mut State, monitor: &Monitor) {
        state.pending = !state.visible || !self.placement.place(monitor);
    }

    /// Publica a lista so quando ela mudou (nova ligacao, novo monitor atual).
    fn emit_if_changed(&self, state: &mut State, list: &[Monitor]) {
        let views = Self::views(list, state.current.as_deref());
        if state.last_emitted.as_ref() == Some(&views) {
            return;
        }
        state.last_emitted = Some(views.clone());
        self.events.changed(&views);
    }

    /// Na abertura: vai para o monitor da RN-20. A janela ja nasce em tela cheia no principal;
    /// so move quando o escolhido e outro.
    pub fn start(&self) {
        let list = self.topology.monitors();
        let preferred = self.store.monitor_id();
        let mut state = self.lock();
        Self::remember(&mut state, &list);
        let Some(target) = choose_monitor(preferred.as_deref(), &list) else {
            tracing::warn!("monitores: nenhum monitor enumerado; a janela fica onde nasceu");
            return;
        };
        state.current = Some(target.id.clone());
        if !target.primary {
            tracing::info!(monitor = %target.id, "launcher abre no monitor escolhido");
            self.place(&mut state, target);
        }
        state.last_emitted = Some(Self::views(&list, state.current.as_deref()));
    }

    /// `display_monitors` (#28).
    pub fn monitors(&self) -> Vec<MonitorView> {
        let list = self.topology.monitors();
        let state = self.lock();
        Self::views(&list, state.current.as_deref())
    }

    /// `settings_set_monitor` (#27): o id deve estar na lista de agora (`UnknownMonitor`); `None`
    /// volta ao principal. Grava a preferencia e move na hora (CA-66).
    pub fn set_monitor(&self, requested: Option<String>) -> Result<(), AppError> {
        let list = self.topology.monitors();
        if let Some(id) = &requested {
            if !list.iter().any(|m| &m.id == id) {
                return Err(AppError::UnknownMonitor);
            }
        }
        self.store.set_monitor_id(requested.clone())?;
        let mut state = self.lock();
        Self::remember(&mut state, &list);
        if let Some(target) = choose_monitor(requested.as_deref(), &list) {
            if state.current.as_deref() != Some(target.id.as_str()) {
                tracing::info!(monitor = %target.id, "launcher passa para o monitor escolhido");
                self.place(&mut state, target);
                state.current = Some(target.id.clone());
            }
        }
        self.emit_if_changed(&mut state, &list);
        Ok(())
    }

    /// `DisplayChanged` do Windows (CA-66, CA-67, E-09): reavalia onde o launcher deve estar.
    pub fn on_topology_changed(&self) {
        let list = self.topology.monitors();
        if list.is_empty() {
            return; // transitorio (todos desligados): nao mexe em nada
        }
        let preferred = self.store.monitor_id();
        let mut state = self.lock();
        let Some(target) = reconcile(state.current.as_deref(), preferred.as_deref(), &list) else {
            return;
        };
        let moved = state.current.as_deref() != Some(target.id.as_str());
        let resized = state
            .rects
            .iter()
            .find(|(id, _)| id == &target.id)
            .is_some_and(|(_, r)| *r != target.rect);
        if moved || resized {
            tracing::info!(monitor = %target.id, moved, resized, "launcher reacomodado");
            self.place(&mut state, target);
            state.current = Some(target.id.clone());
        }
        Self::remember(&mut state, &list);
        self.emit_if_changed(&mut state, &list);
    }

    /// A janela foi escondida (jogo aberto): movimentos passam a ficar pendentes.
    pub fn on_launcher_hidden(&self) {
        self.lock().visible = false;
    }

    /// A janela voltou a aparecer (fim do jogo ou retorno): cumpre o movimento que ficou pendente.
    pub fn on_launcher_shown(&self) {
        let mut state = self.lock();
        state.visible = true;
        if !state.pending {
            return;
        }
        let Some(current) = state.current.clone() else {
            state.pending = false;
            return;
        };
        let list = self.topology.monitors();
        let Some(target) = list.iter().find(|m| m.id == current) else {
            state.pending = false;
            return;
        };
        self.place(&mut state, target);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::display::Rect;

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

    struct Topology(Mutex<Vec<Monitor>>);
    impl DisplayTopology for Topology {
        fn monitors(&self) -> Vec<Monitor> {
            self.0.lock().unwrap().clone()
        }
    }

    #[derive(Default)]
    struct Placement {
        placed: Mutex<Vec<String>>,
    }
    impl LauncherPlacement for Placement {
        fn place(&self, monitor: &Monitor) -> bool {
            self.placed.lock().unwrap().push(monitor.id.clone());
            true
        }
    }

    #[derive(Default)]
    struct Store {
        monitor: Mutex<Option<String>>,
        fail: Mutex<bool>,
    }
    impl SettingsStore for Store {
        fn monitor_id(&self) -> Option<String> {
            self.monitor.lock().unwrap().clone()
        }
        fn set_monitor_id(&self, id: Option<String>) -> Result<(), AppError> {
            if *self.fail.lock().unwrap() {
                return Err(AppError::Internal);
            }
            *self.monitor.lock().unwrap() = id;
            Ok(())
        }
        fn install_defaults_applied(&self) -> bool {
            false
        }
        fn mark_install_defaults_applied(&self) -> Result<(), AppError> {
            Ok(())
        }
    }

    #[derive(Default)]
    struct Events(Mutex<Vec<Vec<MonitorView>>>);
    impl DisplayEvents for Events {
        fn changed(&self, monitors: &[MonitorView]) {
            self.0.lock().unwrap().push(monitors.to_vec());
        }
    }

    struct Rig {
        topology: Arc<Topology>,
        placement: Arc<Placement>,
        store: Arc<Store>,
        events: Arc<Events>,
        svc: DisplayService,
    }

    fn rig(list: Vec<Monitor>, preferred: Option<&str>) -> Rig {
        let topology = Arc::new(Topology(Mutex::new(list)));
        let placement = Arc::new(Placement::default());
        let store = Arc::new(Store::default());
        *store.monitor.lock().unwrap() = preferred.map(str::to_owned);
        let events = Arc::new(Events::default());
        let svc = DisplayService::new(
            topology.clone(),
            placement.clone(),
            store.clone(),
            events.clone(),
        );
        Rig {
            topology,
            placement,
            store,
            events,
            svc,
        }
    }

    fn two() -> Vec<Monitor> {
        vec![mon("A", true, 0), mon("TV", false, 1920)]
    }

    fn current_id(r: &Rig) -> Option<String> {
        r.svc
            .monitors()
            .into_iter()
            .find(|v| v.current)
            .map(|v| v.monitor.id)
    }

    #[test]
    fn abre_no_principal_sem_mover_e_no_escolhido_movendo() {
        let r = rig(two(), None);
        r.svc.start();
        assert!(r.placement.placed.lock().unwrap().is_empty());
        assert_eq!(current_id(&r).as_deref(), Some("A"));

        let r = rig(two(), Some("TV"));
        r.svc.start();
        assert_eq!(*r.placement.placed.lock().unwrap(), vec!["TV"]);
        assert_eq!(current_id(&r).as_deref(), Some("TV"));
    }

    #[test]
    fn escolhido_ausente_abre_no_principal_e_a_preferencia_fica() {
        let r = rig(vec![mon("A", true, 0)], Some("TV"));
        r.svc.start();
        assert_eq!(current_id(&r).as_deref(), Some("A"));
        assert_eq!(r.store.monitor_id().as_deref(), Some("TV")); // CA-67
    }

    /// Roteiro do teste manual com duas telas, com fakes (CA-66, CA-67, E-09).
    #[test]
    fn trocar_move_na_hora_desplugar_cai_no_principal_religar_nao_pula() {
        let r = rig(two(), None);
        r.svc.start();
        // Trocar para a TV: grava, move, avisa.
        r.svc.set_monitor(Some("TV".into())).unwrap();
        assert_eq!(r.store.monitor_id().as_deref(), Some("TV"));
        assert_eq!(current_id(&r).as_deref(), Some("TV"));
        assert_eq!(r.placement.placed.lock().unwrap().last().unwrap(), "TV");
        assert_eq!(r.events.0.lock().unwrap().len(), 1);

        // Desplugar a TV: cai no principal; a preferencia continua TV.
        *r.topology.0.lock().unwrap() = vec![mon("A", true, 0)];
        r.svc.on_topology_changed();
        assert_eq!(current_id(&r).as_deref(), Some("A"));
        assert_eq!(r.placement.placed.lock().unwrap().last().unwrap(), "A");
        assert_eq!(r.store.monitor_id().as_deref(), Some("TV"));

        // Religar com o Sofa aberto: continua no principal e nao ha movimento novo.
        let placed_before = r.placement.placed.lock().unwrap().len();
        *r.topology.0.lock().unwrap() = two();
        r.svc.on_topology_changed();
        assert_eq!(current_id(&r).as_deref(), Some("A"));
        assert_eq!(r.placement.placed.lock().unwrap().len(), placed_before);
        // ...mas a lista publicada ja mostra a TV de volta.
        let last = r.events.0.lock().unwrap().last().cloned().unwrap();
        assert_eq!(last.len(), 2);

        // Reabrir: volta ao escolhido.
        let reaberto = DisplayService::new(
            r.topology.clone(),
            r.placement.clone(),
            r.store.clone(),
            r.events.clone(),
        );
        reaberto.start();
        assert_eq!(r.placement.placed.lock().unwrap().last().unwrap(), "TV");
    }

    #[test]
    fn monitor_fora_da_lista_e_unknown_monitor_e_nada_e_gravado() {
        let r = rig(two(), None);
        r.svc.start();
        assert_eq!(
            r.svc.set_monitor(Some("FANTASMA".into())),
            Err(AppError::UnknownMonitor)
        );
        assert_eq!(r.store.monitor_id(), None);
        assert!(r.placement.placed.lock().unwrap().is_empty());
    }

    #[test]
    fn nulo_volta_ao_principal() {
        let r = rig(two(), Some("TV"));
        r.svc.start();
        r.svc.set_monitor(None).unwrap();
        assert_eq!(r.store.monitor_id(), None);
        assert_eq!(current_id(&r).as_deref(), Some("A"));
    }

    #[test]
    fn falha_ao_gravar_nao_move() {
        let r = rig(two(), None);
        r.svc.start();
        *r.store.fail.lock().unwrap() = true;
        assert_eq!(
            r.svc.set_monitor(Some("TV".into())),
            Err(AppError::Internal)
        );
        assert_eq!(current_id(&r).as_deref(), Some("A"));
        assert!(r.placement.placed.lock().unwrap().is_empty());
    }

    #[test]
    fn mudanca_de_resolucao_no_monitor_atual_reacomoda() {
        let r = rig(two(), None);
        r.svc.start();
        let mut list = two();
        list[0].rect.width = 1280;
        *r.topology.0.lock().unwrap() = list;
        r.svc.on_topology_changed();
        assert_eq!(*r.placement.placed.lock().unwrap(), vec!["A"]);
    }

    #[test]
    fn evento_so_quando_a_lista_muda_e_lista_vazia_e_ignorada() {
        let r = rig(two(), None);
        r.svc.start();
        r.svc.on_topology_changed();
        r.svc.on_topology_changed();
        assert!(r.events.0.lock().unwrap().is_empty());

        *r.topology.0.lock().unwrap() = Vec::new();
        r.svc.on_topology_changed();
        assert!(r.events.0.lock().unwrap().is_empty());
        assert_eq!(current_id(&r), None); // lista vazia agora, mas o estado interno foi mantido

        *r.topology.0.lock().unwrap() = vec![mon("A", true, 0)];
        r.svc.on_topology_changed();
        assert_eq!(r.events.0.lock().unwrap().len(), 1);
    }

    #[test]
    fn janela_oculta_deixa_o_movimento_pendente_ate_voltar() {
        let r = rig(two(), None);
        r.svc.start();
        r.svc.on_launcher_hidden(); // jogo aberto
        r.svc.set_monitor(Some("TV".into())).unwrap();
        assert!(r.placement.placed.lock().unwrap().is_empty());
        r.svc.on_launcher_shown(); // sessao acabou
        assert_eq!(*r.placement.placed.lock().unwrap(), vec!["TV"]);
        // Nao repete.
        r.svc.on_launcher_shown();
        assert_eq!(r.placement.placed.lock().unwrap().len(), 1);
    }
}
