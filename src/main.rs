use nannou::prelude::*;
use glyphvis::models::data_model::Project;
use glyphvis::models::grid_model::Grid;
use glyphvis::services::grid_service::{PathElement, ViewBox};

struct Model {
    grid: Grid,
    tile_size: f32,
}

fn main() {
    nannou::app(model)
        .update(update)
        .run();
}

fn model(app: &App) -> Model {
    // Create window
    app.new_window().size(800, 800).view(view).build().unwrap();
    
    // Load project
    let project = Project::load("../glyphmaker/projects/small-cir-d.json")
        .expect("Failed to load project file");
    
    // Create grid from project
    let grid = Grid::new(&project);
    println!("Created grid with {} elements", grid.elements.len());
    
    // Calculate tile size based on window dimensions
    let window = app.window_rect();
    let max_tile_size = f32::min(
        window.w() / grid.width as f32,
        window.h() / grid.height as f32
    ) * 0.95; // 95% of available space
    
    Model {
        grid,
        tile_size: max_tile_size,
    }
}

fn update(_app: &App, _model: &mut Model, _update: Update) {
}

fn view(app: &App, model: &Model, frame: Frame) {
    let draw = app.draw();
    draw.background().color(BLACK);
    
    // Calculate grid layout
    let grid_width = model.tile_size * model.grid.width as f32;
    let grid_height = model.tile_size * model.grid.height as f32;
    let offset_x = -grid_width / 2.0;
    let offset_y = -grid_height / 2.0;
    
    // Draw grid elements
    for y in 1..=model.grid.height {
        for x in 1..=model.grid.width {
            // offset accounts for grid starting at 1, not 0
            let pos_x = offset_x + ((x - 1) as f32 * model.tile_size) + (model.tile_size / 2.0);
            let pos_y = offset_y + ((y - 1) as f32 * model.tile_size) + (model.tile_size / 2.0);
            
            /* 
            // Draw tile boundary for debugging
            draw.rect()
                .x_y(pos_x, pos_y)
                .w_h(model.tile_size, model.tile_size)
                .stroke(RED)
                .stroke_weight(4.0)
                .no_fill();
            */
            
            // Draw all elements at this grid position
            let elements = model.grid.get_elements_at(x, y);
            let scale = model.tile_size / model.grid.viewbox.width;
            //println!("Drawing elements: {:#?}", elements);
            
            for element in elements {
                // Only draw if the element should be visible
                if model.grid.should_draw_element(element) {
                    //println!("Drawing element {} at position ({}, {})", element.id, x, y);
                    draw_element(&draw, &element.path, pos_x, pos_y, scale, &model.grid.viewbox);
                }
            }
        }
    }
    
    draw.to_frame(app, &frame).unwrap();
}

// transforms path instructions from SVG to Nannou draw instructions.
// SVG instructions have origin at top left, Nannou at center.
fn draw_element(draw: &Draw, element: &PathElement, pos_x: f32, pos_y: f32, scale: f32, viewbox: &ViewBox) {
    let center_x = viewbox.width / 2.0;
    let center_y = viewbox.height / 2.0;
    
    match element {
        PathElement::Line { x1, y1, x2, y2 } => {
            let start = pt2(
                pos_x + (x1 - center_x) * scale, 
                pos_y + (y1 - center_y) * scale 
            );
            let end = pt2(
                pos_x + (x2 - center_x) * scale, 
                pos_y + (y2 - center_y) * scale  
            );
            
            draw.line()
                .start(start)
                .end(end)
                .color(rgb(0.1, 0.1, 0.1))
                .stroke_weight(4.0);
        },
        
        PathElement::Circle { cx, cy, r } => {
            let center = pt2(
                pos_x + (cx - center_x) * scale, 
                pos_y + (cy - center_y) * scale  
            );
            
            draw.ellipse()
                .x_y(center.x, center.y)
                .radius(r * scale)
                .stroke(rgb(0.1, 0.1, 0.1))
                .stroke_weight(4.0)
                .no_fill();
        },

        PathElement::Arc { start_x, start_y, rx, ry: _, sweep, end_x, end_y, .. } => {
            // For these specific 90-degree corner arcs, we can simplify the calculation
            // The center will always be at the corner point
            let (center_point, start_angle, end_angle) = match (start_x, start_y, end_x, end_y) {
                // arc-1: top-left quarter (50,0 -> 0,50)
                (50.0, 0.0, 0.0, 50.0) => {
                    (pt2(0.0, 0.0), 0.0, PI/2.0)
                },
                // arc-2: top-right quarter (50,0 -> 100,50)
                (50.0, 0.0, 100.0, 50.0) => {
                    (pt2(100.0, 0.0), PI/2.0, PI)
                },
                // arc-3: bottom-left quarter (0,50 -> 50,100)
                (0.0, 50.0, 50.0, 100.0) => {
                    (pt2(0.0, 100.0), -PI/2.0, 0.0)
                },
                // arc-4: bottom-right quarter (100,50 -> 50,100)
                (100.0, 50.0, 50.0, 100.0) => {
                    (pt2(100.0, 100.0), PI, -PI/2.0)
                },
                _ => (pt2(0.0, 0.0), 0.0, 0.0), // shouldn't happen
            };

            // Convert center to screen coordinates
            let screen_center = pt2(
                pos_x + (center_point.x - center_x) * scale,
                pos_y + (center_point.y - center_y) * scale
            );

            // Generate points for the arc
            let resolution = 32;
            let points: Vec<Point2> = (0..=resolution)
                .map(|i| {
                    let t = i as f32 / resolution as f32;
                    let angle = if *sweep {
                        start_angle + t * (end_angle - start_angle)
                    } else {
                        end_angle + t * (start_angle - end_angle)
                    };
                    
                    pt2(
                        screen_center.x + rx * angle.cos() * scale,
                        screen_center.y + rx * angle.sin() * scale
                    )
                })
                .collect();

            // Build path with individual line segments
            if let Some(first) = points.first() {
                let mut builder = nannou::geom::Path::builder()
                    .move_to(nannou::geom::Point2::new(first.x, first.y));
                
                // Add line segments to approximate arc
                for point in points.iter().skip(1) {
                    builder = builder.line_to(nannou::geom::Point2::new(point.x, point.y));
                }

                // Build the path
                let path: nannou::geom::Path = builder.build();
                
                draw.path()
                    .stroke()
                    .weight(4.0)
                    .color(rgb(0.1, 0.1, 0.1))
                    .events(path.iter());
            }
        }
    }
}

use std::f32::consts::PI;