// src/main.rs
use nannou::prelude::*;

use glyphvis::models::data_model::Project;
use glyphvis::models::grid_model::Grid;
use glyphvis::models::glyph_model::GlyphModel;

use glyphvis::render::{ Transform2D, RenderParams, Renderer };
use::glyphvis::effects::grid_effects::PulseEffect;
use glyphvis::effects::grid_effects::ColorCycleEffect;


struct Model {
    project: Project,
    grid: Grid,
    glyph_model: GlyphModel,
    renderer: Renderer,
}

fn main() {
    nannou::app(model)
        .update(update)
        .run();
}

fn model(app: &App) -> Model {
    // Create window
    let _window = app.new_window()
        .size(1000, 1000)
        .view(view)
        .key_pressed(key_pressed)
        .build()
        .unwrap();
    
    // Load project
    let project = Project::load("../glyphmaker/projects/small-cir-d.json")
        .expect("Failed to load project file");
    
    // Create grid from project
    let grid = Grid::new(&project);
    println!("Created grid with {} elements", grid.elements.len());

    let glyph_model = GlyphModel::new(&project);
    let renderer = Renderer::new(grid.viewbox.clone());

    Model {
        project,
        grid,
        glyph_model,
        renderer,
    }
}

fn key_pressed(_app: &App, model: &mut Model, key: Key) {
    match key {
        Key::Space => model.glyph_model.next_glyph(),
        _ => (),
    }
}

fn update(_app: &App, _model: &mut Model, _update: Update) {
}

fn view(app: &App, model: &Model, frame: Frame) {
    let debug_flag = false;

    let draw = app.draw();
    draw.background().color(BLACK);

    // Draw the debug origin
    draw.line()
        .points(pt2(0.0, 0.0), pt2(10.0, 0.0))
        .color(RED);
    draw.line()
        .points(pt2(0.0, 0.0), pt2(0.0, 10.0))
        .color(BLUE);
    
    // Calculate grid layout
    let window = app.window_rect();
    let max_tile_size = f32::min(
        window.w() / model.grid.width as f32,
        window.h() / model.grid.height as f32
    ) * 0.95;                                       // SCALE FACTOR: TO REFACTOR

    let grid_width = max_tile_size * model.grid.width as f32;
    let grid_height = max_tile_size * model.grid.height as f32;
    let offset_x = -grid_width / 2.0;
    let offset_y = -grid_height / 2.0;
    
    // Create default grid RenderParams
    let grid_params = RenderParams {
        color: rgb(0.1, 0.1, 0.1),
        stroke_weight: 10.0,
    };
    
    // Create default glyph RenderParams
    let glyph_params = RenderParams {
        color: rgb(0.0, 0.0, 0.0),
        stroke_weight: 10.0,
    };

    // Create grid transform
    let grid_transform = Transform2D {
        translation: Vec2::new(offset_x, offset_y),
        scale: max_tile_size / model.grid.viewbox.width,
        rotation: 0.0,
    };

    let pulse_effect = PulseEffect {
        frequency: 1.0,
        min_brightness: 0.0,
        max_brightness: 0.5,
    };

    let colorcycle_effect = ColorCycleEffect {
        frequency: 1.0,
        saturation: 1.0,
        brightness: 1.0,
    };

    // Get and draw background grid segments
    let background_segments = model.grid.get_background_segments(
        grid_params,
        &model.glyph_model.get_active_segments(&model.project),
        Some(&pulse_effect),
        app.time,
        debug_flag
    );
    
    model.renderer.draw(
        &draw,
        &model.grid,
        &grid_transform,
        background_segments,
    );

    // Get and draw glyph segments
    let glyph_segments = model.glyph_model.get_renderable_segments(
        &model.project,
        &model.grid,
        glyph_params,
        Some(&colorcycle_effect),
        app.time,
        debug_flag
    );

    model.renderer.draw(
        &draw,
        &model.grid,
        &grid_transform,
        glyph_segments,
    );

    // Draw to frame
    draw.to_frame(app, &frame).unwrap();
}