// src/views/grid_manager.rs

use nannou::prelude::*;
use std::collections::{ HashMap, HashSet };

use crate::models::Project;
use crate::views::{ Transform2D, CachedGrid, RenderableSegment, DrawStyle };
use crate::effects::{init_effects, EffectsManager};

pub struct GridInstance {
    pub id: String,
    pub grid: CachedGrid,
    
    pub effects_manager: EffectsManager,
    pub transform: Transform2D,
    pub visible: bool,
}

impl GridInstance {
    pub fn new(app: &App, project: &Project, id: String, position: Point2, rotation: f32) -> Self {
        let mut grid = CachedGrid::new(project);
        let transform = Transform2D {
            translation: position,
            scale: 1.0,
            rotation,
        };
        grid.apply_transform(&transform);

        Self {
            id,
            grid,

            effects_manager: init_effects::init_effects(app),
            transform,
            visible: true,
        }
    }

    pub fn draw_segments(&self, draw: &Draw, segments: Vec<RenderableSegment>) {
        self.grid.draw_segments(draw, segments);
    }
    






    fn print_grid_info(&self, grid: &CachedGrid) {
        println!("<====== Grid Instance {} ======>", self.id);
        println!("\nGrid Info:");
        println!("Position: {:?}", self.transform.translation);
        println!("Dimensions: {:?}", grid.dimensions);
        println!("Viewbox: {:?}", grid.viewbox);
        println!("Segment count: {}", grid.segments.len());
        
        // Print first few segments for inspection
        
        for (i, (id, segment)) in grid.segments.iter().take(2).enumerate() {
            println!("\nSegment {}: {}", i, id);
            println!("Position: {:?}", segment.tile_pos);
            println!("Edge type: {:?}", segment.edge_type);
            
            for (j, cmd) in segment.draw_commands.iter().take(2).enumerate() {
                println!("  Command {}: {:?}", j, cmd);
            }
        }
    }

}