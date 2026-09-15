use crate::FRAME_SAMPLES;
use std::collections::VecDeque;

pub const MAX_REFERENCE_DELAY_FRAMES: usize = 200;
const ALIGNMENT_WINDOW_FRAMES: usize = 32;
const MIN_ALIGNMENT_FRAMES: usize = 20;
const MIN_ALIGNMENT_CORRELATION: f32 = 0.12;
const MAX_FINE_SHIFT_SAMPLES: i32 = 240;
const FINE_SHIFT_STEP: usize = 4;

#[derive(Debug, Clone)]
struct ReferenceFrame {
    audio: [f32; FRAME_SAMPLES],
    level_dbfs: f32,
}

#[derive(Debug, Clone)]
pub struct AlignedReference {
    pub frame: [f32; FRAME_SAMPLES],
    pub delay_ms: u32,
    pub confidence: f32,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct VoiceOnlySubtraction {
    pub correlation: f32,
    pub removed_db: f32,
    pub playback_dominant: bool,
}

#[derive(Debug)]
pub struct VoiceOnlyProcessor {
    render_history: VecDeque<ReferenceFrame>,
    capture_levels: VecDeque<f32>,
    delay_frames: usize,
    delay_confidence: f32,
    delay_ready: bool,
}

impl Default for VoiceOnlyProcessor {
    fn default() -> Self {
        Self {
            render_history: VecDeque::with_capacity(
                MAX_REFERENCE_DELAY_FRAMES + ALIGNMENT_WINDOW_FRAMES + 4,
            ),
            capture_levels: VecDeque::with_capacity(ALIGNMENT_WINDOW_FRAMES),
            delay_frames: 0,
            delay_confidence: 0.0,
            delay_ready: false,
        }
    }
}

impl VoiceOnlyProcessor {
    pub fn observe_render(&mut self, render: &[f32; FRAME_SAMPLES]) {
        let capacity = MAX_REFERENCE_DELAY_FRAMES + ALIGNMENT_WINDOW_FRAMES + 4;
        if self.render_history.len() >= capacity {
            self.render_history.pop_front();
        }
        self.render_history.push_back(ReferenceFrame {
            audio: *render,
            level_dbfs: rms_db(render),
        });
    }

    pub fn aligned_reference(
        &mut self,
        capture: &[f32; FRAME_SAMPLES],
    ) -> Option<AlignedReference> {
        if self.capture_levels.len() >= ALIGNMENT_WINDOW_FRAMES {
            self.capture_levels.pop_front();
        }
        self.capture_levels.push_back(rms_db(capture));

        let window = self.capture_levels.len().min(ALIGNMENT_WINDOW_FRAMES);
        if window >= MIN_ALIGNMENT_FRAMES && self.render_history.len() >= window {
            let capture_levels = self
                .capture_levels
                .iter()
                .skip(self.capture_levels.len() - window)
                .copied()
                .collect::<Vec<_>>();
            let max_delay =
                MAX_REFERENCE_DELAY_FRAMES.min(self.render_history.len().saturating_sub(window));
            let mut best_delay = self.delay_frames.min(max_delay);
            let mut best_correlation = -1.0f32;

            for delay in 0..=max_delay {
                let start = self.render_history.len() - window - delay;
                let render_levels = self
                    .render_history
                    .iter()
                    .skip(start)
                    .take(window)
                    .map(|frame| frame.level_dbfs)
                    .collect::<Vec<_>>();
                let correlation = pearson(&capture_levels, &render_levels);
                if correlation > best_correlation {
                    best_correlation = correlation;
                    best_delay = delay;
                }
            }

            if best_correlation >= MIN_ALIGNMENT_CORRELATION {
                if self.delay_ready {
                    let weighted = ((self.delay_frames * 3 + best_delay + 2) / 4)
                        .min(MAX_REFERENCE_DELAY_FRAMES);
                    self.delay_frames =
                        if weighted == self.delay_frames && best_delay != self.delay_frames {
                            if best_delay > self.delay_frames {
                                self.delay_frames + 1
                            } else {
                                self.delay_frames.saturating_sub(1)
                            }
                        } else {
                            weighted
                        };
                } else {
                    self.delay_frames = best_delay;
                    self.delay_ready = true;
                }
                self.delay_confidence = best_correlation.clamp(0.0, 1.0);
            } else if self.delay_ready {
                self.delay_confidence = (self.delay_confidence * 0.98).max(0.0);
            }
        }

        if self.render_history.is_empty() {
            return None;
        }

        let delay = if self.delay_ready {
            self.delay_frames.min(self.render_history.len() - 1)
        } else {
            0
        };
        let index = self.render_history.len() - 1 - delay;
        let frame = self.render_history.get(index)?.audio;
        Some(AlignedReference {
            frame,
            delay_ms: (delay * 10) as u32,
            confidence: if self.delay_ready {
                self.delay_confidence
            } else {
                0.0
            },
        })
    }

    pub fn subtract_correlated_playback(
        &mut self,
        capture: &mut [f32; FRAME_SAMPLES],
        reference: &[f32; FRAME_SAMPLES],
        strength_percent: f32,
    ) -> VoiceOnlySubtraction {
        let before = rms(capture);
        if before <= 1.0e-7 || rms(reference) <= 1.0e-7 {
            return VoiceOnlySubtraction::default();
        }

        let mut best_correlation = 0.0f32;
        let mut best_gain = 0.0f32;
        let mut best_shift = 0i32;
        for shift in (-MAX_FINE_SHIFT_SAMPLES..=MAX_FINE_SHIFT_SAMPLES).step_by(FINE_SHIFT_STEP) {
            let (correlation, gain) = shifted_correlation_and_gain(capture, reference, shift);
            if correlation > best_correlation {
                best_correlation = correlation;
                best_gain = gain;
                best_shift = shift;
            }
        }

        if best_correlation < 0.10 {
            return VoiceOnlySubtraction {
                correlation: best_correlation,
                removed_db: 0.0,
                playback_dominant: false,
            };
        }

        let strength = (strength_percent / 100.0).clamp(0.0, 1.0);
        let gain = best_gain.clamp(-4.0, 4.0) * strength;
        for i in 0..FRAME_SAMPLES {
            let j = i as i32 + best_shift;
            if (0..FRAME_SAMPLES as i32).contains(&j) {
                capture[i] -= reference[j as usize] * gain;
            }
        }

        let after = rms(capture);
        let removed_db = if after > 1.0e-7 {
            (20.0 * (before / after).max(1.0).log10()).clamp(0.0, 72.0)
        } else {
            72.0
        };
        VoiceOnlySubtraction {
            correlation: best_correlation,
            removed_db,
            playback_dominant: best_correlation >= 0.30 && removed_db >= 6.0,
        }
    }
}

fn shifted_correlation_and_gain(
    capture: &[f32; FRAME_SAMPLES],
    reference: &[f32; FRAME_SAMPLES],
    shift: i32,
) -> (f32, f32) {
    let mut dot = 0.0f32;
    let mut aa = 0.0f32;
    let mut bb = 0.0f32;
    for (i, &x) in capture.iter().enumerate() {
        let j = i as i32 + shift;
        if !(0..FRAME_SAMPLES as i32).contains(&j) {
            continue;
        }
        let y = reference[j as usize];
        dot += x * y;
        aa += x * x;
        bb += y * y;
    }
    if aa <= 1.0e-9 || bb <= 1.0e-9 {
        return (0.0, 0.0);
    }
    ((dot / (aa * bb).sqrt()).abs().clamp(0.0, 1.0), dot / bb)
}

fn pearson(a: &[f32], b: &[f32]) -> f32 {
    let n = a.len().min(b.len());
    if n < 2 {
        return 0.0;
    }
    let a = &a[a.len() - n..];
    let b = &b[b.len() - n..];
    let mean_a = a.iter().sum::<f32>() / n as f32;
    let mean_b = b.iter().sum::<f32>() / n as f32;
    let mut dot = 0.0f32;
    let mut aa = 0.0f32;
    let mut bb = 0.0f32;
    for (x, y) in a.iter().zip(b) {
        let dx = *x - mean_a;
        let dy = *y - mean_b;
        dot += dx * dy;
        aa += dx * dx;
        bb += dy * dy;
    }
    if aa <= 1.0e-9 || bb <= 1.0e-9 {
        0.0
    } else {
        (dot / (aa * bb).sqrt()).clamp(-1.0, 1.0)
    }
}

fn rms(frame: &[f32; FRAME_SAMPLES]) -> f32 {
    (frame.iter().map(|sample| sample * sample).sum::<f32>() / FRAME_SAMPLES as f32).sqrt()
}

fn rms_db(frame: &[f32; FRAME_SAMPLES]) -> f32 {
    20.0 * rms(frame).max(1.0e-6).log10()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::FRAME_SAMPLES;

    fn render_frame(index: usize) -> [f32; FRAME_SAMPLES] {
        let amplitude =
            0.10 + 0.055 * (index as f32 * 0.113).sin() + 0.025 * (index as f32 * 0.037).sin();
        let mut frame = [0.0f32; FRAME_SAMPLES];
        for (sample_index, sample) in frame.iter_mut().enumerate() {
            let phase = sample_index as f32 * (0.041 + (index % 7) as f32 * 0.0021);
            *sample = amplitude * (phase.sin() + 0.31 * (phase * 2.17).sin());
        }
        frame
    }

    #[test]
    fn finds_nine_hundred_millisecond_playback_delay() {
        let mut processor = VoiceOnlyProcessor::default();
        let delay_frames = 90usize;
        let mut observed_delay = None;
        let history = (0..360).map(render_frame).collect::<Vec<_>>();
        for index in 0..history.len() {
            processor.observe_render(&history[index]);
            let capture = if index >= delay_frames {
                history[index - delay_frames].map(|sample| sample * 0.42)
            } else {
                [0.0; FRAME_SAMPLES]
            };
            observed_delay = processor
                .aligned_reference(&capture)
                .map(|value| value.delay_ms);
        }
        let delay = observed_delay.expect("delay alignment should become available");
        assert!(
            (860..=940).contains(&delay),
            "unexpected aligned delay {delay}ms"
        );
    }

    #[test]
    fn subtraction_preserves_uncorrelated_local_voice() {
        let mut processor = VoiceOnlyProcessor::default();
        let reference = render_frame(31);
        let mut voice = [0.0f32; FRAME_SAMPLES];
        for (i, sample) in voice.iter_mut().enumerate() {
            *sample = 0.11 * (i as f32 * 0.067).sin() + 0.035 * (i as f32 * 0.131).sin();
        }
        let mut capture = [0.0f32; FRAME_SAMPLES];
        for i in 0..FRAME_SAMPLES {
            capture[i] = voice[i] + reference[i] * 0.72;
        }
        let result = processor.subtract_correlated_playback(&mut capture, &reference, 100.0);
        let voice_error = capture
            .iter()
            .zip(voice)
            .map(|(actual, expected)| (actual - expected) * (actual - expected))
            .sum::<f32>()
            / FRAME_SAMPLES as f32;
        assert!(
            result.correlation > 0.20,
            "reference was not recognized: {result:#?}"
        );
        assert!(
            voice_error.sqrt() < 0.05,
            "local voice was damaged too much: {voice_error}"
        );
    }

    #[test]
    fn playback_only_becomes_dominant_after_correlated_match() {
        let mut processor = VoiceOnlyProcessor::default();
        let reference = render_frame(55);
        let mut capture = reference.map(|sample| sample * 0.68);
        let result = processor.subtract_correlated_playback(&mut capture, &reference, 100.0);
        assert!(result.playback_dominant, "{result:#?}");
        assert!(result.removed_db >= 12.0, "{result:#?}");
    }
}
