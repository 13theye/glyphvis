// src/draw/mod.rs
// The path drawing module
// The cache system for caching Nannou draw commands

//pub mod cache;
//pub mod grid_draw;
//pub use grid_draw::RenderableSegment;

//pub mod path_draw;

//use nannou::prelude::*;

// moved to models/geometry.rs
/*
#[derive(Debug, Clone)]
pub struct Transform2D {
    pub translation: Vec2,
    pub scale: f32,
    pub rotation: f32,
}

impl Default for Transform2D {
    fn default() -> Self {
        Self {
            translation: Vec2::ZERO,
            scale: 1.0,
            rotation: 0.0,
        }
    }
}
     

// moved to views/cached_grid.rs as DrawStyle
#[derive(Debug, Clone)]
pub struct DrawParams {
    pub color: Rgb<f32>,
    pub stroke_weight: f32,
}

impl Default for DrawParams {
    fn default() -> Self {
        Self {
            color: rgb(0.1, 0.1, 0.1),
            stroke_weight: 5.0,
        }
    }
}

*/