//! Conversoes do dominio para os DTOs da IPC (contrato secoes 1 e 3). Sem regra de negocio.

use launcher_core::application::{SessionView, VolumeView};
use launcher_core::domain::input::InputIntent as CoreIntent;
use launcher_core::domain::session::{
    FrontOutcome as CoreOutcome, LaunchFailure as CoreFailure, SessionState as CoreState,
};

use crate::dto::{
    FrontOutcome, InputIntent, LaunchFailure, SessionDto, SessionPanelEvent, SessionState,
    VolumeState,
};

impl From<CoreState> for SessionState {
    fn from(s: CoreState) -> Self {
        match s {
            CoreState::Idle => Self::Idle,
            CoreState::Launching => Self::Launching,
            CoreState::NeedsElevation => Self::NeedsElevation,
            CoreState::NotConfirmed => Self::NotConfirmed,
            CoreState::Running => Self::Running,
            CoreState::Ending => Self::Ending,
            CoreState::Closing => Self::Closing,
            CoreState::CloseOverdue => Self::CloseOverdue,
        }
    }
}

impl From<CoreOutcome> for FrontOutcome {
    fn from(o: CoreOutcome) -> Self {
        match o {
            CoreOutcome::Front => Self::Front,
            CoreOutcome::FrontAfterMinimize => Self::FrontAfterMinimize,
            CoreOutcome::Failed => Self::Failed,
        }
    }
}

impl From<CoreIntent> for InputIntent {
    fn from(i: CoreIntent) -> Self {
        match i {
            CoreIntent::Up => Self::Up,
            CoreIntent::Down => Self::Down,
            CoreIntent::Left => Self::Left,
            CoreIntent::Right => Self::Right,
            CoreIntent::Confirm => Self::Confirm,
            CoreIntent::Back => Self::Back,
            CoreIntent::Menu => Self::Menu,
            CoreIntent::View => Self::View,
            CoreIntent::PagePrev => Self::PagePrev,
            CoreIntent::PageNext => Self::PageNext,
            CoreIntent::VolumeDown => Self::VolumeDown,
            CoreIntent::VolumeUp => Self::VolumeUp,
        }
    }
}

impl From<&SessionView> for SessionDto {
    fn from(v: &SessionView) -> Self {
        let s = &v.snapshot;
        Self {
            state: s.state.into(),
            game_id: s.game_id.as_ref().map(ToString::to_string),
            since: v.since_unix_ms,
            dismissed: s.dismissed,
            elevated: s.elevated,
            minimized: s.minimized,
            launch_failure: s.launch_failure.map(|f| match f {
                CoreFailure::ElevationDenied => LaunchFailure::ElevationDenied,
            }),
        }
    }
}

impl From<VolumeView> for VolumeState {
    fn from(v: VolumeView) -> Self {
        Self {
            level: v.level,
            muted: v.muted,
            available: v.available,
        }
    }
}

impl SessionPanelEvent {
    pub fn opened(outcome: CoreOutcome) -> Self {
        Self {
            open: true,
            outcome: outcome.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use launcher_core::domain::library::GameId;
    use launcher_core::domain::session::SessionSnapshot;
    use launcher_core::domain::time::Millis;

    #[test]
    fn view_vira_dto_com_since_em_ms_unix() {
        let v = SessionView {
            snapshot: SessionSnapshot {
                state: CoreState::Launching,
                game_id: Some(GameId::parse("steam:620").unwrap()),
                since: Some(Millis(5)),
                dismissed: true,
                elevated: false,
                minimized: false,
                launch_failure: Some(CoreFailure::ElevationDenied),
            },
            since_unix_ms: Some(1_794_610_123_456),
        };
        let dto = SessionDto::from(&v);
        let json = serde_json::to_value(&dto).unwrap();
        assert_eq!(
            json,
            serde_json::json!({
                "state": "Launching", "gameId": "steam:620", "since": 1_794_610_123_456_i64,
                "dismissed": true, "elevated": false, "minimized": false,
                "launchFailure": "ElevationDenied"
            })
        );
    }
}
