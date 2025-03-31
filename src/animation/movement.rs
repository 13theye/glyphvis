// src/animation/movement.rs
//
// The GridInstance movement manager
// scaling and rotation are not currently supported

use crate::{
    config::MovementConfig,
    views::{GridInstance, Transform2D},
};
use nannou::prelude::*;

#[derive(Debug, Clone)]
pub enum EasingType {
    Linear,
    EaseInOut,
    EaseIn,
    EaseOut,
}

#[derive(Debug, Clone)]
pub struct MovementChange {
    pub transform: Transform2D,
}

#[derive(Debug, Clone)]
pub struct Movement {
    changes: Vec<MovementChange>,
    current_step: usize,
    frame_timer: f32,
    frame_duration: f32,
}

impl Movement {
    pub fn new(changes: Vec<MovementChange>, frame_duration: f32) -> Self {
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

    pub fn advance(&mut self) -> Option<MovementChange> {
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

    pub fn get_changes(&self) -> &Vec<MovementChange> {
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
            1
        } else {
            (config.duration * 60.0).floor() as usize
        };
        Self { config, steps }
    }

    pub fn build_timed_movement(
        &self,
        grid: &GridInstance,
        target_x: f32,
        target_y: f32,
    ) -> Movement {
        let target_position = pt2(target_x, target_y);

        let start_transform = Transform2D {
            translation: grid.current_position,
            scale: grid.current_scale,
            rotation: grid.current_rotation,
        };

        let end_transform = Transform2D {
            translation: target_position,
            scale: grid.current_scale,
            rotation: grid.current_rotation,
        };

        let changes = self.generate_movement_changes(start_transform, end_transform);

        Movement::new(changes, 1.0 / 60.0)
    }

    fn generate_movement_changes(
        &self,
        start: Transform2D,
        end: Transform2D,
    ) -> Vec<MovementChange> {
        let mut changes = Vec::with_capacity(self.steps);

        // Calculate total deltas
        let total_translation = end.translation - start.translation;

        for step in 0..self.steps {
            let t = if self.steps > 1 {
                step as f32 / (self.steps - 1) as f32
            } else {
                1.0
            };
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

            changes.push(MovementChange { transform });
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
