// src/main.rs
use nannou::prelude::*;
use std::collections::HashMap;
use rand::Rng;

use glyphvis::{
    models:: Project,
    views:: { GridInstance, CachedGrid, DrawStyle },
    controllers:: GlyphController,
    services:: { FrameRecorder, OutputFormat },
    effects::{ EffectsManager, init_effects },

};

// APP CONSTANTS TO EVENTUALLY BE MOVED TO CONFIG FILE

// size of the render and capture
const TEXTURE_SIZE: [u32; 2] = [4742, 1200];
// number of samples for the texture
const TEXTURE_SAMPLES: u32 = 4;
// path to the output frames
const OUTPUT_DIR: &str = "./frames/";
// capture frame limit
const FRAME_LIMIT: u32 = 30000;
// output format
const OUTPUT_FORMAT: OutputFormat = OutputFormat::JPEG(85);

// size of the window monitor: nice when aspect ratio is same as texture size aspect ratio
const WINDOW_SIZE: [u32; 2] = [1897, 480];
// path to the project file
const PROJECT_PATH: &str = "/Users/jeanhank/Code/glyphmaker/projects/ulsan.json";
// grid scale factor
//const GRID_SCALE_FACTOR: f32 = 0.95; //  won't need this eventually when we define Grid size when drawing


struct Model {
    // Core components:
    project: Project,
    grids: HashMap<String, GridInstance>,    //grid: CachedGrid,
    glyphs: GlyphController,

    // Rendering components:
    //effects_manager: EffectsManager,
    texture: wgpu::Texture,
    draw: nannou::Draw,
    draw_renderer: nannou::draw::Renderer,
    texture_reshaper: wgpu::TextureReshaper,
    random: rand::rngs::ThreadRng,
    effect_target_style: DrawStyle, // for testing

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
    
    /*
    // Create grid from project
    let grid = CachedGrid::new(&project);
    println!("Created grid with {} segments", grid.segments.len());
    */

    let glyphs = GlyphController::new(&project);

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

    // Create effects
    

    Model {
        project,
        grids: HashMap::new(),   //grid,
        glyphs,

        //effects_manager: init_effects(&app),
        texture,
        draw,
        draw_renderer,
        texture_reshaper,
        random: rand::thread_rng(),
        effect_target_style: DrawStyle {
            color: rgb(1.0, 0.0, 0.0),
            stroke_weight: 5.0,
        },

        frame_recorder,
        exit_requested: false,
    }
}

fn key_pressed(app: &App, model: &mut Model, key: Key) {
    match key {
        Key::Space => {
            let segment_ids = model.glyphs.next_glyph(&model.project);
            let glyph_style = DrawStyle {
                color: rgb(model.random.gen(), model.random.gen(), model.random.gen()),
                stroke_weight: 5.0,
            };
            
            for (_, grid_instance) in model.grids.iter_mut() {
                for segment_id in &segment_ids {
                    grid_instance.effects_manager.activate_segment(&segment_id, "power_on", app.time);

                    model.effect_target_style = glyph_style.clone();
                }
            }
        }, 
        Key::G => {
            make_three_grids(app, model);
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
    draw.background().color(BLACK);

    // Apply the current effect
    let bg_style = DrawStyle {
        color: rgb(0.2, 0.2, 0.2),
        stroke_weight: 5.0,
    };

    let glyph_style = DrawStyle {
        color: rgb(0.7, 0.1, 0.1),
        stroke_weight: 5.0,
    };



    for (_, grid_instance) in model.grids.iter() {

        let glyph_segments = model.glyphs.get_renderable_segments(
            &model.project,
            &grid_instance,
            &model.effect_target_style,
            &grid_instance.effects_manager,
            app.time,
            false,
            false,
        );

        let bg_segments = model.glyphs.get_renderable_segments(
            &model.project,
            &grid_instance,
            &bg_style,
            &grid_instance.effects_manager,
            app.time,
            true,
            false,
        );

        //grid.effects_manager.apply_effects(&grid.grid.id, bg_style, app.time);
        //grid.effects_manager.apply_effects(&grid.grid.id, model.effect_target_style, app.time);
        grid_instance.draw_segments(&draw, bg_segments);
        grid_instance.draw_segments(&draw, glyph_segments);
    }
    

    //grid.grid.draw_segments(&draw, bg_segments);
    //grid.grid.draw_segments(&draw, glyph_segments);

/*
    // Add debug visualization of coordinate system
    draw.line()
        .points(pt2(0.0, 0.0), pt2(100.0, 0.0))
        .color(RED)
        .stroke_weight(1.0);
    draw.line()
        .points(pt2(0.0, 0.0), pt2(0.0, 100.0))
        .color(BLUE)
        .stroke_weight(1.0);
 */


    // Rnder to texture and handle frame recording
    render_and_capture(app, model);


}

// Draw the state of Model into the given Frame
fn view(_app: &App, model: &Model, frame: Frame) {

    //resize texture to screen
    let mut encoder = frame.command_encoder();

    model
        .texture_reshaper
        .encode_render_pass(frame.texture_view(), &mut *encoder);

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

fn make_three_grids(app: &App, model: &mut Model) {

    let grid_1 = GridInstance::new(app, &model.project, "Grid Left".to_string(), pt2(-600.0, 0.0), 0.0);
    let grid_2 = GridInstance::new(app, &model.project, "Grid Center".to_string(), pt2(0.0, 0.0), 0.0);
    let grid_3 = GridInstance::new(app, &model.project, "Grid Right".to_string(), pt2(600.0, 0.0), 0.0);

    model.grids.insert(grid_1.id.clone(),grid_1);
    model.grids.insert(grid_2.id.clone(), grid_2);
    model.grids.insert(grid_3.id.clone(), grid_3);

    for (_, grid) in model.grids.iter() {
        grid.print_grid_info();
    }
}


/*
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
    */