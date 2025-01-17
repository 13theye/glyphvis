// src/main.rs
use nannou::prelude::*;

use glyphvis::models::{ Project, GridModel, GlyphModel };

use glyphvis::services::FrameRecorder;
use glyphvis::services::frame_recorder::OutputFormat;

use glyphvis::draw::{ Transform2D, DrawParams };
use glyphvis::draw::grid_draw;
use glyphvis::effects::grid_effects::{ PulseEffect, ColorCycleEffect };

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
const PROJECT_PATH: &str = "/Users/jeanhank/Code/glyphmaker/projects/ulsan.json";


struct Model {
    project: Project,
    grid: GridModel,
    glyph_model: GlyphModel,
    texture: wgpu::Texture,
    draw: nannou::Draw,
    draw_renderer: nannou::draw::Renderer,
    texture_reshaper: wgpu::TextureReshaper,
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
    let grid = GridModel::new(&project);
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
        .usage(wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING )
        // Use nannou's default multisampling sample count.
        .sample_count(texture_samples)
        // Use a spacious 16-bit linear sRGBA format suitable for high quality drawing. Rgba16Float
        // Use 8-bit for standard quality and better perforamnce. Rgba8Unorm Rgb10a2Unorm
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

    // Create the frame recorder
    let frame_recorder = FrameRecorder::new(
        OUTPUT_DIR,
        FRAME_LIMIT,
        OUTPUT_FORMAT,
    );

    Model {
        project,
        grid,
        glyph_model,
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
        Key::Space => model.glyph_model.next_glyph(),
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
    let debug_flag = false;

    // auto cycle glyphs
    //model.glyph_model.next_glyph();

    // frames processing progress bar:
    if model.exit_requested {
        if model.frame_recorder.has_pending_frames() {
            // Clear the window and show progress
            let draw = &model.draw;
            draw.background().color(BLACK);
            
            let (processed, total) = model.frame_recorder.get_queue_status();
            
            // Draw progress text
            let text = format!("{} / {}\nframes saved", processed, total);
            draw.text(&text)
                .color(WHITE)
                .font_size(32)
                .x_y(0.0, 50.0);
                
            // Draw progress bar
            let progress = processed as f32 / total as f32;
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


            // Render progress graphic:
            let window = app.main_window();
            let device = window.device();
            let ce_desc = wgpu::CommandEncoderDescriptor {
                label: Some("Texture renderer"),};
            let mut encoder = device.create_command_encoder(&ce_desc);
            let texture_view = model.texture.view().build();

            model.draw_renderer.encode_render_pass(device, &mut encoder, &model.draw, 2.0, model.texture.size(), &texture_view, None);
            window.queue().submit(Some(encoder.finish()));        

            // IMPORTANT: Add a small sleep to prevent maxing out CPU
            std::thread::sleep(std::time::Duration::from_millis(200));
        } else {
            // Only quit once all frames are processed
            app.quit();
        }
        return;  // Important: return here to not continue with normal rendering
    }

    // normal rendering routine begin:

    let draw = &model.draw;

    draw.background().color(BLACK);
    
    // Calculate grid layout
    //let window_rect = app.window_rect();
    let max_tile_size = f32::min(
        //window_rect.w() / model.grid.width as f32,
        //window_rect.h() / model.grid.height as f32
        model.texture.size()[1] as f32 / 2.0 / model.grid.width as f32,
        model.texture.size()[1] as f32 / 2.0 / model.grid.height as f32
    ) * 0.95;                                       // SCALE FACTOR: TO REFACTOR

    let grid_width = max_tile_size * model.grid.width as f32;
    let grid_height = max_tile_size * model.grid.height as f32;
    let offset_x = -grid_width / 2.0;
    let offset_y = -grid_height / 2.0;
    
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


    // render path draw to texture only & capture
    let window = app.main_window();
    let device = window.device();
    let ce_desc = wgpu::CommandEncoderDescriptor {
        label: Some("Texture renderer"),};
    let mut encoder = device.create_command_encoder(&ce_desc);
    let texture_view = model.texture.view().build();

    model.draw_renderer.encode_render_pass(device, &mut encoder, &model.draw, 2.0, model.texture.size(), &texture_view, None);

    // Capture the texture for FrameRecorder
    if model.frame_recorder.is_recording() {
        model.frame_recorder.capture_frame(device, &mut encoder, &model.texture);
    }

    window.queue().submit(Some(encoder.finish()));

}

// Draw the state of Model into the given Frame
fn view(_app: &App, model: &Model, frame: Frame) {

    //resize texture to screen
    let mut encoder = frame.command_encoder();

    model
        .texture_reshaper
        .encode_render_pass(frame.texture_view(), &mut *encoder);

}