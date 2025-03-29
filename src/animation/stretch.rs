// src/animation/stretch.rs
//
// Implements the stretch effect that distorts the grid along a specified axis.

use crate::models::Axis;
use crate::services::SegmentGraph;
use crate::utilities::segment_analysis;
use crate::views::{
    CachedGrid, CachedSegment, DrawCommand, DrawStyle, SegmentAction, SegmentType, StyleUpdateMsg,
};
use nannou::prelude::*;
use std::collections::{HashMap, HashSet};

pub struct StretchEffect {
    pub axis: Axis,
    pub amount: f32,
    pub target_amount: f32,
    pub start_time: f32,
    pub interpolation_duration: f32,

    pub stretch_segments: Vec<CachedSegment>,
    pub tile_offsets: Vec<f32>,
}

impl StretchEffect {
    pub fn new(axis: Axis, target_amount: f32, start_time: f32) -> Self {
        Self {
            axis,
            amount: 0.0,
            target_amount,
            start_time,
            interpolation_duration: 1.0 / 60.0,
            stretch_segments: Vec::new(),
            tile_offsets: Vec::new(),
        }
    }

    pub fn generate_stretch_segments(&mut self, grid: &CachedGrid, graph: &SegmentGraph) {}
}
