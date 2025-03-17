// src/effects/grid_effects.rs
// these effects are applied to sets of segments, like Glyphs and Grids.

use super::BackboneEffect;
use crate::views::DrawStyle;
use nannou::prelude::*;

pub struct PulseEffect {
    pub frequency: f32,
    pub min_brightness: f32,
    pub max_brightness: f32,
}

impl BackboneEffect for PulseEffect {
    fn update(&self, start_style: &DrawStyle, time: f32) -> DrawStyle {
        let brightness = (time * self.frequency).sin() * 0.5 + 0.5;
        let brightness =
            self.min_brightness + brightness * (self.max_brightness - self.min_brightness);

        let color = start_style.color;
        DrawStyle {
            color: rgba(
                color.red * brightness,
                color.green * brightness,
                color.blue * brightness,
                color.alpha,
            ),
            stroke_weight: start_style.stroke_weight,
        }
    }

    // this is a continuous effect
    fn is_finished(&self, _time: f32) -> bool {
        false
    }
}

pub struct ColorCycleEffect {
    pub frequency: f32,
    pub saturation: f32,
    pub brightness: f32,
    pub alpha: f32,
}

impl BackboneEffect for ColorCycleEffect {
    fn update(&self, base_style: &DrawStyle, time: f32) -> DrawStyle {
        let hue = (time * self.frequency) % 1.0;
        DrawStyle {
            color: hsla(hue, self.saturation, self.brightness, self.alpha).into(),
            stroke_weight: base_style.stroke_weight,
        }
    }

    fn is_finished(&self, _time: f32) -> bool {
        false
    }
}

pub struct FadeEffect {
    pub base_style: DrawStyle,
    pub target_style: DrawStyle,
    pub duration: f32,
    pub start_time: f32,
    pub is_active: bool,
}

impl BackboneEffect for FadeEffect {
    fn update(&self, current_style: &DrawStyle, time: f32) -> DrawStyle {
        // if time is 0.0, immediately change to target style.
        if self.duration.abs() < 0.001 {
            return self.target_style.clone();
        }

        let elapsed = time - self.start_time;
        let t = (elapsed / self.duration).clamp(0.0, 1.0);

        let base_color: Hsla<_, _> = Hsla::from(self.base_style.color);
        let base_hue: f32 = base_color.hue.into();

        let target_color = Hsla::from(self.target_style.color);
        let target_hue: f32 = target_color.hue.into();

        let new_hue = nannou::color::RgbHue::from(base_hue + (target_hue - base_hue) * t);

        let interpolated_color = Hsla::new(
            new_hue,
            base_color.saturation + (target_color.saturation - base_color.saturation) * t,
            base_color.lightness + (target_color.lightness - base_color.lightness) * t,
            base_color.alpha + (target_color.alpha - base_color.alpha) * t,
        );

        DrawStyle {
            color: Rgba::from(interpolated_color),
            ..*current_style
        }
    }

    fn is_finished(&self, time: f32) -> bool {
        let elapsed = time - self.start_time;
        elapsed > self.duration
    }
}
