//src/utilities/easing.rs

// easing functions for style transitions
// todo: refactor for generics

use nannou::prelude::*;

pub fn color_exp_ease(start: Rgba<f32>, end: Rgba<f32>, time: f32, decay_rate: f32) -> Rgba<f32> {
    let adjusted_time = 1.0 - (1.0 - time).powf(2.0); // Exponentiness of curve
    let hsl_start = Hsla::from(start); // Convert to HSL for easier manipulation
    let hsl_end = Hsla::from(end);

    let result = Hsla::new(
        hsl_start.hue,
        hsl_start.saturation
            + (hsl_end.saturation - hsl_start.saturation)
                * (1.0 - (-adjusted_time * decay_rate).exp()),
        hsl_start.lightness
            + (hsl_end.lightness - hsl_start.lightness)
                * (1.0 - (-adjusted_time * decay_rate).exp()),
        hsl_start.alpha
            + (hsl_end.alpha - hsl_start.alpha) * (1.0 - (-adjusted_time * decay_rate).exp()),
    );
    Rgba::from(result)
}

pub fn log_ease(start: Rgba<f32>, end: Rgba<f32>, time: f32, curve_strength: f32) -> Rgba<f32> {
    let adjusted_time = (time * curve_strength + 1.0).ln() / (curve_strength + 1.0).ln(); // Logarithmic curve adjustment

    let hsl_start = Hsla::from(start);
    let hsl_end = Hsla::from(end);

    let result = Hsla::new(
        hsl_start.hue,
        hsl_start.saturation + (hsl_end.saturation - hsl_start.saturation) * adjusted_time,
        hsl_start.lightness + (hsl_end.lightness - hsl_start.lightness) * adjusted_time,
        hsl_start.alpha + (hsl_end.alpha - hsl_start.alpha) * adjusted_time,
    );

    Rgba::from(result)
}
