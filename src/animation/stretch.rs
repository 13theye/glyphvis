// src/animation/stretch.rs
//
// The grid stretch effect.

use crate::{
    models::{Axis, EdgeType, PathElement},
    services::SegmentGraph,
    utilities::segment_analysis,
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
    pub boundary_intersections: Vec<Vec<(Point2, Vec<String>)>>, // Cache boundary intersections
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
            boundary_intersections: Vec::new(),
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

        // Save current amount as last applied
        self.last_applied_amount = self.current_amount;

        // 1. Calculate displacements for each column/row
        let displacements = self.calculate_displacements(grid_instance.grid.dimensions);

        // 2. Update or find boundary intersections if needed
        if self.boundary_intersections.is_empty() {
            // Only find intersections once - they stay the same throughout animation
            self.find_boundary_intersections(&grid_instance.grid, &grid_instance.graph);
        }

        // 3. Create/update stretch segments
        self.stretch_segments.clear();

        for (boundary_idx, intersections) in self.boundary_intersections.iter().enumerate() {
            for (point, connected_segments) in intersections {
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
                    let segment =
                        self.create_stretch_segment(&grid_instance.grid, boundary_idx + 1, *point);
                    self.stretch_segments
                        .insert(segment_id.clone(), segment.clone());
                    grid_instance
                        .grid
                        .segments
                        .insert(segment_id.clone(), segment);

                    // Determine style (active or backbone)
                    let is_active = self.is_intersection_active(grid_instance, connected_segments);

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

        // 4. Apply transform at the GridInstance level
        // Generate a transform for each column/row based on displacements
        let transforms = self.generate_transforms(displacements);
        self.apply_transforms_to_grid_instance(grid_instance, &transforms);

        // 5. Remove old stretch segments not in the current set
        let mut to_remove = Vec::new();
        for (id, _) in &grid_instance.grid.segments {
            if id.starts_with("stretch-") && !self.stretch_segments.contains_key(id) {
                to_remove.push(id.clone());
            }
        }

        for id in to_remove {
            grid_instance.grid.segments.remove(&id);
        }

        style_updates
    }

    // Reset the grid by removing stretch segments and repositioning original segments
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

    // Generate Transform2D objects for each column/row
    fn generate_transforms(&self, displacements: Vec<f32>) -> HashMap<u32, Transform2D> {
        let mut transforms = HashMap::new();

        for (idx, displacement) in displacements.iter().enumerate() {
            let position = idx as u32;

            // Create transform with appropriate translation
            let transform = match self.axis {
                Axis::X => Transform2D {
                    translation: Vec2::new(*displacement, 0.0),
                    scale: 1.0,
                    rotation: 0.0,
                },
                Axis::Y => Transform2D {
                    translation: Vec2::new(0.0, *displacement),
                    scale: 1.0,
                    rotation: 0.0,
                },
            };

            transforms.insert(position, transform);
        }

        transforms
    }

    // Apply transforms to segments based on their position
    fn apply_transforms_to_grid_instance(
        &self,
        grid_instance: &mut GridInstance,
        transforms: &HashMap<u32, Transform2D>,
    ) {
        // Reset all segments to base positions first
        grid_instance.reset_segment_positions();

        // Apply transforms to segments based on their position
        for segment in grid_instance.grid.segments.values_mut() {
            // Skip stretch segments (they'll be positioned correctly when created)
            if segment.id.starts_with("stretch-") {
                continue;
            }

            let (col, row) = segment.tile_coordinate;

            let transform_idx = match self.axis {
                Axis::X => col,
                Axis::Y => row,
            };

            if let Some(transform) = transforms.get(&transform_idx) {
                segment.apply_transform(transform);
            }
        }
    }

    fn find_boundary_intersections(&mut self, grid: &CachedGrid, graph: &SegmentGraph) {
        let mut all_intersections = Vec::new();

        match self.axis {
            Axis::X => {
                // For each vertical boundary
                for boundary_x in 1..grid.dimensions.0 {
                    // Use the segment_analysis utility function
                    let boundary_intersections =
                        segment_analysis::find_x_boundary_intersections(grid, graph, boundary_x);
                    all_intersections.push(boundary_intersections);
                }
            }
            Axis::Y => {
                // For each horizontal boundary
                for boundary_y in 1..grid.dimensions.1 {
                    // Use the segment_analysis utility function
                    let boundary_intersections =
                        segment_analysis::find_y_boundary_intersections(grid, graph, boundary_y);
                    all_intersections.push(boundary_intersections);
                }
            }
        }

        self.boundary_intersections = all_intersections;
    }

    // Check if any of the connected segments are active or targeted
    fn is_intersection_active(
        &self,
        grid_instance: &GridInstance,
        connected_segments: &[String],
    ) -> bool {
        for segment_id in connected_segments {
            if grid_instance.current_active_segments.contains(segment_id) {
                return true;
            }

            if let Some(target_segments) = &grid_instance.target_segments {
                if target_segments.contains(segment_id) {
                    return true;
                }
            }
        }

        false
    }

    fn create_stretch_segment(
        &self,
        grid: &CachedGrid,
        boundary_idx: u32,
        point: Point2,
    ) -> CachedSegment {
        match self.axis {
            Axis::X => {
                // Calculate position based on boundary
                let tile_x = boundary_idx;
                let tile_y =
                    ((point.y / grid.viewbox.height) * grid.dimensions.1 as f32).ceil() as u32;

                let path = PathElement::Line {
                    x1: point.x,
                    y1: point.y,
                    x2: point.x + self.current_amount,
                    y2: point.y,
                };

                CachedSegment::new(
                    format!("stretch-x-{}-{}", boundary_idx, (point.y * 100.0) as u32),
                    (tile_x, tile_y),
                    &path,
                    EdgeType::None,
                    &grid.viewbox,
                    grid.dimensions,
                )
            }
            Axis::Y => {
                // Calculate position based on boundary
                let tile_x =
                    ((point.x / grid.viewbox.width) * grid.dimensions.0 as f32).ceil() as u32;
                let tile_y = boundary_idx;

                let path = PathElement::Line {
                    x1: point.x,
                    y1: point.y,
                    x2: point.x,
                    y2: point.y + self.current_amount,
                };

                CachedSegment::new(
                    format!("stretch-y-{}-{}", (point.x * 100.0) as u32, boundary_idx),
                    (tile_x, tile_y),
                    &path,
                    EdgeType::None,
                    &grid.viewbox,
                    grid.dimensions,
                )
            }
        }
    }
}
