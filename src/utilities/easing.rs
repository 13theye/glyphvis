//src/utilities/easing.rs

// easing functions for style transitions
// maybe refactor to be more generic?

use nannou::prelude::*;

pub fn color_exp_ease(start: Rgb<f32>, end: Rgb<f32>, time: f32, decay_rate: f32) -> Rgb<f32> {
    let adjusted_time = 1.0 - (1.0 - time).powf(2.0); // Exponentiness of curve
    let hsl_start = Hsl::from(start); // Convert to HSL for easier manipulation
    let hsl_end = Hsl::from(end);

    let result = Hsl::new(
        hsl_start.hue,
        hsl_start.saturation
            + (hsl_end.saturation - hsl_start.saturation)
                * (1.0 - (-adjusted_time * decay_rate).exp()),
        hsl_start.lightness
            + (hsl_end.lightness - hsl_start.lightness)
                * (1.0 - (-adjusted_time * decay_rate).exp()),
    );
    Rgb::from(result)
}

pub fn log_ease(start: Rgb<f32>, end: Rgb<f32>, time: f32, curve_strength: f32) -> Rgb<f32> {
    let adjusted_time = (time * curve_strength + 1.0).ln() / (curve_strength + 1.0).ln(); // Logarithmic curve adjustment

    let hsl_start = Hsl::from(start);
    let hsl_end = Hsl::from(end);

    let result = Hsl::new(
        hsl_start.hue,
        hsl_start.saturation + (hsl_end.saturation - hsl_start.saturation) * adjusted_time,
        hsl_start.lightness + (hsl_end.lightness - hsl_start.lightness) * adjusted_time,
    );

    Rgb::from(result)
}
