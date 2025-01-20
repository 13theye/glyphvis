// src/views/grid_manager.rs

use nannou::prelude::*;
use std::collections::{ HashMap, HashSet };

use crate::models::Project;
use crate::views::{ Transform2D, CachedGrid, RenderableSegment, DrawStyle };
use crate::effects::{init_effects, EffectsManager};

pub struct GridInstance {
    pub grid: CachedGrid,

    pub active_glyph: Option<String>,
    pub active_segments: HashSet<String>,
    
    pub effects_manager: EffectsManager,
    pub transform: Transform2D,
    pub visible: bool,
}

impl GridInstance {
    pub fn new(app: &App, project: &Project, position: Point2, rotation: f32) -> Self {
        let mut grid = CachedGrid::new(project);
        let transform = Transform2D {
            translation: position,
            scale: 1.0,
            rotation,
        };
        grid.apply_transform(&transform);

        Self {
            grid,
            active_glyph: None,

            active_segments: HashSet::new(),
            effects_manager: init_effects::init_effects(app),

            transform,
            visible: true,
        }
    }
}