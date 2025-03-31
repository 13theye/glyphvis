// src/animation/stretch.rs

use crate::{
    models::{Axis, EdgeType, PathElement, ViewBox},
    services::SegmentGraph,
    views::{CachedGrid, CachedSegment, SegmentType},
};
use nannou::prelude::*;
use std::collections::HashSet;

pub struct StretchAnimation {
    pub segment_ids: HashSet<String>,
    pub axis: Axis,
    pub current_amount: f32,
    pub target_amount: f32,
    pub start_time: f32,
    pub duration: f32,
}

impl StretchAnimation {
    pub fn new(
        grid: &mut CachedGrid,
        current_grid_position: &Point2,
        graph: &SegmentGraph,
        axis: Axis,
        target_amount: f32,
        start_time: f32,
    ) -> Self {
        // the points where stretch_segments should be placed
        let mut stretch_points = Vec::new();

        // the boundaries between tiles in the grid
        let mut boundary_segments = boundary_segments(grid, axis);

        // the outer boundaries of the grid are excluded
        boundary_segments.retain(|id| !is_outer_boundary(grid, grid.segment(id).unwrap()));

        // grid segments that intersect the boundary segments.
        // whether or not these are active will determine the style of the
        // stretch segments
        let mut neighbors = HashSet::new();

        // set which type of neighbor we are looking for
        let neighbor_segment_type = match axis {
            Axis::X => SegmentType::Horizontal,
            Axis::Y => SegmentType::Vertical,
        };

        // iter through the boundary segments and gather the neighbors and intersection points
        for segment in &boundary_segments {
            graph
                .neighbors_of(segment)
                .iter()
                .filter_map(|id| grid.segment(id))
                .filter(|s| s.segment_type == neighbor_segment_type)
                .for_each(|s| {
                    neighbors.insert(s.id.clone());
                    stretch_points.push(graph.get_connection_point(segment, &s.id).unwrap());
                });
        }

        let mut segment_ids = HashSet::new();

        for point in stretch_points {
            let segment = generate_stretch_segment(point, current_grid_position, axis);
            segment_ids.insert(segment.id.clone());
            grid.add_stretch_segment(segment);
        }

        Self {
            segment_ids,
            axis,
            current_amount: 0.0,
            target_amount,
            start_time,
            duration: 1.0 / 60.0,
        }
    }

    pub fn is_finished(&self) -> bool {
        (self.target_amount - self.current_amount).abs() < 0.001
    }
}

fn generate_stretch_segment(
    start_point: &Point2,
    current_grid_position: &Point2,
    axis: Axis,
) -> CachedSegment {
    let axis_label = match axis {
        Axis::X => 'x',
        Axis::Y => 'y',
    };

    let x1 = match axis {
        Axis::X => start_point.x + current_grid_position.x,
        Axis::Y => start_point.x,
    };
    let y1 = match axis {
        Axis::X => start_point.y,
        Axis::Y => start_point.y + current_grid_position.y,
    };

    CachedSegment::new(
        format!("stretch-{}-{:?}", axis_label, current_grid_position),
        (0, 0), // unused for stretch segment
        &PathElement::Line {
            x1,
            x2: x1, // zero length
            y1,
            y2: y1, // zero length
        },
        EdgeType::None,
        &ViewBox {
            // unused for stretch segment
            min_x: 0.0,
            min_y: 0.0,
            height: 0.0,
            width: 0.0,
        },
        (0, 0), // unused for stretch segment
    )
}

pub fn boundary_segments(grid: &CachedGrid, axis: Axis) -> HashSet<String> {
    let mut boundary_segments = HashSet::new();
    for segment in grid.segments.values() {
        match axis {
            Axis::X => {
                if segment.segment_type == SegmentType::Vertical
                    && (segment.edge_type == EdgeType::East || segment.edge_type == EdgeType::West)
                {
                    boundary_segments.insert(segment.id.clone());
                }
            }
            Axis::Y => {
                if segment.segment_type == SegmentType::Horizontal
                    && (segment.edge_type == EdgeType::North
                        || segment.edge_type == EdgeType::South)
                {
                    boundary_segments.insert(segment.id.clone());
                }
            }
        }
    }
    boundary_segments
}

pub fn is_outer_boundary(grid: &CachedGrid, segment: &CachedSegment) -> bool {
    match segment.edge_type {
        EdgeType::North => segment.tile_coordinate.1 == 1,
        EdgeType::South => segment.tile_coordinate.1 == grid.dimensions.1,
        EdgeType::East => segment.tile_coordinate.0 == grid.dimensions.0,
        EdgeType::West => segment.tile_coordinate.0 == 1,
        _ => false,
    }
}
