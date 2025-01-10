use nannou::prelude::*;
use glyphvis::models::data_model::Project;
use glyphvis::models::grid_model::Grid;
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
                    draw_element(&draw, &element.path, pos_x, pos_y, scale);
                }
            }
        }
    }
    
    draw.to_frame(app, &frame).unwrap();
}

fn draw_element(draw: &Draw, element: &PathElement, pos_x: f32, pos_y: f32, scale: f32) {
    //println!("Drawing element: {:#?}", element);
    match element {
        PathElement::Line { x1, y1, x2, y2 } => {
            let start = pt2(
                pos_x + (x1 - 50.0) * scale, 
                pos_y - (y1 - 50.0) * scale
            );
            let end = pt2(
                pos_x + (x2 - 50.0) * scale, 
                pos_y - (y2 - 50.0) * scale
            );
            
            draw.line()
                .start(start)
                .end(end)
                .color(rgb(0.1, 0.1, 0.1))
                .stroke_weight(4.0);
        },
        PathElement::Circle { cx, cy, r } => {
            let center = pt2(
                pos_x + (cx - 50.0) * scale, 
                pos_y - (cy - 50.0) * scale
            );
            
            draw.ellipse()
                .x_y(center.x, center.y)
                .radius(r * scale)
                .stroke(rgb(0.1, 0.1, 0.1))
                .stroke_weight(4.0)
                .no_fill();
        },
        PathElement::Arc { start_x, start_y, end_x, end_y, .. } => {
            let start = pt2(
                pos_x + (start_x - 50.0) * scale, 
                pos_y - (start_y - 50.0) * scale
            );
            let end = pt2(
                pos_x + (end_x - 50.0) * scale, 
                pos_y - (end_y - 50.0) * scale
            );
            
            draw.line()
                .start(start)
                .end(end)
                .color(rgb(0.1, 0.1, 0.1))
                .stroke_weight(4.0);
        }
    }
}