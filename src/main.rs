// src/main.rs
use nannou::prelude::*;

use glyphvis::models::data_model::Project;
use glyphvis::models::grid_model::Grid;
use glyphvis::render::path_renderer::PathRenderer;
use glyphvis::render::{Transform2D, RenderParams, GlyphDisplay};

struct Model {
    grid: Grid,
    tile_size: f32,
    path_renderer: PathRenderer,
    project: Project,
    glyph_display: GlyphDisplay,
}

fn main() {
    nannou::app(model)
        .event(event)        // Add event handling
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
    let glyph_display = GlyphDisplay::new(&project);

    Model {
        grid,
        tile_size: max_tile_size,
        path_renderer,
        project,
        glyph_display,
    }
}

fn event(app: &App, model: &mut Model, event: Event) {
    if let Event::WindowEvent { simple: Some(KeyPressed(key)), .. } = event {
        match key {
            Key::Space => model.glyph_display.next_glyph(),
            _ => (),
        }
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
    
    // Get active segments for current glyph
    let active_segments = model.glyph_display.get_active_segments(&model.project);
    
    // First draw the background grid
    let grid_params = RenderParams {
        color: rgb(0.1, 0.1, 0.1),  // Dark gray for inactive grid
        stroke_weight: 5.0,
    };
    
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
            
            let transform = Transform2D {
                translation: Vec2::new(pos_x, pos_y), // Flip y-axis
                scale: model.tile_size / model.grid.viewbox.width,
                rotation: 0.0,
            };
            
            // Draw all elements at this grid position
            let elements = model.grid.get_elements_at(x, y);
            
            for element in &elements {  // Reference the elements
                // Only draw grid elements that aren't part of the active glyph
                if model.grid.should_draw_element(element) {
                    let segment_id = format!("{},{} : {}", x, y, element.id);
                    if !active_segments.contains(&segment_id) {
                        model.path_renderer.draw_element(
                            &draw,
                            &element.path,
                            &transform,
                            &grid_params
                        );
                    }
                }
            }
            
            // Draw active glyph segments on top
            for element in &elements {  // Reference the elements again
                if model.grid.should_draw_element(element) {
                    let segment_id = format!("{},{} : {}", x, y, element.id);
                    if active_segments.contains(&segment_id) {
                        let glyph_params = RenderParams {
                            color: rgb(0.9, 0.9, 0.9),  // Bright white for active segments
                            stroke_weight: 5.0,
                        };
                        model.path_renderer.draw_element(
                            &draw,
                            &element.path,
                            &transform,
                            &glyph_params
                        );
                    }
                }
            }
            
        }
    }
    draw.to_frame(app, &frame).unwrap();
}