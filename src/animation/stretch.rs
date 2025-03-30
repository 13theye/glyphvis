// src/animation/stretch.rs

use crate::{
    models::{Axis, EdgeType},
    services::SegmentGraph,
    views::{CachedGrid, CachedSegment, SegmentType},
};
use std::collections::HashSet;

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
