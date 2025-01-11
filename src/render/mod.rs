// src/renderer/mod.rs
// The path rendering module
// The cache system for caching Nannou draw commands

//pub mod cache;
pub mod path_renderer;
pub mod glyph_display;

pub use path_renderer::PathRenderer;
pub use glyph_display::GlyphDisplay;

use nannou::prelude::*;

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

#[derive(Debug)]
pub struct RenderParams {
    pub color: Rgb<f32>,
    pub stroke_weight: f32,
}

impl Default for RenderParams {
    fn default() -> Self {
        Self {
            color: rgb(0.1, 0.1, 0.1),
            stroke_weight: 5.0,
        }
    }
}