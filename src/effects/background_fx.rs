// src/effects/background_flash.rs

use nannou::prelude::*;

#[derive(Debug, Default)]
pub struct BackgroundFlash {
    color: Rgb,
    start_time: f32,
    duration: f32,
    is_active: bool,
}

impl BackgroundFlash {
    pub fn new() -> Self {
        Self {
            color: rgb(0.0, 0.0, 0.0),
            start_time: 0.0,
            duration: 0.0,
            is_active: false,
        }
    }

    pub fn start_flash(&mut self, color: Rgb, duration: f32, current_time: f32) {
        self.color = color;
        self.duration = duration;
        self.start_time = current_time;
        self.is_active = true;
    }

    pub fn get_current_color(&mut self, current_time: f32) -> Rgb {
        if !self.is_active {
            return rgb(0.0, 0.0, 0.0);
        }

        let elapsed = current_time - self.start_time;
        if elapsed >= self.duration {
            self.is_active = false;
            return rgb(0.0, 0.0, 0.0);
        }

        // Calculate alpha based on time elapsed
        let progress = elapsed / self.duration;
        let alpha = 1.0 - progress; // Linear fade out

        // Blend with black background
        rgb(
            self.color.red * alpha,
            self.color.green * alpha,
            self.color.blue * alpha,
        )
    }
}
