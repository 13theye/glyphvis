// src/views/mod.rs

pub mod background;
pub mod grid;

pub use background::BackgroundManager;
pub use grid::grid_generic::{
    CachedGrid, CachedSegment, DrawCommand, DrawStyle, Layer, SegmentAction, SegmentState,
    SegmentType, StyleUpdateMsg,
};
pub use grid::grid_instance::GridInstance;
pub use grid::transform::Transform2D;
