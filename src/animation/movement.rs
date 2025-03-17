// src/animation/movement.rs
//
// The GridInstance movement manager
// scaling and rotation are not currently supported

use crate::{config::MovementConfig, views::Transform2D};

#[derive(Debug, Clone)]
pub enum EasingType {
    Linear,
    EaseInOut,
    EaseIn,
    EaseOut,
}

#[derive(Debug, Clone)]
pub struct MovementUpdate {
    pub transform: Transform2D,
}

#[derive(Debug, Clone)]
pub struct Movement {
    changes: Vec<MovementUpdate>,
    current_step: usize,
    frame_timer: f32,
    frame_duration: f32,
}

impl Movement {
    pub fn new(changes: Vec<MovementUpdate>, frame_duration: f32) -> Self {
        Self {
            changes,
            current_step: 0,
            frame_timer: 0.0,
            frame_duration,
        }
    }

    pub fn update(&mut self, dt: f32) -> bool {
        self.frame_timer += dt;
        if self.frame_timer >= self.frame_duration {
            self.frame_timer -= self.frame_duration;
            true
        } else {
            false
        }
    }

    pub fn advance(&mut self) -> Option<MovementUpdate> {
        if self.current_step < self.changes.len() {
            let current_change = self.changes[self.current_step].clone();
            self.current_step += 1;
            Some(current_change)
        } else {
            None
        }
    }

    pub fn is_complete(&self) -> bool {
        self.current_step >= self.changes.len()
    }

    pub fn get_current_step(&self) -> usize {
        self.current_step
    }

    pub fn get_changes(&self) -> &Vec<MovementUpdate> {
        &self.changes
    }

    pub fn get_frame_duration(&self) -> f32 {
        self.frame_duration
    }
}

pub struct MovementEngine {
    pub config: MovementConfig,
    pub steps: usize,
}

impl MovementEngine {
    pub fn new(config: MovementConfig) -> Self {
        let steps = if config.duration == 0.0 {
            2
        } else {
            (config.duration * 60.0).floor() as usize
        };
        Self { config, steps }
    }

    pub fn generate_movement(&self, start: Transform2D, end: Transform2D) -> Vec<MovementUpdate> {
        let mut changes = Vec::with_capacity(self.steps);

        // Calculate total deltas
        let total_translation = end.translation - start.translation;
        //let total_rotation = end.rotation - start.rotation;
        //let total_scale_change = end.scale - start.scale;

        for step in 0..self.steps {
            let t = step as f32 / (self.steps - 1) as f32;
            let eased_t = match self.config.easing {
                EasingType::Linear => t,
                EasingType::EaseInOut => ease_in_out(t),
                EasingType::EaseIn => ease_in(t),
                EasingType::EaseOut => ease_out(t),
            };

            // if this isn't the first step, calculate the delta from previous step
            let previous_t = if step == 0 {
                0.0
            } else {
                (step - 1) as f32 / (self.steps - 1) as f32
            };
            let previous_eased_t = match self.config.easing {
                EasingType::Linear => previous_t,
                EasingType::EaseInOut => ease_in_out(previous_t),
                EasingType::EaseIn => ease_in(previous_t),
                EasingType::EaseOut => ease_out(previous_t),
            };

            let translation_delta = total_translation * (eased_t - previous_eased_t);
            //let rotation_delta = total_rotation * (eased_t - previous_eased_t);
            //let scale_delta = total_scale_change * (eased_t - previous_eased_t);

            let transform = Transform2D {
                translation: translation_delta,
                rotation: 0.0,
                scale: 1.0,
            };

            changes.push(MovementUpdate { transform });
        }
        changes
    }
}

fn ease_in_out(t: f32) -> f32 {
    if t < 0.5 {
        2.0 * t * t
    } else {
        -1.0 + (4.0 - 2.0 * t) * t
    }
}

fn ease_in(t: f32) -> f32 {
    t * t
}

fn ease_out(t: f32) -> f32 {
    t * (2.0 - t)
}
