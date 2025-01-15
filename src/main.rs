// src/main.rs
use nannou::prelude::*;

use glyphvis::models::data_model::Project;
use glyphvis::models::grid_model::Grid;
use glyphvis::models::glyph_model::GlyphModel;

use glyphvis::draw::{ Transform2D, DrawParams };
use glyphvis::draw::grid_draw;
use glyphvis::effects::grid_effects::PulseEffect;
use glyphvis::effects::grid_effects::ColorCycleEffect;

struct Model {
    project: Project,
    grid: Grid,
    glyph_model: GlyphModel,
    texture: wgpu::Texture,
    draw: nannou::Draw,
    draw_renderer: nannou::draw::Renderer,
    texture_reshaper: wgpu::TextureReshaper
}

fn main() {
    nannou::app(model)
        .update(update)
        .run();
}

fn model(app: &App) -> Model {
    let window_size: [u32; 2] = [1000, 1000];
    let texture_size: [u32; 2] = [1000, 1000];


    // Load project
    let project = Project::load("../glyphmaker/projects/small-cir-d.json")
    .expect("Failed to load project file");
    
    // Create grid from project
    let grid = Grid::new(&project);
    println!("Created grid with {} elements", grid.elements.len());

    let glyph_model = GlyphModel::new(&project);

    // Create window
    let window_id = app.new_window()
        .size(window_size[0], window_size[1])
        .msaa_samples(1)
        .view(view)
        .key_pressed(key_pressed)
        .build()
        .unwrap();
    let window = app.window(window_id).unwrap();    

    // Retrieve the wgpu device
    let device = window.device();
    
    // Create our custom texture.
    let sample_count = window.msaa_samples();
    let texture = wgpu::TextureBuilder::new()
        .size(texture_size)
        // Our texture will be used as the RENDER_ATTACHMENT for our `Draw` render pass.
        // It will also be SAMPLED by the `TextureCapturer` and `TextureResizer`.
        .usage(wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING)
        // Use nannou's default multisampling sample count.
        .sample_count(sample_count)
        // Use a spacious 16-bit linear sRGBA format suitable for high quality drawing.
        .format(wgpu::TextureFormat::Rgba16Float)
        // Build
        .build(device);

    let draw = nannou::Draw::new();

    let draw_renderer = nannou::draw::RendererBuilder::new()
        .build_from_texture_descriptor(device, texture.descriptor());

    // Create the texture reshaper.
    let texture_view = texture.view().build();
    let texture_sample_count = texture.sample_count();
    let texture_sample_type = texture.sample_type();
    let dst_format = Frame::TEXTURE_FORMAT;
    let texture_reshaper = wgpu::TextureReshaper::new(
        device,
        &texture_view,
        texture_sample_count,
        texture_sample_type,
        sample_count,
        dst_format,
    );

    Model {
        project,
        grid,
        glyph_model,
        texture,
        draw,
        draw_renderer,
        texture_reshaper,
    }
}

fn key_pressed(_app: &App, model: &mut Model, key: Key) {
    match key {
        Key::Space => model.glyph_model.next_glyph(),
        _ => (),
    }
}

fn update(app: &App, model: &mut Model, _update: Update) {
    let debug_flag = false;

    let draw = &model.draw;

    draw.background().color(BLACK);
    
    // Calculate grid layout
    let window_rect = app.window_rect();
    let max_tile_size = f32::min(
        window_rect.w() / model.grid.width as f32,
        window_rect.h() / model.grid.height as f32
    ) * 0.95;                                       // SCALE FACTOR: TO REFACTOR

    let grid_width = max_tile_size * model.grid.width as f32;
    let grid_height = max_tile_size * model.grid.height as f32;
    let offset_x = -grid_width / 2.0;
    let offset_y = -grid_height / 2.0;
    
    // Create default grid DrawParams
    let grid_params = DrawParams {
        color: rgb(0.1, 0.1, 0.1),
        stroke_weight: 10.0,
    };
    
    // Create default glyph DrawParams
    let glyph_params = DrawParams {
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

    // Draw the debug origin
    if debug_flag {
        draw.line()
            .points(pt2(0.0, 0.0), pt2(10.0, 0.0))
            .color(RED);
        draw.line()
            .points(pt2(0.0, 0.0), pt2(0.0, 10.0))
            .color(BLUE);
    }

    // Get and draw background grid segments
    let background_segments = model.grid.get_background_segments(
        grid_params,
        &model.glyph_model.get_active_segments(&model.project),
        Some(&pulse_effect),
        app.time,
        debug_flag
    );
    
    grid_draw::draw_segments(
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

    grid_draw::draw_segments(
        &draw,
        &model.grid,
        &grid_transform,
        glyph_segments,
    );

    // Render the draw commands to the texture
    let window = app.main_window();
    let device = window.device();
    let ce_desc = wgpu::CommandEncoderDescriptor {
        label: Some("Texture renderer"),
    };
    let mut encoder = device.create_command_encoder(&ce_desc);
    model
        .draw_renderer
        .render_to_texture(device, &mut encoder, draw, &model.texture);

    // Submit the commands for drawing to the GPU
    window.queue().submit(Some(encoder.finish()));

}

// Draw the state of Model into the given Frame
fn view(_app: &App, model: &Model, frame: Frame) {
    
    // Sample the texture and write it to the Frame
    let mut encoder = frame.command_encoder();

    model
        .texture_reshaper
        .encode_render_pass(frame.texture_view(), &mut *encoder);

    /* higher level way of doing the same thing
    let draw = app.draw();
    
    // Get the texture view
    let texture_view = model.texture.view().build();
    
    // Draw the texture to fill the window
    draw.texture(&texture_view)
        .wh(frame.rect().wh());
    
    draw.to_frame(app, &frame).unwrap();
    */

}