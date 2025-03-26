// src/utilities/segment_utility.rs

// Utility functions for initializing CachedSegments

use crate::{
    models::{PathElement, ViewBox},
    utilities::grid_init,
    views::grid::grid_generic::ARC_RESOLUTION,
    views::{DrawCommand, Transform2D},
};
use nannou::prelude::*;

pub fn generate_draw_commands(
    path: &PathElement,
    viewbox: &ViewBox,
    transform: &Transform2D,
) -> Vec<DrawCommand> {
    match path {
        PathElement::Line { x1, y1, x2, y2 } => {
            vec![DrawCommand::Line {
                start: initial_transform(*x1, *y1, viewbox, transform),
                end: initial_transform(*x2, *y2, viewbox, transform),
            }]
        }
        PathElement::Arc {
            start_x,
            start_y,
            rx,
            ry,
            x_axis_rotation,
            large_arc,
            sweep,
            end_x,
            end_y,
        } => {
            let start = initial_transform(*start_x, *start_y, viewbox, transform);
            let end = initial_transform(*end_x, *end_y, viewbox, transform);

            // no need to translate b/c rx, ry are relative measures
            let (center, start_angle, sweep_angle) = grid_init::calculate_arc_center(
                start,
                end,
                *rx,
                *ry,
                *x_axis_rotation,
                *large_arc,
                *sweep,
            );

            // Calculate all points, scale radii
            let points = grid_init::generate_arc_points(
                center,
                *rx * transform.scale,
                *ry * transform.scale,
                start_angle,
                sweep_angle,
                *x_axis_rotation,
                ARC_RESOLUTION,
            );

            vec![DrawCommand::Arc { points }]
        }
        PathElement::Circle { cx, cy, r } => {
            vec![DrawCommand::Circle {
                center: initial_transform(*cx, *cy, viewbox, transform),
                radius: *r * transform.scale,
            }]
        }
    }
}

// Translates a point to the correct Tile position
pub fn calculate_tile_transform(
    viewbox: &ViewBox,
    position: (u32, u32),
    grid_dims: (u32, u32),
) -> Transform2D {
    let (x, y) = position;
    let (grid_x, grid_y) = grid_dims;
    let tile_width = viewbox.width;
    let tile_height = viewbox.height;

    let grid_width = grid_x as f32 * tile_width;
    let grid_height = grid_y as f32 * tile_height;

    let tile_center_x = (x as f32 - 1.0) * tile_width - grid_width / 2.0 + tile_width / 2.0;
    let tile_center_y = -((y as f32 - 1.0) * tile_height) + grid_height / 2.0 - tile_height / 2.0;

    Transform2D {
        translation: pt2(tile_center_x, tile_center_y),
        scale: 1.0,
        rotation: 0.0,
    }
}

// Transform a point from SVG to Nannou Coordinates, then applies tile transform
fn initial_transform(svg_x: f32, svg_y: f32, viewbox: &ViewBox, transform: &Transform2D) -> Point2 {
    let center_x = viewbox.width / 2.0;
    let center_y = viewbox.height / 2.0;
    let local_x = svg_x - center_x;
    let local_y = center_y - svg_y;
    // return:
    transform.apply_to_point(pt2(local_x, local_y))
}
