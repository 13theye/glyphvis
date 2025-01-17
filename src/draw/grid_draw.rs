// src/draw/grid_draw.rs
// Applies 2D transformations to a group of PathElements and calls path_draw to draw them

use nannou::prelude::*;

use crate::models::GridModel;
use crate::views::Transform2D;
use crate::draw::{ DrawParams, path_draw };
use crate::models::GridElement;

pub struct RenderableSegment<'a>{
    pub element: &'a GridElement,
    pub params: DrawParams,
}

/// Draws a collection of segments with the specified grid-level transform
pub fn draw_segments(
    draw: &Draw,
    grid: &GridModel,
    transform: &Transform2D,
    segments: &Vec<RenderableSegment>,
) {
    let tile_size = transform.scale * grid.viewbox.width;

    // Draw all segments
    for segment in segments {
        let (x, y) = segment.element.position;
        
        // Calculate position for this tile
        let x_idx = x - 1;
        let y_idx = grid.height - y; // Invert y to match SVG coordinates
        
        let pos_x = transform.translation.x + 
                    (x_idx as f32 * tile_size) + 
                    (tile_size / 2.0);
        let pos_y = transform.translation.y + 
                    (y_idx as f32 * tile_size) + 
                    (tile_size / 2.0);

        let tile_transform = Transform2D {
            translation: Vec2::new(pos_x, pos_y),
            scale: transform.scale,
            rotation: transform.rotation,
        };

        /*
        // Draw tile boundary for debug
        draw.rect()
            .x_y(pos_x, pos_y)
            .w_h(tile_size, tile_size)
            .stroke(RED)
            .stroke_weight(5.0)
            .no_fill();
        */

        path_draw::draw_element(
            draw,
            &segment.element.path,
            &tile_transform,
            &segment.params,
            &grid.viewbox,
        );
    }
}

/// Calculate the screen position for a grid coordinate
pub fn grid_to_screen(
    grid_pos: (u32, u32),
    grid: &GridModel,
    transform: &Transform2D,
) -> Vec2 {
    let tile_size = transform.scale * grid.viewbox.width;
    let (x, y) = grid_pos;
    
    let x_idx = x - 1;
    let y_idx = grid.height - y;
    
    let pos_x = transform.translation.x + 
               (x_idx as f32 * tile_size) + 
               (tile_size / 2.0);
    let pos_y = transform.translation.y + 
               (y_idx as f32 * tile_size) + 
               (tile_size / 2.0);
               
    Vec2::new(pos_x, pos_y)
}


