// src/animation/slide_movement.rs
//
// Tears the rows and columns of a grid apart visually.

use crate::models::Axis;

pub struct SlideAnimation {
    pub axis: Axis,
    pub index: i32,
    pub start_position: f32,
    pub current_position: f32,
    pub target_position: f32,
    pub start_time: f32,
    pub duration: f32,
}
