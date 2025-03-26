// src/animation/stretch.rs
//
// The grid stretch effect.

use crate::{
    models::{Axis, EdgeType, PathElement},
    services::SegmentGraph,
    views::{
        CachedGrid, CachedSegment, DrawCommand, DrawStyle, GridInstance, SegmentAction,
        SegmentType, StyleUpdateMsg, Transform2D,
    },
};
use nannou::prelude::*;
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone)]
pub struct StretchEffect {
    pub axis: Axis,
    pub current_amount: f32,
    pub target_amount: f32,
    pub start_time: f32,
    pub stretch_segments: HashMap<String, CachedSegment>,
    pub last_applied_amount: f32,
    pub interpolation_speed: f32,
}

impl StretchEffect {
    pub fn new(axis: Axis, start_time: f32) -> Self {
        Self {
            axis,
            current_amount: 0.0,
            target_amount: 0.0,
            start_time,
            stretch_segments: HashMap::new(),
            last_applied_amount: 0.0,
            interpolation_speed: 1.0 / 60.0,
        }
    }

    pub fn set_target_amount(&mut self, amount: f32, time: f32) {
        if (self.target_amount - amount).abs() > 0.001 {
            self.start_time = time;
            self.target_amount = amount;
        }
    }

    pub fn update(&mut self, dt: f32) -> bool {
        let mut changed = false;

        // Calculate how much to move this frame
        let max_delta = self.interpolation_speed * dt;

        // Direction to move
        let direction = if self.target_amount > self.current_amount {
            1.0
        } else {
            -1.0
        };

        // Calculate distance to target
        let distance = (self.target_amount - self.current_amount).abs();

        if distance > 0.001 {
            // Move toward target
            let delta = direction * max_delta.min(distance);
            self.current_amount += delta;
            changed = true;
        }

        changed
    }

    pub fn apply(&mut self, grid_instance: &mut GridInstance) -> HashMap<String, StyleUpdateMsg> {
        let mut style_updates = HashMap::new();

        if (self.current_amount - self.last_applied_amount).abs() < 0.001 {
            return style_updates;
        }

        // Save current amount as laste applied
        self.last_applied_amount = self.current_amount;

        let grid = &mut grid_instance.grid;
        let graph = &grid_instance.graph;

        // 1. Calculate displacements for each column/row
        let displacements = self.calculate_displacements(grid.dimensions);

        // 2. Apply displacements to reposition existing segments;
        self.reposition_grid(grid, &displacements);

        // 3. Find boundary intersections using the graph
        let boundary_intersections = self.find_boundary_intersections(grid, graph);

        // 4. Create/update stretch segments
        self.stretch_segments.clear();

        for (boundary_idx, intersections) in boundary_intersections.iter().enumerate() {
            for point in intersections {
                let segment_id = match self.axis {
                    Axis::X => format!(
                        "stretch-x-{}-{}",
                        boundary_idx + 1,
                        (point.y * 100.0) as u32
                    ),
                    Axis::Y => format!(
                        "stretch-y-{}-{}",
                        (point.x * 100.0) as u32,
                        boundary_idx + 1
                    ),
                };

                if self.current_amount > 0.0 {
                    // Create stretch segment
                    let segment = self.create_stretch_segment(grid, boundary_idx + 1, *point);
                    self.stretch_segments
                        .insert(segment_id.clone(), segment.clone());
                    grid.segments.insert(segment_id.clone(), segment);

                    // Determine style (active or backbone)
                    let is_active = self.is_intersection_active(grid_instance, *point);

                    if is_active {
                        style_updates.insert(
                            segment_id,
                            StyleUpdateMsg {
                                action: Some(SegmentAction::InstantStyleChange),
                                target_style: Some(grid_instance.target_style.clone()),
                            },
                        );
                    } else {
                        style_updates.insert(
                            segment_id,
                            StyleUpdateMsg {
                                action: Some(SegmentAction::BackboneUpdate),
                                target_style: Some(grid_instance.backbone_style.clone()),
                            },
                        );
                    }
                }
            }
        }

        // 5. Remove old stretch segments not in the current set
        let mut to_remove = Vec::new();
        for (id, _) in &grid.segments {
            if id.starts_with("stretch-") && !self.stretch_segments.contains_key(id) {
                to_remove.push(id.clone());
            }
        }

        for id in to_remove {
            grid.segments.remove(&id);
        }

        style_updates
    }

    // reset the grid by removing stretch segments and positioning original segments
    pub fn reset(&mut self, grid_instance: &mut GridInstance) {
        self.current_amount = 0.0;
        self.target_amount = 0.0;
        self.apply(grid_instance);
    }

    fn calculate_displacements(&self, dimensions: (u32, u32)) -> Vec<f32> {
        let mut displacements = vec![0.0];

        let total_boundaries = match self.axis {
            Axis::X => dimensions.0 - 1,
            Axis::Y => dimensions.1 - 1,
        };

        for i in 1..=total_boundaries {
            displacements.push(i as f32 * self.current_amount);
        }

        displacements
    }

    fn reposition_grid(&self, grid: &mut CachedGrid, displacements: &[f32]) {
        todo!();
    }

    fn find_boundary_intersections(
        &self,
        grid: &CachedGrid,
        graph: &SegmentGraph,
    ) -> Vec<Vec<Point2>> {
        todo!();
    }

    fn find_command_x_intersection(
        &self,
        command: &DrawCommand,
        boundary_x: f32,
    ) -> Option<Point2> {
        todo!();
    }

    fn is_intersection_active(&self, grid_instance: &GridInstance, point: Point2) -> bool {
        todo!();
    }

    fn create_stretch_segment(
        &self,
        grid: &CachedGrid,
        boundary_idx: u32,
        point: Point2,
    ) -> CachedSegment {
        todo!();
    }
}
