// src/effects/power_on_effect.rs

use nannou::prelude::*;

use std::cell::RefCell;
use std::collections::HashMap;
use super::SegmentEffect;
use crate::views::DrawStyle;

struct SegmentState {
    activation_time: f32,
    target_color: Rgb<f32>,
}

pub struct PowerOnEffect {
    // Track activation times and target colors for each segment
    segment_states: RefCell<HashMap<String, SegmentState>>,
    //Effect parameters
    target_color: Rgb<f32>,
    flash_duration: f32,
    fade_duration: f32,
}


impl PowerOnEffect {
    pub fn new(target_color: Rgb<f32>, flash_duration: f32, fade_duration: f32) -> Self {
        Self {
            segment_states: RefCell::new(HashMap::new()),
            target_color,
            flash_duration,
            fade_duration,
        }
    }

    // an additional helper that maybe we don't need
    pub fn is_animating(&self, segment_id: &str, current_time: f32) -> bool {
        if let Some(state) = self.segment_states.borrow().get(segment_id) {
            let elapsed_time = current_time - state.activation_time;
            elapsed_time <= self.flash_duration + self.fade_duration
        } else {
            false
        }
    }
}

impl SegmentEffect for PowerOnEffect {

    fn apply_to_segment(&self, segment_id: &str, target_style: &DrawStyle, current_time: f32) -> DrawStyle {
        let mut states = self.segment_states.borrow_mut();
        if let Some(state) = states.get(segment_id) {
            let elapsed_time = current_time - state.activation_time;
            
            // Calculate color based on animation phase
            let color = if elapsed_time <= self.flash_duration {
                // Initial white flash phase
                let _flash_progress = elapsed_time / self.flash_duration;
                let brightness = 1.0; // Full white
                rgb(brightness, brightness, brightness)
            } else if elapsed_time <= self.flash_duration + self.fade_duration {
                // Fade to target color
                let fade_progress = (elapsed_time - self.flash_duration) / self.fade_duration;
                let white = rgb(1.0, 1.0, 1.0);
                exp_flash(white, target_style.color, fade_progress)
                
            } else {
                // Animation complete
                states.remove(segment_id);
                target_style.color
            };

            DrawStyle {
                color, 
                stroke_weight: target_style.stroke_weight,
            }
            
        } else {
            target_style.clone()
        }   
    }

    fn activate_segment(&mut self, segment_id: &str, current_time: f32) {
        self.segment_states.borrow_mut().insert(String::from(segment_id), SegmentState {
            activation_time: current_time,
            target_color: self.target_color,
        });
    }

    fn deactivate_segment(&mut self, segment_id: &str) {
        self.segment_states.borrow_mut().remove(segment_id);
    }

    fn is_segment_active(&self, segment_id: &str) -> bool {
        self.segment_states.borrow().contains_key(segment_id)
    }

    fn is_effect_finished(&self) -> bool {
        self.segment_states.borrow().is_empty()
    }

}

fn exp_flash(start: Rgb<f32>, end: Rgb<f32>, time: f32) -> Rgb <f32> {
    let decay_rate = 5.0;                             // Steepness of curve
    let adjusted_time = 1.0 - (1.0 - time).powf(2.0); // Exponentiness of curve
    let hsl_start = Hsl::from(start); // Convert to HSL for easier manipulation
    let hsl_end = Hsl::from(end);

    let result = Hsl::new(
        hsl_end.hue,
        hsl_end.saturation,
        hsl_start.lightness + (hsl_end.lightness - hsl_start.lightness) * (1.0 - (-adjusted_time * decay_rate).exp()),
    );
    Rgb::from(result)

}

/*
fn exp_color(start: Rgb<f32>, end: Rgb<f32>, time: f32) -> Rgb<f32> {
    let decay_rate = 1.5;
    let adjusted_time = 1.0 - (1.0 - time).powf(2.0); // Adjust for a sharp drop with a long tail
    rgb(
        start.red + (end.red - start.red) * (1.0 - (-adjusted_time * decay_rate).exp()),
        start.green + (end.green - start.green) * (1.0 - (-adjusted_time * decay_rate).exp()),
        start.blue + (end.blue - start.blue) * (1.0 - (-adjusted_time * decay_rate).exp()),
    )
}


fn lerp_color(start: Rgb<f32>, end: Rgb<f32>, time: f32) -> Rgb<f32> {
    rgb(
        start.red + (end.red - start.red) * time,
        start.green + (end.green - start.green) * time,
        start.blue + (end.blue - start.blue) * time,
    )
}
*/
