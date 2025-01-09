#![allow(unused_imports)]

use nannou::prelude::*;
use glyphvis::models::Project;
use glyphvis::services::grid_service;

struct Model {}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("{}", std::any::type_name::<Project>());
    // Load the project file
    let project = Project::load("../glyphmaker/projects/small-cir-d.json")?;

    // Access grid dimensions
    println!("Grid dimensions: {}x{}", project.grid_x, project.grid_y);
    
    // Get a specific glyph
    if let Some(glyph) = project.get_glyph("다") {
        println!("\nGlyph '{}' segments:", glyph.name);
        for (col, row, segment_type) in glyph.get_parsed_segments() {
            println!("Position ({}, {}): {}", col, row, segment_type);
        }
    }
    
    // Get a specific show
    if let Some(show) = project.get_show("b") {
        println!("\nShow '{}' elements:", show.name);
        for (pos, element) in &show.show_order {
            println!("Position {}: {} ({})", pos, element.name, element.element_type);
        }
    }



    nannou::app(model)
        .update(update) 
        .simple_window(view)
        .run();

    Ok(())


}

fn model(_app: &App) -> Model {
    Model {}
}

fn update(_app: &App, _model: &mut Model, _update: Update) {
}


fn view(app: &App, _model: &Model, frame: Frame) {
    let draw = app.draw();

    let sine = app.time.sin();
    let slowersine = (app.time / 2.0).sin();

    let boundary = app.window_rect();

    let x = map_range(sine, -1.0, 1.0, boundary.bottom(), boundary.top());
    let y = map_range(slowersine, -1.0, 1.0, boundary.bottom(), boundary.top());


    draw.background().color(PLUM);

    draw.ellipse().color(STEELBLUE).x_y(x,y);

    draw.to_frame(app, &frame).unwrap();

}