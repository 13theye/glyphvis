// src/animation/stretch_effect.rs
//
// Implements the stretch effect that modifies how a grid looks.
// This effect distorts the grid along a specified axis.

use crate::services::SegmentGraph;
use crate::utilities::segment_analysis;
use crate::views::{
    CachedGrid, CachedSegment, DrawCommand, DrawStyle, SegmentAction, SegmentType, StyleUpdateMsg,
};
use nannou::prelude::*;
use std::collections::{HashMap, HashSet};

pub struct StretchEffect {
    pub active: bool,
    pub axis: char,         // 'x' or 'y'
    pub amount: f32,        // Current stretch amount
    pub target_amount: f32, // Target stretch amount
    pub start_time: f32,    // Time when the stretch started
    pub duration: f32,      // Duration of the stretch animation (for interpolation)

    // Maps boundary index to current position offset
    pub boundary_offsets: HashMap<i32, f32>,

    // Maps boundary index to stretch segments on that boundary
    pub stretch_segments: HashMap<i32, Vec<String>>,

    // Keeps track of created segments for cleanup
    pub created_segments: HashSet<String>,
}

impl Default for StretchEffect {
    fn default() -> Self {
        Self {
            active: false,
            axis: 'x',
            amount: 0.0,
            target_amount: 0.0,
            start_time: 0.0,
            duration: 1.0 / 60.0, // Default to 1 frame for immediate effect
            boundary_offsets: HashMap::new(),
            stretch_segments: HashMap::new(),
            created_segments: HashSet::new(),
        }
    }
}

impl StretchEffect {
    pub fn new() -> Self {
        Self::default()
    }

    // Start a new stretch effect
    pub fn start(&mut self, axis: char, amount: f32, time: f32) {
        self.active = true;
        self.axis = axis;
        self.target_amount = amount;
        self.start_time = time;

        // Save previous amounts for interpolation
        if self.axis != axis {
            // Reset if changing axis
            self.amount = 0.0;
            self.boundary_offsets.clear();
        }
    }

    // Update the target amount for an ongoing stretch
    pub fn update_target(&mut self, amount: f32, time: f32) {
        self.target_amount = amount;
        self.start_time = time;
    }

    // Stop the stretch effect
    pub fn stop(&mut self) {
        self.active = false;
        self.amount = 0.0;
        self.target_amount = 0.0;
        self.boundary_offsets.clear();
    }

    // Check if the stretch effect is active
    pub fn is_active(&self) -> bool {
        self.active
    }

    // Calculate the current stretch amount based on interpolation
    pub fn calculate_current_amount(&self, current_time: f32) -> f32 {
        if !self.active {
            return 0.0;
        }

        let elapsed = current_time - self.start_time;
        let progress = (elapsed / self.duration).clamp(0.0, 1.0);

        // Linear interpolation between current and target amount
        self.amount + (self.target_amount - self.amount) * progress
    }

    // Generate stretch segments at a specific boundary
    pub fn generate_stretch_segments(
        &mut self,
        grid: &mut CachedGrid,
        graph: &SegmentGraph,
        boundary_index: i32,
    ) -> Vec<String> {
        let mut stretch_segments = Vec::new();

        // Find intersections at the boundary
        let intersections = if self.axis == 'x' {
            segment_analysis::find_x_boundary_intersections(grid, graph, boundary_index as u32)
        } else {
            segment_analysis::find_y_boundary_intersections(grid, graph, boundary_index as u32)
        };

        // Create stretch segments for each intersection
        for (i, (intersection_point, connected_segments)) in intersections.iter().enumerate() {
            let segment_id = format!("stretch-{}-{}-{}", self.axis, boundary_index, i);

            // Create stretch segment
            let segment = self.create_stretch_segment(
                segment_id.clone(),
                *intersection_point,
                boundary_index as u32,
                self.axis,
                grid,
            );

            // Add to grid
            grid.segments.insert(segment_id.clone(), segment);
            stretch_segments.push(segment_id.clone());
            self.created_segments.insert(segment_id);
        }

        stretch_segments
    }

    // Create a new stretch segment at an intersection point
    fn create_stretch_segment(
        &self,
        id: String,
        intersection: Point2,
        boundary: u32,
        axis: char,
        grid: &CachedGrid,
    ) -> CachedSegment {
        // Determine direction based on axis
        let (start, end) = if axis == 'x' {
            // Create a horizontal line for x-axis stretch
            (
                pt2(intersection.x - 0.1, intersection.y),
                pt2(intersection.x + 0.1, intersection.y),
            )
        } else {
            // Create a vertical line for y-axis stretch
            (
                pt2(intersection.x, intersection.y - 0.1),
                pt2(intersection.x, intersection.y + 0.1),
            )
        };

        // Create draw command
        let draw_command = DrawCommand::Line { start, end };

        // Determine segment type based on axis
        let segment_type = if axis == 'x' {
            SegmentType::Horizontal
        } else {
            SegmentType::Vertical
        };

        // Create segment
        let mut segment = CachedSegment {
            id: id.clone(),
            tile_coordinate: (boundary, 0), // Use boundary as x-coordinate
            segment_type,
            layer: crate::views::Layer::Background,
            state: crate::views::SegmentState::Idle {
                style: DrawStyle::default(),
            },
            draw_commands: vec![draw_command],
            original_path: crate::models::PathElement::Line {
                x1: start.x,
                y1: start.y,
                x2: end.x,
                y2: end.y,
            },
            edge_type: crate::models::EdgeType::None,
        };

        segment
    }

    // Update the stretch segments based on active segments
    pub fn update_stretch_segment_styles(
        &self,
        grid: &CachedGrid,
        active_segments: &HashSet<String>,
        target_style: &DrawStyle,
        update_batch: &mut HashMap<String, StyleUpdateMsg>,
    ) {
        for stretch_id in &self.created_segments {
            if let Some(stretch_segment) = grid.segments.get(stretch_id) {
                // Check if this stretch segment intersects any active segments
                let is_active =
                    self.check_stretch_segment_active(stretch_segment, active_segments, grid);

                if is_active {
                    // Set to target style if active
                    update_batch.insert(
                        stretch_id.clone(),
                        StyleUpdateMsg::new(
                            SegmentAction::InstantStyleChange,
                            target_style.clone(),
                        ),
                    );
                } else {
                    // Set to inactive/backbone style
                    update_batch.insert(
                        stretch_id.clone(),
                        StyleUpdateMsg::new(
                            SegmentAction::BackboneUpdate,
                            DrawStyle {
                                color: rgba(0.19, 0.19, 0.19, 1.0),
                                stroke_weight: target_style.stroke_weight,
                            },
                        ),
                    );
                }
            }
        }
    }

    // Check if a stretch segment should be active (visible)
    fn check_stretch_segment_active(
        &self,
        stretch_segment: &CachedSegment,
        active_segments: &HashSet<String>,
        grid: &CachedGrid,
    ) -> bool {
        // Simple approach: check if any active segments are in the same tile
        for segment_id in active_segments {
            if let Some(segment) = grid.segments.get(segment_id) {
                if segment.tile_coordinate.0 == stretch_segment.tile_coordinate.0 {
                    return true;
                }
            }
        }

        false
    }

    // Update the positions of stretch segments
    pub fn update_stretch_segments(
        &mut self,
        grid: &mut CachedGrid,
        current_amount: f32,
        prev_amount: f32,
    ) {
        // Calculate the delta from the previous position
        let delta = current_amount - prev_amount;

        if delta.abs() < 0.001 {
            return; // No significant change
        }

        // Update each stretch segment
        for (boundary_index, segments) in &self.stretch_segments {
            // Calculate the movement for this boundary
            let current_offset = self.boundary_offsets.entry(*boundary_index).or_insert(0.0);
            *current_offset += delta;

            // Apply to all segments at this boundary
            for segment_id in segments {
                if let Some(segment) = grid.segments.get_mut(segment_id) {
                    for command in &mut segment.draw_commands {
                        match command {
                            DrawCommand::Line { start, end } => {
                                if self.axis == 'x' {
                                    start.x += delta;
                                    end.x += delta;
                                } else {
                                    start.y += delta;
                                    end.y += delta;
                                }
                            }
                            _ => { /* Other command types not relevant for stretch segments */ }
                        }
                    }
                }
            }
        }
    }

    // Clean up stretch segments
    pub fn cleanup_segments(&mut self, grid: &mut CachedGrid) {
        for segment_id in &self.created_segments {
            grid.segments.remove(segment_id);
        }
        self.created_segments.clear();
        self.stretch_segments.clear();
    }
}
