// src/render/renderer.rs
// the renderer applies 2D transformations to a group of PathElements and calls PathRenderer to draw them

use nannou::prelude::*;

use crate::models::grid_model::{ Grid, ViewBox };
use crate::render::{Transform2D, RenderParams, PathRenderer};
use crate::services::path_service::GridElement;

pub struct Renderer {
    path_renderer: PathRenderer,
}
pub struct RenderableSegment<'a>{
    pub element: &'a GridElement,
    pub params: RenderParams,
}

impl Renderer {
    pub fn new(viewbox: ViewBox) -> Self {
        Self {
            path_renderer: PathRenderer::new(viewbox),
        }
    }

    /// Draws a collection of segments with the specified grid transform
    pub fn draw(
        &self,
        draw: &Draw,
        grid: &Grid,
        transform: &Transform2D,
        segments: Vec<RenderableSegment>,
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

            self.path_renderer.draw_element(
                draw,
                &segment.element.path,
                &tile_transform,
                &segment.params
            );
        }
    }

}
