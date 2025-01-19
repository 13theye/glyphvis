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

    fn apply_to_segment(&self, segment_id: &str, base_style: &DrawStyle, current_time: f32) -> DrawStyle {
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
                let color = lerp_color(white, state.target_color, fade_progress);
                println!("Fading to color: {:?}", color);
                if color.red + color.green + color.blue - state.target_color.red - state.target_color.green - state.target_color.blue < 0.1 {
                    println!{"Completing PowerOnEffect to segment {}", segment_id};
                    states.remove(segment_id);
                }
                color
            } else {
                // Animation complete
                state.target_color
            };

            DrawStyle {
                color, 
                stroke_weight: base_style.stroke_weight,
            }
            
        } else {
            base_style.clone()
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


fn lerp_color(start: Rgb<f32>, end: Rgb<f32>, time: f32) -> Rgb<f32> {
    rgb(
        start.red + (end.red - start.red) * time,
        start.green + (end.green - start.green) * time,
        start.blue + (end.blue - start.blue) * time,
    )
}
