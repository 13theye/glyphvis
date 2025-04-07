// src/animation/movement.rs
//
// The GridInstance movement manager
// scaling and rotation are not currently supported

use crate::{
    animation::Animation,
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
pub struct InstantMovement {
    target_position: Point2,
    last_position: Point2,
    trigger_time: f32, // time when command was received
    duration: f32,     // usually equal to time between frame updates (1.0/60.0)
}

impl InstantMovement {
    pub fn new(target_position: Point2, last_position: Point2, trigger_time: f32) -> Self {
        Self {
            target_position,
            last_position,
            trigger_time,
            duration: 1.0 / 60.0,
        }
    }
}

impl Animation for InstantMovement {
    fn should_update(&mut self, _dt: f32) -> bool {
        // InstantMovement should update on very next frame
        true
    }

    fn advance(&mut self, current_position: Point2, time: f32) -> Option<MovementChange> {
        let elapsed = time - self.trigger_time;
        let progress = (elapsed / self.duration).clamp(0.0, 1.0);

        // Snap to exact target when very close to completion
        if progress > 0.99 {
            // Direct snap to target
            let delta = self.target_position - current_position;

            // Only return a transform if the delta is significant
            if delta.length() < 0.001 {
                return None;
            }

            let transform = Transform2D {
                translation: delta,
                scale: 1.0,
                rotation: 0.0,
            };

            self.last_position = self.target_position;
            return Some(MovementChange { transform });
        }

        let adjusted_target =
            interpolate_position(current_position, self.target_position, progress);

        // Apply threshold to avoid tiny movements
        let delta = adjusted_target - current_position;
        if delta.length() < 0.001 {
            return None;
        }

        let transform = Transform2D {
            translation: delta,
            scale: 1.0,
            rotation: 0.0,
        };

        self.last_position = adjusted_target;
        Some(MovementChange { transform })
    }

    fn is_complete(&self) -> bool {
        self.last_position == self.target_position
    }
}

#[derive(Debug, Clone)]
pub struct TimedMovement {
    changes: Vec<MovementChange>,
    current_step: usize,
    frame_timer: f32,
    frame_duration: f32,
}

impl TimedMovement {
    pub fn new(changes: Vec<MovementChange>, frame_duration: f32) -> Self {
        Self {
            changes,
            current_step: 0,
            frame_timer: 0.0,
            frame_duration,
        }
    }
}

impl Animation for TimedMovement {
    fn should_update(&mut self, dt: f32) -> bool {
        self.frame_timer += dt;
        if self.frame_timer >= self.frame_duration {
            self.frame_timer -= self.frame_duration;
            true
        } else {
            false
        }
    }

    fn advance(&mut self, _current_position: Point2, _time: f32) -> Option<MovementChange> {
        if self.current_step < self.changes.len() {
            let current_change = self.changes[self.current_step].clone();
            self.current_step += 1;
            Some(current_change)
        } else {
            None
        }
    }

    fn is_complete(&self) -> bool {
        self.current_step >= self.changes.len()
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
    ) -> TimedMovement {
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

        TimedMovement::new(changes, 1.0 / 60.0)
    }

    pub fn build_zero_duration_movement(
        &self,
        target_position: Point2,
        current_position: Point2,
        trigger_time: f32,
    ) -> InstantMovement {
        InstantMovement::new(target_position, current_position, trigger_time)
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

fn interpolate_position(last_position: Point2, target_position: Point2, progress: f32) -> Point2 {
    let interp_x = last_position.x + (target_position.x - last_position.x) * progress;
    let interp_y = last_position.y + (target_position.y - last_position.y) * progress;
    pt2(interp_x, interp_y)
}
