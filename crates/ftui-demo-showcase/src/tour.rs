#![forbid(unsafe_code)]

//! Guided Tour orchestration for the demo showcase.
//!
//! Provides a deterministic, data-driven storyboard that advances across
//! key screens using the Screen Registry metadata.

use std::time::Duration;

use ftui_core::geometry::Rect;

use crate::app::ScreenId;
use crate::screens::{self, ScreenCategory, ScreenMeta};

const SPEED_MIN: f64 = 0.25;
const SPEED_MAX: f64 = 4.0;
const DEFAULT_STEP_DURATION_MS: u64 = 5200;

const DEFAULT_HIGHLIGHT: TourHighlight = TourHighlight::new_pct(0.12, 0.12, 0.76, 0.72);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TourAdvanceReason {
    Auto,
    ManualNext,
    ManualPrev,
    Jump,
}

#[derive(Debug, Clone)]
pub enum TourEvent {
    StepChanged {
        from: ScreenId,
        to: ScreenId,
        reason: TourAdvanceReason,
    },
    Finished {
        last_screen: ScreenId,
    },
}

#[derive(Debug, Clone)]
pub struct TourStep {
    pub id: String,
    pub screen: ScreenId,
    pub title: &'static str,
    pub blurb: &'static str,
    pub hint: Option<&'static str>,
    pub duration: Duration,
    pub highlight: Option<TourHighlight>,
}

#[derive(Debug, Clone, Copy)]
pub struct TourHighlight {
    x_pct: f32,
    y_pct: f32,
    w_pct: f32,
    h_pct: f32,
}

impl TourHighlight {
    pub const fn new_pct(x_pct: f32, y_pct: f32, w_pct: f32, h_pct: f32) -> Self {
        Self {
            x_pct,
            y_pct,
            w_pct,
            h_pct,
        }
    }

    pub fn resolve(self, area: Rect) -> Rect {
        let width = ((area.width as f32) * self.w_pct).round() as u16;
        let height = ((area.height as f32) * self.h_pct).round() as u16;
        let width = width.clamp(1, area.width);
        let height = height.clamp(1, area.height);
        let x = area.x + ((area.width as f32) * self.x_pct).round() as u16;
        let y = area.y + ((area.height as f32) * self.y_pct).round() as u16;
        let x = x.min(area.right().saturating_sub(width));
        let y = y.min(area.bottom().saturating_sub(height));
        Rect::new(x, y, width, height)
    }
}

#[derive(Debug, Clone)]
pub struct TourOverlayStep<'a> {
    pub index: usize,
    pub title: &'a str,
    pub category: ScreenCategory,
    pub hint: Option<&'a str>,
    pub is_current: bool,
}

#[derive(Debug, Clone)]
pub struct TourOverlayState<'a> {
    pub step_index: usize,
    pub step_count: usize,
    pub screen_title: &'a str,
    pub screen_category: ScreenCategory,
    pub callout_title: &'a str,
    pub callout_body: &'a str,
    pub callout_hint: Option<&'a str>,
    pub paused: bool,
    pub speed: f64,
    pub remaining: Duration,
    pub steps: Vec<TourOverlayStep<'a>>,
    pub highlight: Option<Rect>,
}

#[derive(Debug, Clone)]
pub struct GuidedTourState {
    active: bool,
    paused: bool,
    speed: f64,
    step_index: usize,
    step_elapsed: Duration,
    steps: Vec<TourStep>,
    resume_screen: ScreenId,
}

impl Default for GuidedTourState {
    fn default() -> Self {
        Self::new()
    }
}

impl GuidedTourState {
    pub fn new() -> Self {
        Self {
            active: false,
            paused: false,
            speed: 1.0,
            step_index: 0,
            step_elapsed: Duration::ZERO,
            steps: build_steps(),
            resume_screen: ScreenId::Dashboard,
        }
    }

    pub fn is_active(&self) -> bool {
        self.active
    }

    pub fn is_paused(&self) -> bool {
        self.paused
    }

    pub fn speed(&self) -> f64 {
        self.speed
    }

    pub fn set_speed(&mut self, speed: f64) {
        self.speed = normalize_speed(speed);
    }

    pub fn toggle_pause(&mut self) {
        self.paused = !self.paused;
    }

    pub fn pause(&mut self) {
        self.paused = true;
    }

    pub fn resume(&mut self) {
        self.paused = false;
    }

    pub fn start(&mut self, resume_screen: ScreenId, start_step: usize, speed: f64) {
        self.steps = build_steps();
        self.active = true;
        self.paused = false;
        self.speed = normalize_speed(speed);
        self.step_index = start_step.min(self.steps.len().saturating_sub(1));
        self.step_elapsed = Duration::ZERO;
        self.resume_screen = resume_screen;
    }

    pub fn stop(&mut self, keep_last: bool) -> ScreenId {
        let screen = if keep_last {
            self.active_screen()
        } else {
            self.resume_screen
        };
        self.active = false;
        self.paused = false;
        self.step_elapsed = Duration::ZERO;
        screen
    }

    pub fn step_index(&self) -> usize {
        self.step_index
    }

    pub fn step_count(&self) -> usize {
        self.steps.len()
    }

    pub fn active_screen(&self) -> ScreenId {
        self.steps
            .get(self.step_index)
            .map(|step| step.screen)
            .unwrap_or(self.resume_screen)
    }

    pub fn advance(&mut self, delta: Duration) -> Option<TourEvent> {
        if !self.active || self.paused || self.steps.is_empty() {
            return None;
        }

        self.step_elapsed = self
            .step_elapsed
            .checked_add(scale_duration(delta, self.speed))
            .unwrap_or(Duration::MAX);

        let step = self.steps.get(self.step_index)?;
        if self.step_elapsed < step.duration {
            return None;
        }
        self.next_step(TourAdvanceReason::Auto)
    }

    pub fn next_step(&mut self, reason: TourAdvanceReason) -> Option<TourEvent> {
        if !self.active || self.steps.is_empty() {
            return None;
        }
        let from = self.active_screen();
        if self.step_index + 1 >= self.steps.len() {
            self.active = false;
            self.paused = false;
            self.step_elapsed = Duration::ZERO;
            return Some(TourEvent::Finished { last_screen: from });
        }
        self.step_index += 1;
        self.step_elapsed = Duration::ZERO;
        let to = self.active_screen();
        Some(TourEvent::StepChanged { from, to, reason })
    }

    pub fn prev_step(&mut self) -> Option<TourEvent> {
        if !self.active || self.steps.is_empty() {
            return None;
        }
        if self.step_index == 0 {
            return None;
        }
        let from = self.active_screen();
        self.step_index = self.step_index.saturating_sub(1);
        self.step_elapsed = Duration::ZERO;
        let to = self.active_screen();
        Some(TourEvent::StepChanged {
            from,
            to,
            reason: TourAdvanceReason::ManualPrev,
        })
    }

    pub fn jump_to(&mut self, index: usize) -> Option<TourEvent> {
        if !self.active || self.steps.is_empty() {
            return None;
        }
        let idx = index.min(self.steps.len().saturating_sub(1));
        if idx == self.step_index {
            return None;
        }
        let from = self.active_screen();
        self.step_index = idx;
        self.step_elapsed = Duration::ZERO;
        let to = self.active_screen();
        Some(TourEvent::StepChanged {
            from,
            to,
            reason: TourAdvanceReason::Jump,
        })
    }

    pub fn overlay_state<'a>(
        &'a self,
        content_area: Rect,
        max_steps: usize,
    ) -> Option<TourOverlayState<'a>> {
        if !self.active {
            return None;
        }
        let step = self.steps.get(self.step_index)?;
        let step_count = self.steps.len();
        let highlight = step.highlight.map(|h| h.resolve(content_area));

        let window = max_steps.max(1);
        let start = self.step_index.saturating_sub(1);
        let end = (start + window).min(step_count);
        let steps = self.steps[start..end]
            .iter()
            .enumerate()
            .map(|(offset, step)| {
                let index = start + offset;
                TourOverlayStep {
                    index,
                    title: step.title,
                    category: screens::screen_category(step.screen),
                    hint: step.hint,
                    is_current: index == self.step_index,
                }
            })
            .collect::<Vec<_>>();

        let remaining = step
            .duration
            .saturating_sub(self.step_elapsed.min(step.duration));

        Some(TourOverlayState {
            step_index: self.step_index,
            step_count,
            screen_title: step.title,
            screen_category: screens::screen_category(step.screen),
            callout_title: step.title,
            callout_body: step.blurb,
            callout_hint: step.hint,
            paused: self.paused,
            speed: self.speed,
            remaining,
            steps,
            highlight,
        })
    }
}

fn build_steps() -> Vec<TourStep> {
    screens::screen_registry()
        .iter()
        .filter(|meta| meta.tour_step_hint.is_some() && meta.id != ScreenId::GuidedTour)
        .map(|meta| TourStep {
            id: format!("step:{}", slugify(meta.title)),
            screen: meta.id,
            title: meta.title,
            blurb: meta.blurb,
            hint: meta.tour_step_hint,
            duration: step_duration(meta),
            highlight: Some(DEFAULT_HIGHLIGHT),
        })
        .collect()
}

fn slugify(input: &str) -> String {
    input.to_lowercase().replace(' ', "_")
}

fn step_duration(meta: &ScreenMeta) -> Duration {
    let base = match meta.category {
        ScreenCategory::Visuals => DEFAULT_STEP_DURATION_MS + 1800,
        ScreenCategory::Systems => DEFAULT_STEP_DURATION_MS + 1200,
        ScreenCategory::Tour => DEFAULT_STEP_DURATION_MS,
        ScreenCategory::Core => DEFAULT_STEP_DURATION_MS + 800,
        ScreenCategory::Interaction => DEFAULT_STEP_DURATION_MS + 800,
        ScreenCategory::Text => DEFAULT_STEP_DURATION_MS + 800,
    };
    Duration::from_millis(base)
}

fn normalize_speed(speed: f64) -> f64 {
    if speed.is_finite() && speed > 0.0 {
        speed.clamp(SPEED_MIN, SPEED_MAX)
    } else {
        1.0
    }
}

fn scale_duration(delta: Duration, speed: f64) -> Duration {
    let micros = delta.as_micros() as f64 * speed;
    let micros = micros.round().clamp(0.0, u64::MAX as f64) as u64;
    Duration::from_micros(micros)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tour_advances_and_finishes() {
        let mut tour = GuidedTourState::new();
        tour.start(ScreenId::Dashboard, 0, 1.0);
        assert!(tour.is_active());
        let steps = tour.step_count();
        assert!(steps > 0);

        // Force-advance until completion.
        for _ in 0..steps {
            let _ = tour.advance(Duration::from_secs(10));
        }
        assert!(!tour.is_active());
    }

    #[test]
    fn tour_pause_blocks_advance() {
        let mut tour = GuidedTourState::new();
        tour.start(ScreenId::Dashboard, 0, 1.0);
        tour.pause();
        let before = tour.step_index();
        let _ = tour.advance(Duration::from_secs(10));
        assert_eq!(before, tour.step_index());
    }

    #[test]
    fn tour_next_prev_clamps() {
        let mut tour = GuidedTourState::new();
        tour.start(ScreenId::Dashboard, 0, 1.0);
        let first = tour.active_screen();
        let _ = tour.prev_step();
        assert_eq!(tour.active_screen(), first);
        let _ = tour.next_step(TourAdvanceReason::ManualNext);
        assert_ne!(tour.active_screen(), first);
    }
}
