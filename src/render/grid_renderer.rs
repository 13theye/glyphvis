use nannou::prelude::*;

use crate::models::grid_model::{ Grid, ViewBox };
use crate::render::{Transform2D, RenderParams, PathRenderer};
use crate::services::path_service::GridElement;

pub struct GridRenderer {
    path_renderer: PathRenderer,
}
pub struct RenderableSegment<'a>{
    pub element: &'a GridElement,
    pub params: RenderParams,
}

impl GridRenderer {
    pub fn new(viewbox: ViewBox) -> Self {
        GridRenderer {
            path_renderer: PathRenderer::new(viewbox),
        }
    }

    /// Draws a collection of segments with the specified grid transform
    pub fn draw(
        &self,
        draw: &Draw,
        grid: &Grid,
        grid_transform: &Transform2D,
        segments: Vec<RenderableSegment>,
    ) {
        let tile_size = grid_transform.scale * grid.viewbox.width;

        // Draw all segments
        for segment in segments {
            let (x, y) = segment.element.position;
            
            // Calculate position for this tile
            let x_idx = x - 1;
            let y_idx = grid.height - y; // Invert y to match SVG coordinates
            
            let pos_x = grid_transform.translation.x + 
                       (x_idx as f32 * tile_size) + 
                       (tile_size / 2.0);
            let pos_y = grid_transform.translation.y + 
                       (y_idx as f32 * tile_size) + 
                       (tile_size / 2.0);

            let tile_transform = Transform2D {
                translation: Vec2::new(pos_x, pos_y),
                scale: grid_transform.scale,
                rotation: grid_transform.rotation,
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

    /* 
    pub fn draw(&self,
                draw: &Draw, 
                grid: &Grid, 
                grid_transform: &Transform2D,
                default_params: &RenderParams,
                active_segments: &HashSet<String>,
                active_segments_params: Option<&RenderParams>,
                debug_flag: bool
            ) {
                let debug_color = |x:u32, y:u32| -> f32 {
                    ((x+y) as f32) / (grid.height + grid.width) as f32
                };

        let tile_size = grid_transform.scale * grid.viewbox.width;


        // First draw the background grid
        // Draw grid elements
        for y in 1..=grid.height {
            for x in 1..=grid.width {

                // Calculate position for this tile
                let x_idx = x - 1;
                let y_idx = grid.height - y; // Invert y to match SVG coordinates
                
                let pos_x = grid_transform.translation.x + 
                            (x_idx as f32 * tile_size) + 
                            (tile_size / 2.0);
                let pos_y = grid_transform.translation.y + 
                            (y_idx as f32 * tile_size) + 
                            (tile_size / 2.0);

                let tile_transform = Transform2D {
                    translation: Vec2::new(pos_x, pos_y),
                    scale: grid_transform.scale,
                    rotation: grid_transform.rotation,
                };


                let elements = grid.get_elements_at(x, y);

                for element in &elements {
                    if grid.should_draw_element(element) {
                        let segment_id = format!("{},{} : {}", x, y, element.id);
                        let is_active = active_segments.contains(&segment_id);

                        // Skip active segments if no active params provided
                        if is_active && active_segments_params.is_none() {
                            continue;
                        }

                        // Determine rendering parameters
                        let params = if is_active {
                            active_segments_params.unwrap()  // TODO: use Some?
                        } else {
                            if debug_flag {
                                let b = debug_color(x, y);
                                &RenderParams {
                                    color: rgb(0.05, 0.05, b/3.0),  // Dark gray for inactive grid
                                    stroke_weight: default_params.stroke_weight,
                                }
                            } else {
                                default_params
                            }
                        };

                        /*
                        // Draw tile boundary for debug
                        draw.rect()
                            .x_y(pos_x, pos_y)
                            .w_h(self.tile_size, self.tile_size)
                            .stroke(RED)
                            .stroke_weight(5.0)
                            .no_fill();
                        */

                        self.path_renderer.draw_element(
                            draw,
                            &element.path,
                            &tile_transform,
                            params
                        );
                    }
                }
            }
        }
    }
    */
}
