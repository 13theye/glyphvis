// segmenet_effects.rs

use nannou::prelude::*;

use crate::draw::DrawParams;

pub trait GridEffect {
    fn apply(&self, base_params: &DrawParams, time: f32) -> DrawParams;
}

pub struct PulseEffect {
    pub frequency: f32,
    pub min_brightness: f32,
    pub max_brightness: f32,
}

impl GridEffect for PulseEffect {
    fn apply(&self, base_params: &DrawParams, time: f32) -> DrawParams {
        let brightness = (time * self.frequency).sin() * 0.5 + 0.5;
        let brightness = self.min_brightness + brightness * (self.max_brightness - self.min_brightness);

        let color = base_params.color;
        DrawParams {
            color: rgb(
                color.red * brightness,
                color.green * brightness,
                color.blue * brightness,
            ),
            stroke_weight: base_params.stroke_weight,
        }
    }
}

pub struct ColorCycleEffect {
    pub frequency: f32,
    pub saturation: f32,
    pub brightness: f32,
}

impl GridEffect for ColorCycleEffect {
    fn apply(&self, base_params: &DrawParams, time: f32) -> DrawParams {
        let hue = (time * self.frequency) % 1.0;
        DrawParams {
            color: hsv(hue, self.saturation, self.brightness).into(),
            stroke_weight: base_params.stroke_weight,
        }
    }
}