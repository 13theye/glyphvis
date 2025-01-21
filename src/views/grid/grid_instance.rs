// src/views/grid_manager.rs

use nannou::prelude::*;
//use std::collections::{ HashMap, HashSet };

use crate::models::Project;
use crate::views::{ Transform2D, CachedGrid, RenderableSegment };
use crate::effects::{init_effects, EffectsManager};

pub struct GridInstance {
    pub id: String,
    pub grid: CachedGrid,
    
    pub effects_manager: EffectsManager,
    pub location: Transform2D,
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
            location: transform,
            visible: true,
        }
    }

    pub fn apply_transform(&mut self, transform: &Transform2D) {
        self.location = self.location.combine(transform);
        self.grid.apply_transform(transform);
    }

    pub fn rotate_in_place(&mut self, angle: f32) {
        // 1. Transform to pivot-relative space
        let to_local = Transform2D {
            translation: -self.location.translation,
            scale: 1.0,
            rotation: 0.0,
        };

        // 2. Just rotation
        let rotate = Transform2D {
            translation: Vec2::ZERO,
            scale: 1.0,
            rotation: angle,
        };

        // 3. Transform back
        let to_world = Transform2D {
            translation: self.location.translation,
            scale: 1.0,
            rotation: 0.0,
        };

        // Apply each transform in sequence
        self.grid.apply_transform(&to_local);
        self.grid.apply_transform(&rotate);
        self.grid.apply_transform(&to_world);
        
        // Update location's rotation (but not position)
        self.location.rotation += angle;
    }


    pub fn draw_segments(&self, draw: &Draw, segments: Vec<RenderableSegment>) {
        self.grid.draw_segments(draw, &segments);
    }

    pub fn activate_segment_effect(&mut self, segment_id: &str, effect_name: &str, time: f32) {
        self.effects_manager.activate_segment(segment_id, effect_name, time);
    }
    
    pub fn print_grid_info(&self) {
        println!("<====== Grid Instance: {} ======>", self.id);
        println!("\nGrid Info:");
        println!("Location: {:?}", self.location.translation);
        println!("Dimensions: {:?}", self.grid.dimensions);
        println!("Viewbox: {:?}", self.grid.viewbox);
        println!("Segment count: {}\n", self.grid.segments.len());
        
        // Print first few segments for inspection
        /*
        for (i, (id, segment)) in self.grid.segments.iter().take(2).enumerate() {
            println!("\nSegment {}: {}", i, id);
            println!("Position: {:?}", segment.tile_pos);
            println!("Edge type: {:?}", segment.edge_type);
            
            for (j, cmd) in segment.draw_commands.iter().take(2).enumerate() {
                println!("  Command {}: {:?}", j, cmd);
            }
             
        }*/
    }

}