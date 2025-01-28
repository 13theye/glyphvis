// src/animation/segment_animation.rs
/// Use neighbor logic to make a segment within a grid look like
/// it's moving.
use nannou::prelude::*;

#[derive(Debug, Clone)]
pub enum MoveDirection {
    Up,
    Down,
    Left,
    Right,
    UpLeft,
    UpRight,
    DownLeft,
    DownRight,
}

#[derive(Debug, Clone)]
pub struct SegmentAnimation {
    pub from_segment_id: String,
    pub to_segment_id: String,
    pub start_time: f32,
    pub duration: f32,
    pub frame_time: f32,
    pub next_move_direction: Option<MoveDirection>,
    pub is_active: bool,
}
