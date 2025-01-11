use nannou::prelude::*;
use glyphvis::models::data_model::Project;
use glyphvis::models::grid_model:: { Grid, ViewBox };
use glyphvis::services::grid_service::PathElement;

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
    let project = Project::load("../glyphmaker/projects/small-cir-d2.json")
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
            // Draw tile boundary for debugging.
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
    let color = rgb(0.1, 0.1, 0.1);
    let weight = 4.0;
    
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
                .color(color)
                .stroke_weight(weight);
        },
        
        PathElement::Circle { cx, cy, r } => {
            let center = pt2(
                pos_x + (cx - center_x) * scale, 
                pos_y + (cy - center_y) * scale  
            );
            
            draw.ellipse()
                .x_y(center.x, center.y)
                .radius(r * scale)
                .stroke(color)
                .stroke_weight(weight)
                .color(color);
        },
        
        PathElement::Arc { start_x, start_y, rx, ry, x_axis_rotation, large_arc, sweep, end_x, end_y } => {
            println!("\nArc path at grid position ({}, {})", pos_x, pos_y);
            println!("Input params:");
            println!("  Start: ({}, {}), End: ({}, {})", start_x, start_y, end_x, end_y);
            println!("  rx: {}, ry: {}, rotation: {}", rx, ry, x_axis_rotation);
            println!("  large_arc: {}, sweep: {}", large_arc, sweep);

            // Convert coordinates to screen space first
            let screen_start = pt2(
                pos_x + (start_x - center_x) * scale,
                pos_y + (start_y - center_y) * scale
            );
            let screen_end = pt2(
                pos_x + (end_x - center_x) * scale,
                pos_y + (end_y - center_y) * scale
            );

            println!("Screen coordinates:");
            println!("  Start: ({:.2}, {:.2})", screen_start.x, screen_start.y);
            println!("  End: ({:.2}, {:.2})", screen_end.x, screen_end.y);

            // SVG to center parameterization conversion
            // Step 1: Transform to origin
            let x1p = (screen_start.x - screen_end.x) / 2.0;
            let y1p = (screen_start.y - screen_end.y) / 2.0;

            println!("Step 1 - Transform to origin:");
            println!("  x1p: {:.2}, y1p: {:.2}", x1p, y1p);

            // Rotate to align with coordinate axes
            let angle_rad = x_axis_rotation.to_radians();
            let cos_angle = angle_rad.cos();
            let sin_angle = angle_rad.sin();

            let xp = cos_angle * x1p + sin_angle * y1p;
            let yp = -sin_angle * x1p + cos_angle * y1p;

            println!("After rotation:");
            println!("  xp: {:.2}, yp: {:.2}", xp, yp);

            // Step 2: Compute center
            let rx_sq = rx * rx * scale * scale;
            let ry_sq = ry * ry * scale * scale;
            let xp_sq = xp * xp;
            let yp_sq = yp * yp;

            // Ensure radii are large enough
            let radii_scale = xp_sq / rx_sq + yp_sq / ry_sq;
            let (rx_scaled, ry_scaled) = if radii_scale > 1.0 {
                let sqrt_scale = radii_scale.sqrt();
                println!("Scaling up radii by factor: {:.2}", sqrt_scale);
                (rx * scale * sqrt_scale, ry * scale * sqrt_scale)
            } else {
                (rx * scale, ry * scale)
            };

            println!("Scaled radii:");
            println!("  rx: {:.2}, ry: {:.2}", rx_scaled, ry_scaled);

            let rx_sq = rx_scaled * rx_scaled;
            let ry_sq = ry_scaled * ry_scaled;

            let term = (rx_sq * ry_sq - rx_sq * yp_sq - ry_sq * xp_sq) / 
                      (rx_sq * yp_sq + ry_sq * xp_sq);
            
            println!("Center calculation:");
            println!("  Term under sqrt: {:.2}", term);

            // Guard against numerical errors that might make term slightly negative
            let s = if term <= 0.0 { 
                println!("  Warning: Non-positive term, using s = 0");
                0.0 
            } else { 
                term.sqrt() 
            };
            let cp = if *large_arc == *sweep {-s} else {s};

            let cxp = cp * rx_scaled * yp / ry_scaled;
            let cyp = -cp * ry_scaled * xp / rx_scaled;

            // Step 3: Transform back
            let center_x = cos_angle * cxp - sin_angle * cyp + (screen_start.x + screen_end.x) / 2.0;
            let center_y = sin_angle * cxp + cos_angle * cyp + (screen_start.y + screen_end.y) / 2.0;

            println!("Calculated center: ({:.2}, {:.2})", center_x, center_y);

            // Calculate angles
            let start_angle = ((yp - cyp) / ry_scaled).atan2((xp - cxp) / rx_scaled);
            let end_angle = ((-yp - cyp) / ry_scaled).atan2((-xp - cxp) / rx_scaled);
            
            println!("Angles:");
            println!("  Start: {:.2}, End: {:.2}", start_angle, end_angle);
            
            // Generate points
            let resolution = 64;
            let mut points = Vec::with_capacity(resolution + 1);
            
            // Calculate total angle sweep
            let mut delta_angle = end_angle - start_angle;
            
            // Ensure we're sweeping in the correct direction
            if *sweep {
                if delta_angle < 0.0 {
                    delta_angle += 2.0 * PI;
                }
            } else {
                if delta_angle > 0.0 {
                    delta_angle -= 2.0 * PI;
                }
            }

            for i in 0..=resolution {
                let t = i as f32 / resolution as f32;
                let angle = start_angle + t * delta_angle;

                let x = center_x + rx_scaled * (cos_angle * angle.cos() - sin_angle * angle.sin());
                let y = center_y + ry_scaled * (sin_angle * angle.cos() + cos_angle * angle.sin());

                points.push(pt2(x, y));
            }

            println!("Generated {} points", points.len());
            if let Some(first) = points.first() {
                println!("First point: ({:.2}, {:.2})", first.x, first.y);
            }
            if let Some(last) = points.last() {
                println!("Last point: ({:.2}, {:.2})", last.x, last.y);
            }

            // Build and draw the path
            if let Some(first) = points.first() {
                let mut builder = nannou::geom::Path::builder()
                    .move_to(nannou::geom::Point2::new(first.x, first.y));
                
                for point in points.iter().skip(1) {
                    builder = builder.line_to(nannou::geom::Point2::new(point.x, point.y));
                }

                let path = builder.build();
                draw.path()
                    .stroke()
                    .weight(weight)
                    .color(color)
                    .events(path.iter());
            }
        }
    }
}

use std::f32::consts::PI;