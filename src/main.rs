// src/main.rs
use nannou::prelude::*;

use glyphvis::{
    models::{ Project, ViewBox },
    views:: { CachedGrid, Transform2D, DrawCommand, DrawStyle },
    services:: { FrameRecorder, OutputFormat },
    effects::grid_effects::{ GridEffect, PulseEffect },
};

//use glyphvis::views::Transform2D;

//use glyphvis::services::FrameRecorder;
//use glyphvis::services::frame_recorder::OutputFormat;

//use glyphvis::draw::DrawParams;
//use glyphvis::draw::grid_draw;
//use glyphvis::effects::grid_effects::{ PulseEffect, ColorCycleEffect };

// APP CONSTANTS TO EVENTUALLY BE MOVED TO CONFIG FILE

// size of the render and capture
const TEXTURE_SIZE: [u32; 2] = [3840, 1280];
// number of samples for the texture
const TEXTURE_SAMPLES: u32 = 4;
// path to the output frames
const OUTPUT_DIR: &str = "./frames/";
// capture frame limit
const FRAME_LIMIT: u32 = 20000;
// output format
const OUTPUT_FORMAT: OutputFormat = OutputFormat::JPEG(85);

// size of the window monitor: nice when aspect ratio is same as texture size aspect ratio
const WINDOW_SIZE: [u32; 2] = [1000, 333];
// path to the project file
const PROJECT_PATH: &str = "/Users/jeanhank/Code/glyphmaker/projects/debug.json";
// grid scale factor
const GRID_SCALE_FACTOR: f32 = 0.95; //  won't need this eventually when we define Grid size when drawing


struct Model {
    // Core components:
    project: Project,
    grid: CachedGrid,
    current_effect: Box<dyn GridEffect>,

    // Rendering components:
    texture: wgpu::Texture,
    draw: nannou::Draw,
    draw_renderer: nannou::draw::Renderer,
    texture_reshaper: wgpu::TextureReshaper,

    // Frame recording:
    frame_recorder: FrameRecorder,
    exit_requested: bool,
}

fn main() {
    nannou::app(model)
        .update(update)
        .run();
}

fn model(app: &App) -> Model {

    // size of captures
    let texture_size= TEXTURE_SIZE;

    // size of view window
    let window_size= WINDOW_SIZE;

    let texture_samples = TEXTURE_SAMPLES;

    // Load project
    let project = Project::load(PROJECT_PATH)
        .expect("Failed to load project file");
    
    // Create grid from project
    let mut grid = CachedGrid::new(&project);
    println!("Created grid with {} segments", grid.segments.len());

    // let glyph_model = GlyphModel::new(&project);

    // Set initial glyph
    grid.set_glyph(Some("쿵3"), &project);

    // Create window
    let window_id = app.new_window()
        .size(window_size[0], window_size[1])
        .msaa_samples(1)
        .view(view)
        .key_pressed(key_pressed)
        .build()
        .unwrap();
    let window = app.window(window_id).unwrap();

    // Set up render texture
    let device = window.device();
    let draw = nannou::Draw::new();
    let texture = wgpu::TextureBuilder::new()
        .size(texture_size)
        // Our texture will be used as the RENDER_ATTACHMENT for our `Draw` render pass.
        // It will also be SAMPLED by the `TextureCapturer` and `TextureResizer`.
        .usage(wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING )
        // Use nannou's default multisampling sample count.
        .sample_count(texture_samples)
        // Use a spacious 16-bit linear sRGBA format suitable for high quality drawing. Rgba16Float
        // Use 8-bit for standard quality and better perforamnce. Rgba8Unorm Rgb10a2Unorm
        .format(wgpu::TextureFormat::Rgba16Float)
        // Build
        .build(device);

    // Set up rendering pipeline
    let draw_renderer = nannou::draw::RendererBuilder::new()
    .build_from_texture_descriptor(device, texture.descriptor());
    let sample_count = window.msaa_samples();

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

    // Create the frame recorder
    let frame_recorder = FrameRecorder::new(
        OUTPUT_DIR,
        FRAME_LIMIT,
        OUTPUT_FORMAT,
    );

    // Set up initial effect
    let effect: Box<dyn GridEffect> = Box::new(PulseEffect {
        frequency: 1.0,
        min_brightness: 0.2,
        max_brightness: 0.6,
    });

    Model {
        project,
        grid,
        //glyph_model,
        current_effect: effect,
        
        texture,
        draw,
        draw_renderer,
        texture_reshaper,

        frame_recorder,
        exit_requested: false,
    }
}

fn key_pressed(_app: &App, model: &mut Model, key: Key) {
    match key {
        Key::Space => {
            // Get list of glyph names from project
            let glyph_names: Vec<&String> = model.project.glyphs.keys().collect();
            if !glyph_names.is_empty() {
                // Find current glyph and get next one
                let current = model.grid.get_active_glyph();
                let next_idx = match current {
                    Some(current_name) => {
                        glyph_names.iter()
                            .position(|&name| name == current_name)
                            .map(|pos| (pos + 1) % glyph_names.len())
                            .unwrap_or(0)
                    },
                    None => 0
                };
                // Set next glyph
                let next_name = glyph_names[next_idx];
                model.grid.set_glyph(Some(next_name), &model.project);
                println!("Switched to glyph: {}", next_name);
            }
        },
        Key::R => model.frame_recorder.toggle_recording(),
        Key::Q => {
            let (processed, total) = model.frame_recorder.get_queue_status();
            println!("Processed {} frames out of {}", processed, total);
            if model.frame_recorder.is_recording() {
                model.frame_recorder.toggle_recording();
            }
            model.exit_requested = true;
        },
        _ => (),
    }
}

fn update(app: &App, model: &mut Model, _update: Update) {
    //let debug_flag = false;

    // auto cycle glyphs
    //model.glyph_model.next_glyph();

    // frames processing progress bar:
    if model.exit_requested {
        handle_exit_state(app, model);  
        return;  // Important: return here to not continue with normal rendering
    }

    // normal rendering routine begin:
    let draw = &model.draw;
    draw.background().color(ORANGE);

    // Apply the current effect
    let base_params = DrawStyle {
        color: rgb(0.2, 0.2, 0.2),
        stroke_weight: 10.0,
    };

    //let effect_params = model.current_effect.apply(&base_params, app.time);

    /* 
    // we're going to want to move this to the CachedGrid model
    for segment in model.grid.segments.values_mut() {
        for cmd in &mut segment.draw_commands {
            match cmd {
                DrawCommand::Line { color, stroke_weight, .. } |
                DrawCommand::Arc { color, stroke_weight, .. } |
                DrawCommand::Circle { color, stroke_weight, .. } => {
                    *color = effect_params.color;
                    *stroke_weight = effect_params.stroke_weight;
                }
            }
        }
    }
    */

    //model.grid.draw_full_grid(&draw);
    // Draw grid with effects
    model.grid.draw(&draw);

    // Add debug visualization of coordinate system
    draw.line()
        .points(pt2(0.0, 0.0), pt2(100.0, 0.0))
        .color(RED)
        .stroke_weight(1.0);
    draw.line()
        .points(pt2(0.0, 0.0), pt2(0.0, 100.0))
        .color(BLUE)
        .stroke_weight(1.0);

    // Rnder to texture and handle frame recording
    render_and_capture(app, model);

    /* 
    // Create default grid DrawParams
    let grid_params = DrawParams {
        color: rgb(0.2, 0.2, 0.2),
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
        min_brightness: 0.2,
        max_brightness: 0.6,
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
        &background_segments,
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
        &glyph_segments,
    );

    */

}

// Draw the state of Model into the given Frame
fn view(_app: &App, model: &Model, frame: Frame) {

    //resize texture to screen
    let mut encoder = frame.command_encoder();

    model
        .texture_reshaper
        .encode_render_pass(frame.texture_view(), &mut *encoder);

}


// ******************************* Grid Setup Helpers ********************************

fn calculate_grid_transform(texture: &wgpu::Texture, grid: &CachedGrid) -> Transform2D {
    // Calculate scale to fit grid in view
    // Calculate base scale from view height
    println!("\n=== Grid Transform Calculation ===");

    let (grid_x, grid_y) = grid.dimensions;
    let texture_height = texture.size()[1] as f32;
    
    // Calculate scale to fit grid in view
    let max_grid_dim = grid_x.max(grid_y) as f32;
    //let base_scale = texture_height / 2.0 / max_grid_dim;
    //let scale = base_scale * GRID_SCALE_FACTOR;
    let scale = 1.0;
    
    println!("Grid dimensions: {}x{}", grid_x, grid_y);
    println!("Texture height: {}", texture_height);
    //println!("Base scale: {}", base_scale);
    println!("Final scale: {}", scale);

    // Center the grid
    let grid_size = Vec2::new(
        scale * grid_x as f32,
        scale * grid_y as f32
    );
    let offset = -grid_size / 2.0;
    
    println!("Grid size: {:?}", grid_size);
    println!("Center offset: {:?}", offset);

    Transform2D {
        translation: offset,
        scale,
        rotation: 0.0,
    }
}


// ******************************* Rendering and Capture *****************************

fn render_and_capture(app: &App, model: &mut Model) {
    let window = app.main_window();
    let device = window.device();
    let ce_desc = wgpu::CommandEncoderDescriptor {
        label: Some("Texture renderer"),};
    let mut encoder = device.create_command_encoder(&ce_desc);
    let texture_view = model.texture.view().build();

    model.draw_renderer.encode_render_pass(
        device, 
        &mut encoder, 
        &model.draw, 
        2.0, 
        model.texture.size(), 
        &texture_view, 
        None
    );

    // Capture the texture for FrameRecorder
    if model.frame_recorder.is_recording() {
        model.frame_recorder.capture_frame(device, &mut encoder, &model.texture);
    }

    window.queue().submit(Some(encoder.finish()));
}


// ******************************* Exit State Handling *******************************

fn handle_exit_state(app: &App, model: &mut Model) {
    if model.frame_recorder.has_pending_frames() {
        draw_progress_screen(app, model);
        std::thread::sleep(std::time::Duration::from_millis(200));
    } else {
        app.quit(); // quit only once all frames are processed
    }
}

fn draw_progress_screen(app: &App, model: &mut Model) {
    // Clear the window and show progress
    let draw = &model.draw;
    draw.background().color(BLACK);

    let (processed, total)  = model.frame_recorder.get_queue_status();

    // Draw progress text
    let text = format!("{} / {}\nframes saved", processed, total);
    draw.text(&text)
        .color(WHITE)
        .font_size(32)
        .x_y(0.0, 50.0);
        
    // Draw progress bar
    let progress = processed as f32 / total as f32;
    draw_progress_bar(draw, progress);

    // Render progress screen
    render_progress(app, model);
}

fn draw_progress_bar(draw: &Draw, progress: f32) {
    let bar_width = 400.0;
    let bar_height = 30.0;

    // Background bar
    draw.rect()
    .color(GRAY)
    .w_h(bar_width, bar_height)
    .x_y(0.0, -50.0);
        
    // Progress bar
    draw.rect()
        .color(GREEN)
        .w_h(bar_width * progress, bar_height)
        .x_y(-bar_width/2.0 + (bar_width * progress)/2.0, -50.0);
}

fn render_progress(app: &App, model: &mut Model) {
    let window = app.main_window();
    let device = window.device();
    let ce_desc = wgpu::CommandEncoderDescriptor {
        label: Some("Progress renderer"),};
    let mut encoder = device.create_command_encoder(&ce_desc);
    let texture_view = model.texture.view().build();

    model.draw_renderer.encode_render_pass(
        device, 
        &mut encoder, 
        &model.draw, 
        2.0, 
        model.texture.size(), 
        &texture_view, 
        None
    );
    window.queue().submit(Some(encoder.finish()));        
}





// ******************************* Debug stuff *******************************


fn print_grid_info(grid: &CachedGrid) {
    println!("\nGrid Info:");
    println!("Dimensions: {:?}", grid.dimensions);
    println!("Viewbox: {:?}", grid.viewbox);
    println!("Segment count: {}", grid.segments.len());
    
    // Print first few segments for inspection
    
    for (i, (id, segment)) in grid.segments.iter().take(2).enumerate() {
        println!("\nSegment {}: {}", i, id);
        println!("Position: {:?}", segment.tile_pos);
        println!("Edge type: {:?}", segment.edge_type);
        
        for (j, cmd) in segment.draw_commands.iter().take(2).enumerate() {
            println!("  Command {}: {:?}", j, cmd);
        }
    }
     
}