#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CarouselState {
    selected: usize,
    len: usize,
}

impl CarouselState {
    pub fn new(len: usize) -> Self {
        Self {
            selected: 0,
            len: len.max(1),
        }
    }

    pub fn current(self) -> usize {
        self.selected
    }

    pub fn next(&mut self) {
        self.selected = (self.selected + 1) % self.len;
    }

    pub fn previous(&mut self) {
        self.selected = if self.selected == 0 {
            self.len - 1
        } else {
            self.selected - 1
        };
    }

    pub fn select(&mut self, index: usize) {
        self.selected = index.min(self.len - 1);
    }
}

pub fn normalized_phase(time_seconds: f64, speed: f32) -> f32 {
    (((time_seconds as f32 * speed).sin() + 1.0) * 0.5).clamp(0.0, 1.0)
}

pub fn ease_out_cubic(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    1.0 - (1.0 - t).powi(3)
}

pub fn motion_amount(reduced_motion: bool, time_seconds: f64, speed: f32) -> f32 {
    if reduced_motion {
        0.0
    } else {
        ease_out_cubic(normalized_phase(time_seconds, speed))
    }
}

pub fn queue_progress(done: bool, failed: bool, animated_phase: f32) -> f32 {
    if done || failed {
        1.0
    } else {
        (0.18 + animated_phase.clamp(0.0, 1.0) * 0.62).clamp(0.0, 0.8)
    }
}

pub fn progress_from_detail(detail: &str) -> Option<f32> {
    let percent = detail
        .trim()
        .strip_suffix('%')?
        .trim()
        .parse::<f32>()
        .ok()?;
    Some((percent / 100.0).clamp(0.0, 1.0))
}

pub fn selection_emphasis(
    selected: bool,
    hovered: bool,
    reduced_motion: bool,
    time_seconds: f64,
) -> f32 {
    let base = if selected {
        0.72
    } else if hovered {
        0.42
    } else {
        0.0
    };
    if reduced_motion || base == 0.0 {
        base
    } else {
        (base + normalized_phase(time_seconds, 1.15) * 0.18).clamp(0.0, 1.0)
    }
}

pub fn transition_progress(
    reduced_motion: bool,
    now_seconds: f64,
    started_seconds: f64,
    duration_seconds: f32,
) -> f32 {
    if reduced_motion || duration_seconds <= 0.0 {
        return 1.0;
    }
    let elapsed = (now_seconds - started_seconds).max(0.0) as f32;
    ease_out_cubic((elapsed / duration_seconds).clamp(0.0, 1.0))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn carousel_wraps_in_both_directions() {
        let mut carousel = CarouselState::new(3);
        assert_eq!(carousel.current(), 0);
        carousel.previous();
        assert_eq!(carousel.current(), 2);
        carousel.next();
        assert_eq!(carousel.current(), 0);
        carousel.next();
        assert_eq!(carousel.current(), 1);
    }

    #[test]
    fn carousel_selection_is_clamped_to_last_item() {
        let mut carousel = CarouselState::new(3);
        carousel.select(99);
        assert_eq!(carousel.current(), 2);
    }

    #[test]
    fn easing_keeps_expected_endpoints() {
        assert_eq!(ease_out_cubic(0.0), 0.0);
        assert_eq!(ease_out_cubic(1.0), 1.0);
        assert_eq!(ease_out_cubic(-1.0), 0.0);
        assert_eq!(ease_out_cubic(2.0), 1.0);
    }

    #[test]
    fn reduced_motion_disables_decorative_motion() {
        assert_eq!(motion_amount(true, 42.0, 0.8), 0.0);
        assert_eq!(transition_progress(true, 12.0, 10.0, 0.3), 1.0);
    }

    #[test]
    fn transition_progress_clamps_before_and_after_window() {
        assert_eq!(transition_progress(false, 9.0, 10.0, 0.4), 0.0);
        assert_eq!(transition_progress(false, 10.4, 10.0, 0.4), 1.0);
        assert_eq!(transition_progress(false, 20.0, 10.0, 0.4), 1.0);
    }
    #[test]
    fn queue_progress_is_deterministic_for_terminal_states() {
        assert_eq!(queue_progress(true, false, 0.3), 1.0);
        assert_eq!(queue_progress(false, true, 0.3), 1.0);
        assert!(queue_progress(false, false, 0.0) < 1.0);
        assert!(queue_progress(false, false, 1.0) <= 0.8);
    }

    #[test]
    fn selection_emphasis_respects_reduced_motion() {
        assert_eq!(selection_emphasis(true, false, true, 99.0), 0.72);
        assert_eq!(selection_emphasis(false, true, true, 99.0), 0.42);
        assert_eq!(selection_emphasis(false, false, true, 99.0), 0.0);
        assert!(selection_emphasis(true, false, false, 1.0) >= 0.72);
    }

    #[test]
    fn parses_percent_activity_detail() {
        assert_eq!(progress_from_detail("15%"), Some(0.15));
        assert_eq!(progress_from_detail("100%"), Some(1.0));
        assert_eq!(progress_from_detail("125%"), Some(1.0));
        assert_eq!(progress_from_detail("Complete"), None);
    }
}
