// src/animation/stretch.rs

use crate::{
    models::EdgeType,
    views::{CachedGrid, CachedSegment, SegmentType},
};

pub enum Axis {
    X,
    Y,
}

pub fn boundary_segments(grid: &CachedGrid, axis: Axis) -> Vec<&CachedSegment> {
    let mut boundary_segments = Vec::new();
    for segment in grid.segments.values() {
        match axis {
            Axis::X => {
                if segment.segment_type == SegmentType::Vertical
                    && (segment.edge_type == EdgeType::East || segment.edge_type == EdgeType::West)
                {
                    boundary_segments.push(segment)
                }
            }
            Axis::Y => {
                if segment.segment_type == SegmentType::Horizontal
                    && (segment.edge_type == EdgeType::North
                        || segment.edge_type == EdgeType::South)
                {
                    boundary_segments.push(segment)
                }
            }
        }
    }
    boundary_segments
}
