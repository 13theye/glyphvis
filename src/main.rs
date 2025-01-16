// src/main.rs
use nannou::prelude::*;

use glyphvis::models::data_model::Project;
use glyphvis::models::grid_model::Grid;
use glyphvis::models::glyph_model::GlyphModel;

use glyphvis::services::FrameRecorder;
use glyphvis::services::frame_recorder::OutputFormat;

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
    texture_reshaper: wgpu::TextureReshaper,
    frame_recorder: FrameRecorder,
    exit_requested: bool,
    new_render_flag: bool,
}

fn main() {
    nannou::app(model)
        .update(update)
        .run();
}

fn model(app: &App) -> Model {
    let window_size: [u32; 2] = [1000, 1000];
    let texture_size: [u32; 2] = [2000, 2000];
    let texture_samples = 4;

    // set to 'true' to try new render pipeline
    let new_render_flag = true;


    // Load project
    let project = Project::load("/Users/jeanhank/Code/glyphmaker/projects/ulsan.json")
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
        .usage(wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING )
        // Use nannou's default multisampling sample count.
        .sample_count(texture_samples)
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

    // Create the frame recorder
    let frame_recorder = FrameRecorder::new(
        "frames/",
        9999,
        OutputFormat::JPEG(85),
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
        new_render_flag
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

    if model.exit_requested {
        if model.frame_recorder.has_pending_frames() {
            // Clear the window and show progress
            let draw = &model.draw;
            draw.background().color(BLACK);
            
            let (processed, total) = model.frame_recorder.get_queue_status();
            
            // Draw progress text
            let text = format!("{} / {} frames saved", processed, total);
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

            // Use the draw renderer to update the texture
            let window = app.main_window();
            let device = window.device();
            let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Progress renderer"),
            });
            
            model.draw_renderer.render_to_texture(
                device,
                &mut encoder,
                draw,
                &model.texture
            );
            
            window.queue().submit(Some(encoder.finish()));

            // IMPORTANT: Add a small sleep to prevent maxing out CPU
            std::thread::sleep(std::time::Duration::from_millis(200));
        } else {
            // Only quit once all frames are processed
            app.quit();
        }
        return;  // Important: return here to not continue with normal rendering
    }

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

    if !model.new_render_flag {
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
        
        

        // Capture the texture for FrameRecorder
        if model.frame_recorder.is_recording() {
            model.frame_recorder.capture_frame(device, &mut encoder, &model.texture);
        }

        // Submit the commands for drawing to the GPU
        window.queue().submit(Some(encoder.finish()));
    } else {
        // experimental rendering path: do nothing here
    }

}

// Draw the state of Model into the given Frame
fn view(app: &App, model: &Model, frame: Frame) {

    if model.exit_requested && model.frame_recorder.has_pending_frames() {
        // After exit progress loop routine:
        let draw = app.draw();
        let texture_view = model.texture.view().build();
        draw.texture(&texture_view)
            .wh(frame.rect().wh());
        draw.to_frame(app, &frame).unwrap();

    } else {
        if !model.new_render_flag{
            // Normal rendering path
            let mut encoder = frame.command_encoder();
            
            model
                .texture_reshaper
                .encode_render_pass(frame.texture_view(), &mut *encoder);

        } else {
            // experimental rendering path
            let window = app.main_window();
            let device = window.device();

            let ce_desc = wgpu::CommandEncoderDescriptor {
                label: Some("Texture renderer"),};
            let mut encoder = device.create_command_encoder(&ce_desc);

            let texture_view = model.texture.view().build();

            let mut draw_renderer = nannou::draw::RendererBuilder::new()
                .build_from_texture_descriptor(device, model.texture.descriptor());

            draw_renderer.encode_render_pass(device, &mut encoder, &model.draw, 2.0, [2000,2000], &texture_view, Some(frame.texture_view()));


            // Capture the texture for FrameRecorder
            if model.frame_recorder.is_recording() {
                model.frame_recorder.capture_frame(device, &mut encoder, &model.texture);
            }
            window.queue().submit(Some(encoder.finish()));
        }
    }


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