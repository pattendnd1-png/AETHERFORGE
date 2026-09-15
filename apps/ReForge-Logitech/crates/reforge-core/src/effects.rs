use crate::{LightingEffect, LightingState, RgbColor};

pub fn render_host_frame(
    state: &LightingState,
    keys: &[u8],
    elapsed_ms: u64,
    reactive_color: Option<RgbColor>,
    audio_level: Option<u8>,
) -> Vec<(u8, RgbColor)> {
    if !state.enabled || state.effect == LightingEffect::Off {
        return keys.iter().copied().map(|key| (key, RgbColor::BLACK)).collect();
    }

    let intensity = state.intensity.min(100);
    let count = keys.len().max(1);
    keys.iter()
        .copied()
        .enumerate()
        .map(|(index, key)| {
            let color = match state.effect {
                LightingEffect::ScreenReactive => reactive_color.unwrap_or(state.primary),
                LightingEffect::AudioReactive => {
                    let amount = audio_level.unwrap_or(0);
                    state.secondary.blend(state.primary, amount)
                }
                LightingEffect::Gradient => {
                    let amount = if count <= 1 {
                        0
                    } else {
                        ((index * 255) / (count - 1)) as u8
                    };
                    state.primary.blend(state.secondary, amount)
                }
                LightingEffect::Breathing => {
                    let period = u64::from(state.period_ms.max(250));
                    let phase = (elapsed_ms % period) as f32 / period as f32;
                    let amount = (((phase * std::f32::consts::TAU).sin() + 1.0) * 127.5) as u8;
                    RgbColor::BLACK.blend(state.primary, amount)
                }
                LightingEffect::ColorCycle => hsv_to_rgb(
                    (((elapsed_ms / 15) + index as u64 * 5) % 360) as u16,
                    255,
                    255,
                ),
                LightingEffect::Wave => {
                    let period = u64::from(state.period_ms.max(250));
                    let phase = ((elapsed_ms % period) as f32 / period as f32)
                        + index as f32 / count as f32;
                    let amount = (((phase * std::f32::consts::TAU).sin() + 1.0) * 127.5) as u8;
                    state.primary.blend(state.secondary, amount)
                }
                LightingEffect::Ripple => {
                    let period = u64::from(state.period_ms.max(250));
                    let center = ((elapsed_ms % period) as f32 / period as f32) * count as f32;
                    let distance = (index as f32 - center).abs();
                    let amount = (255.0 * (1.0 - (distance / 6.0).min(1.0))) as u8;
                    state.secondary.blend(state.primary, amount)
                }
                _ => state
                    .per_key
                    .get(&key)
                    .copied()
                    .unwrap_or(state.primary),
            };
            (key, color.scale(intensity))
        })
        .collect()
}

fn hsv_to_rgb(hue: u16, saturation: u8, value: u8) -> RgbColor {
    let h = (hue % 360) as f32 / 60.0;
    let s = saturation as f32 / 255.0;
    let v = value as f32 / 255.0;
    let c = v * s;
    let x = c * (1.0 - ((h % 2.0) - 1.0).abs());
    let m = v - c;
    let (r, g, b) = match h as u8 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    RgbColor::new(
        ((r + m) * 255.0).round() as u8,
        ((g + m) * 255.0).round() as u8,
        ((b + m) * 255.0).round() as u8,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gradient_places_endpoints_at_requested_colors() {
        let state = LightingState {
            effect: LightingEffect::Gradient,
            primary: RgbColor::new(255, 0, 0),
            secondary: RgbColor::new(0, 0, 255),
            ..Default::default()
        };
        let frame = render_host_frame(&state, &[4, 5, 6], 0, None, None);
        assert_eq!(frame[0].1, state.primary);
        assert_eq!(frame[2].1, state.secondary);
    }

    #[test]
    fn screen_effect_uses_sampled_color() {
        let state = LightingState {
            effect: LightingEffect::ScreenReactive,
            ..Default::default()
        };
        let sample = RgbColor::new(12, 34, 56);
        let frame = render_host_frame(&state, &[4, 5], 0, Some(sample), None);
        assert!(frame.iter().all(|(_, color)| *color == sample));
    }

    #[test]
    fn audio_effect_blends_from_secondary_to_primary() {
        let state = LightingState {
            effect: LightingEffect::AudioReactive,
            primary: RgbColor::WHITE,
            secondary: RgbColor::BLACK,
            ..Default::default()
        };
        assert_eq!(render_host_frame(&state, &[4], 0, None, Some(0))[0].1, RgbColor::BLACK);
        assert_eq!(render_host_frame(&state, &[4], 0, None, Some(255))[0].1, RgbColor::WHITE);
    }
}
