// src/main.rs
use nannou::prelude::*;

use glyphvis::models::data_model::Project;
use glyphvis::models::grid_model::Grid;
use glyphvis::render::path_renderer::PathRenderer;
use glyphvis::render::{Transform2D, RenderParams};

struct Model {
    grid: Grid,
    tile_size: f32,
    path_renderer: PathRenderer,
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
    
    let path_renderer = PathRenderer::new(grid.viewbox.clone());
    
    Model {
        grid,
        tile_size: max_tile_size,
        path_renderer,
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
    
    let render_params = RenderParams::default();
    
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
            
            let transform = Transform2D {
                translation: Vec2::new(pos_x, pos_y),
                scale,
                rotation: 0.0,
            };
            
            for element in elements {
                // Only draw if the element should be visible
                if model.grid.should_draw_element(element) {
                    model.path_renderer.draw_element(
                        &draw,
                        &element.path,
                        &transform,
                        &render_params
                    );
                }
            }
        }
    }
    
    draw.to_frame(app, &frame).unwrap();
}