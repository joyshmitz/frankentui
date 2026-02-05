#![forbid(unsafe_code)]

//! VOI overlay demo screen (Galaxy-Brain widget).

use std::time::Instant;

use ftui_core::event::{Event, KeyCode, KeyEvent, KeyEventKind};
use ftui_core::geometry::Rect;
use ftui_render::frame::Frame;
use ftui_runtime::{
    Cmd, InlineAutoRemeasureConfig, VoiLogEntry, VoiSampler, VoiSamplerSnapshot,
    inline_auto_voi_snapshot,
};
use ftui_style::Style;
use ftui_widgets::Widget;
use ftui_widgets::borders::BorderType;
use ftui_widgets::paragraph::Paragraph;
use ftui_widgets::voi_debug_overlay::{
    VoiDebugOverlay, VoiDecisionSummary, VoiLedgerEntry, VoiObservationSummary, VoiOverlayData,
    VoiOverlayStyle, VoiPosteriorSummary,
};

use super::{HelpEntry, Screen};
use crate::theme;

/// Tiny screen showcasing the VOI overlay widget.
pub struct VoiOverlayScreen {
    sampler: VoiSampler,
    tick: u64,
    start: Instant,
}

impl Default for VoiOverlayScreen {
    fn default() -> Self {
        Self::new()
    }
}

impl VoiOverlayScreen {
    pub fn new() -> Self {
        let mut config = InlineAutoRemeasureConfig::default().voi;
        config.enable_logging = true;
        config.max_log_entries = 96;
        let sampler = VoiSampler::new(config);
        Self {
            sampler,
            tick: 0,
            start: Instant::now(),
        }
    }

    fn reset(&mut self) {
        *self = Self::new();
    }

    fn overlay_area(area: Rect, width: u16, height: u16) -> Rect {
        let w = width.min(area.width).max(1);
        let h = height.min(area.height).max(1);
        let x = area.x + area.width.saturating_sub(w) / 2;
        let y = area.y + area.height.saturating_sub(h) / 2;
        Rect::new(x, y, w, h)
    }

    fn data_from_snapshot(&self, snapshot: &VoiSamplerSnapshot, source: &str) -> VoiOverlayData {
        VoiOverlayData {
            title: "VOI Overlay".to_string(),
            tick: Some(self.tick),
            source: Some(source.to_string()),
            posterior: VoiPosteriorSummary {
                alpha: snapshot.alpha,
                beta: snapshot.beta,
                mean: snapshot.posterior_mean,
                variance: snapshot.posterior_variance,
                expected_variance_after: snapshot.expected_variance_after,
                voi_gain: snapshot.voi_gain,
            },
            decision: snapshot
                .last_decision
                .as_ref()
                .map(|decision| VoiDecisionSummary {
                    event_idx: decision.event_idx,
                    should_sample: decision.should_sample,
                    reason: decision.reason.to_string(),
                    score: decision.score,
                    cost: decision.cost,
                    log_bayes_factor: decision.log_bayes_factor,
                    e_value: decision.e_value,
                    e_threshold: decision.e_threshold,
                    boundary_score: decision.boundary_score,
                }),
            observation: snapshot
                .last_observation
                .as_ref()
                .map(|obs| VoiObservationSummary {
                    sample_idx: obs.sample_idx,
                    violated: obs.violated,
                    posterior_mean: obs.posterior_mean,
                    alpha: obs.alpha,
                    beta: obs.beta,
                }),
            ledger: Self::ledger_entries_from_logs(snapshot.recent_logs.iter().rev().take(6).rev()),
        }
    }

    fn data_from_sampler(&self, source: &str) -> VoiOverlayData {
        let (alpha, beta) = self.sampler.posterior_params();
        let variance = self.sampler.posterior_variance();
        let expected_after = self.sampler.expected_variance_after();
        VoiOverlayData {
            title: "VOI Overlay".to_string(),
            tick: Some(self.tick),
            source: Some(source.to_string()),
            posterior: VoiPosteriorSummary {
                alpha,
                beta,
                mean: self.sampler.posterior_mean(),
                variance,
                expected_variance_after: expected_after,
                voi_gain: (variance - expected_after).max(0.0),
            },
            decision: self
                .sampler
                .last_decision()
                .map(|decision| VoiDecisionSummary {
                    event_idx: decision.event_idx,
                    should_sample: decision.should_sample,
                    reason: decision.reason.to_string(),
                    score: decision.score,
                    cost: decision.cost,
                    log_bayes_factor: decision.log_bayes_factor,
                    e_value: decision.e_value,
                    e_threshold: decision.e_threshold,
                    boundary_score: decision.boundary_score,
                }),
            observation: self
                .sampler
                .last_observation()
                .map(|obs| VoiObservationSummary {
                    sample_idx: obs.sample_idx,
                    violated: obs.violated,
                    posterior_mean: obs.posterior_mean,
                    alpha: obs.alpha,
                    beta: obs.beta,
                }),
            ledger: Self::ledger_entries_from_logs(self.sampler.logs().iter().rev().take(6).rev()),
        }
    }

    fn ledger_entries_from_logs<'a, I>(logs: I) -> Vec<VoiLedgerEntry>
    where
        I: IntoIterator<Item = &'a VoiLogEntry>,
    {
        logs.into_iter()
            .map(|entry| match entry {
                VoiLogEntry::Decision(decision) => VoiLedgerEntry::Decision {
                    event_idx: decision.event_idx,
                    should_sample: decision.should_sample,
                    voi_gain: decision.voi_gain,
                    log_bayes_factor: decision.log_bayes_factor,
                },
                VoiLogEntry::Observation(obs) => VoiLedgerEntry::Observation {
                    sample_idx: obs.sample_idx,
                    violated: obs.violated,
                    posterior_mean: obs.posterior_mean,
                },
            })
            .collect()
    }
}

impl Screen for VoiOverlayScreen {
    type Message = ();

    fn update(&mut self, event: &Event) -> Cmd<Self::Message> {
        if let Event::Key(KeyEvent {
            code: KeyCode::Char('r'),
            kind: KeyEventKind::Press,
            ..
        }) = event
        {
            self.reset();
        }
        Cmd::None
    }

    fn view(&self, frame: &mut Frame, area: Rect) {
        if area.is_empty() {
            return;
        }

        let hint = "runtime snapshot: inline-auto  |  fallback: local sampler  |  r:reset";
        Paragraph::new(hint)
            .style(Style::new().fg(theme::fg::MUTED))
            .render(
                Rect::new(area.x + 1, area.y, area.width.saturating_sub(2), 1),
                frame,
            );

        let overlay_area = Self::overlay_area(area, 68, 22);
        if overlay_area.width < 28 || overlay_area.height < 8 {
            return;
        }

        let data = if let Some(snapshot) = inline_auto_voi_snapshot() {
            self.data_from_snapshot(&snapshot, "runtime:inline-auto")
        } else {
            self.data_from_sampler("demo:fallback")
        };

        let style = VoiOverlayStyle {
            border: Style::new().fg(theme::accent::PRIMARY).bg(theme::bg::DEEP),
            text: Style::new().fg(theme::fg::PRIMARY),
            background: Some(theme::bg::DEEP.into()),
            border_type: BorderType::Rounded,
        };

        VoiDebugOverlay::new(data)
            .with_style(style)
            .render(overlay_area, frame);
    }

    fn keybindings(&self) -> Vec<HelpEntry> {
        vec![HelpEntry {
            key: "r",
            action: "Reset VOI sampler",
        }]
    }

    fn tick(&mut self, tick_count: u64) {
        self.tick = tick_count;
        let now = Instant::now();
        let decision = self.sampler.decide(now);
        if decision.should_sample {
            let violated = (tick_count % 17) < 3;
            self.sampler.observe_at(violated, now);
        }
    }

    fn title(&self) -> &'static str {
        "VOI Overlay"
    }

    fn tab_label(&self) -> &'static str {
        "VOI"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ftui_core::event::Modifiers;
    use ftui_render::grapheme_pool::GraphemePool;
    use ftui_runtime::{VoiDecision, VoiObservation};

    fn sample_decision() -> VoiDecision {
        VoiDecision {
            event_idx: 7,
            should_sample: true,
            forced_by_interval: false,
            blocked_by_min_interval: false,
            voi_gain: 0.22,
            score: 0.3,
            cost: 0.1,
            log_bayes_factor: 1.2,
            posterior_mean: 0.4,
            posterior_variance: 0.5,
            e_value: 1.1,
            e_threshold: 2.0,
            boundary_score: 0.7,
            events_since_sample: 3,
            time_since_sample_ms: 12.0,
            reason: "test",
        }
    }

    fn sample_observation() -> VoiObservation {
        VoiObservation {
            event_idx: 7,
            sample_idx: 2,
            violated: true,
            posterior_mean: 0.55,
            posterior_variance: 0.1,
            alpha: 2.0,
            beta: 3.0,
            e_value: 1.4,
            e_threshold: 2.0,
        }
    }

    #[test]
    fn overlay_area_clamps_and_centers() {
        let area = Rect::new(10, 5, 20, 10);
        let overlay = VoiOverlayScreen::overlay_area(area, 60, 60);
        assert_eq!(overlay.width, 20);
        assert_eq!(overlay.height, 10);
        assert_eq!(overlay.x, 10);
        assert_eq!(overlay.y, 5);

        let overlay = VoiOverlayScreen::overlay_area(area, 10, 4);
        assert_eq!(overlay.width, 10);
        assert_eq!(overlay.height, 4);
        assert_eq!(overlay.x, 10 + (20 - 10) / 2);
        assert_eq!(overlay.y, 5 + (10 - 4) / 2);
    }

    #[test]
    fn ledger_entries_from_logs_maps_entries() {
        let decision = sample_decision();
        let observation = sample_observation();
        let logs = [
            VoiLogEntry::Decision(decision.clone()),
            VoiLogEntry::Observation(observation.clone()),
        ];
        let entries = VoiOverlayScreen::ledger_entries_from_logs(logs.iter());
        assert_eq!(entries.len(), 2);

        match &entries[0] {
            VoiLedgerEntry::Decision {
                event_idx,
                should_sample,
                ..
            } => {
                assert_eq!(*event_idx, decision.event_idx);
                assert_eq!(*should_sample, decision.should_sample);
            }
            _ => panic!("expected decision entry"),
        }

        match &entries[1] {
            VoiLedgerEntry::Observation {
                sample_idx,
                violated,
                ..
            } => {
                assert_eq!(*sample_idx, observation.sample_idx);
                assert_eq!(*violated, observation.violated);
            }
            _ => panic!("expected observation entry"),
        }
    }

    #[test]
    fn data_from_snapshot_populates_fields() {
        let decision = sample_decision();
        let observation = sample_observation();
        let logs = vec![
            VoiLogEntry::Decision(decision.clone()),
            VoiLogEntry::Observation(observation.clone()),
        ];
        let snapshot = VoiSamplerSnapshot {
            captured_ms: 123,
            alpha: 2.0,
            beta: 3.0,
            posterior_mean: 0.4,
            posterior_variance: 0.05,
            expected_variance_after: 0.02,
            voi_gain: 0.03,
            last_decision: Some(decision.clone()),
            last_observation: Some(observation.clone()),
            recent_logs: logs,
        };

        let mut screen = VoiOverlayScreen::new();
        screen.tick = 42;
        let data = screen.data_from_snapshot(&snapshot, "snapshot");
        assert_eq!(data.tick, Some(42));
        assert_eq!(data.source.as_deref(), Some("snapshot"));
        assert_eq!(data.posterior.alpha, snapshot.alpha);
        assert_eq!(data.posterior.beta, snapshot.beta);
        assert!(data.decision.is_some());
        assert!(data.observation.is_some());
        assert_eq!(data.ledger.len(), 2);
    }

    #[test]
    fn data_from_sampler_populates_fields() {
        let mut screen = VoiOverlayScreen::new();
        screen.tick = 7;
        let now = Instant::now();
        let _ = screen.sampler.decide(now);
        let _ = screen.sampler.observe_at(true, now);

        let data = screen.data_from_sampler("fallback");
        assert_eq!(data.tick, Some(7));
        assert_eq!(data.source.as_deref(), Some("fallback"));
        assert!(data.decision.is_some());
        assert!(data.observation.is_some());
        assert!(!data.ledger.is_empty());
    }

    #[test]
    fn update_resets_on_r() {
        let mut screen = VoiOverlayScreen::new();
        screen.tick(5);
        assert!(!screen.sampler.logs().is_empty());
        let event = Event::Key(KeyEvent {
            code: KeyCode::Char('r'),
            modifiers: Modifiers::NONE,
            kind: KeyEventKind::Press,
        });
        screen.update(&event);
        assert_eq!(screen.tick, 0);
        assert!(screen.sampler.logs().is_empty());
    }

    #[test]
    fn tick_updates_counter_and_sampler() {
        let mut screen = VoiOverlayScreen::new();
        screen.tick(1);
        assert_eq!(screen.tick, 1);
        assert_eq!(screen.sampler.summary().total_events, 1);
    }

    #[test]
    fn render_no_panic_empty_area() {
        let screen = VoiOverlayScreen::new();
        let mut pool = GraphemePool::new();
        let mut frame = Frame::new(1, 1, &mut pool);
        screen.view(&mut frame, Rect::new(0, 0, 0, 0));
    }

    #[test]
    fn render_no_panic_small_area() {
        let screen = VoiOverlayScreen::new();
        let mut pool = GraphemePool::new();
        let mut frame = Frame::new(20, 6, &mut pool);
        screen.view(&mut frame, Rect::new(0, 0, 20, 6));
    }

    #[test]
    fn render_no_panic_standard_area() {
        let screen = VoiOverlayScreen::new();
        let mut pool = GraphemePool::new();
        let mut frame = Frame::new(80, 24, &mut pool);
        screen.view(&mut frame, Rect::new(0, 0, 80, 24));
    }
}
