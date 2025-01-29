// src/main.rs
use nannou::prelude::*;
use rand::Rng;
use std::collections::HashMap;
use std::time::Instant;

use glyphvis::{
    animation::{TransitionConfig, TransitionEngine},
    controllers::GlyphController,
    models::Project,
    services::{FrameRecorder, OutputFormat},
    //effects::{ EffectsManager, init_effects },
    views::{DrawStyle, GridInstance, Transform2D},
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
const PROJECT_PATH: &str = "/Users/jeanhank/Code/glyphmaker/projects/small-cir-d2.json";
// grid scale factor
//const GRID_SCALE_FACTOR: f32 = 0.95; //  won't need this eventually when we define Grid size when drawing

// const BPM: u32 = 120;

struct Model {
    // Core components:
    project: Project,
    grids: HashMap<String, GridInstance>, //<grid_id : CachedGrid>
    glyphs: GlyphController,

    // Rendering components:
    texture: wgpu::Texture,
    draw: nannou::Draw,
    draw_renderer: nannou::draw::Renderer,
    texture_reshaper: wgpu::TextureReshaper,
    random: rand::rngs::ThreadRng,

    // Segment Transitions
    transition_engine: TransitionEngine,

    // Message
    needs_glyph_update: bool,
    debug_flag: bool,

    // Style
    effect_target_style: DrawStyle, // for testing

    // Frame recording:
    frame_recorder: FrameRecorder,
    exit_requested: bool,

    // FPS
    last_update: Instant,
    fps: f32,
}

fn main() {
    nannou::app(model).update(update).run();
}

fn model(app: &App) -> Model {
    // size of captures
    let texture_size = TEXTURE_SIZE;

    // size of view window
    let window_size = WINDOW_SIZE;

    let texture_samples = TEXTURE_SAMPLES;

    // Load project
    let project = Project::load(PROJECT_PATH).expect("Failed to load project file");

    /*
    // Create grid from project
    let grid = CachedGrid::new(&project);
    println!("Created grid with {} segments", grid.segments.len());
    */

    let glyphs = GlyphController::new(&project);

    // Create window
    let window_id = app
        .new_window()
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
        .usage(wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING)
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

    let transition_config = TransitionConfig {
        steps: 50,
        frame_duration: 0.05,
        wandering: 1.0,
        density: 0.01,
    };

    // Create the frame recorder
    let frame_recorder =
        FrameRecorder::new(device, &texture, OUTPUT_DIR, FRAME_LIMIT, OUTPUT_FORMAT);

    // Create effects

    Model {
        project,
        grids: HashMap::new(), //grid,
        glyphs,

        texture,
        draw,
        draw_renderer,
        texture_reshaper,
        random: rand::thread_rng(),

        needs_glyph_update: false,
        debug_flag: false,

        effect_target_style: DrawStyle {
            color: rgb(1.0, 0.0, 0.0),
            stroke_weight: 5.0,
        },

        transition_engine: TransitionEngine::new(transition_config),

        frame_recorder,
        exit_requested: false,

        // FPS
        last_update: Instant::now(),
        fps: 0.0,
    }
}

fn key_pressed(app: &App, model: &mut Model, key: Key) {
    match key {
        // show next glyph
        Key::Space => {
            for (_, grid_instance) in model.grids.iter_mut() {
                model.glyphs.next_glyph(
                    &model.project,
                    grid_instance,
                    &model.transition_engine,
                    app.time,
                );
            }
            model.needs_glyph_update = true;
        }
        // Return grids to where they spawned
        Key::Backslash => {
            for (_, grid_instance) in model.grids.iter_mut() {
                grid_instance.reset_location();
            }
        }
        // Init grids or hide/show them
        Key::G => {
            if model.grids.is_empty() {
                make_three_grids(app, model);
            } else {
                for (name, grid_instance) in model.grids.iter_mut() {
                    if name != "Grid Center" {
                        grid_instance.visible = !grid_instance.visible;
                    }
                }
            }
        }
        Key::R => model.frame_recorder.toggle_recording(),
        // Graceful quit that waits for frame queue to be processed
        Key::Q => {
            let (processed, total) = model.frame_recorder.get_queue_status();
            println!("Processed {} frames out of {}", processed, total);
            if model.frame_recorder.is_recording() {
                model.frame_recorder.toggle_recording();
            }
            model.exit_requested = true;
        }
        // Move grids 10pts to the right
        Key::Right => {
            let position_delta = Transform2D {
                translation: pt2(10.0, 0.0),
                scale: 1.0,
                rotation: 0.0,
            };
            for (_, grid_instance) in model.grids.iter_mut() {
                grid_instance.apply_transform(&position_delta);
            }
        }
        // Move grids 10pts to the left
        Key::Left => {
            let position_delta = Transform2D {
                translation: pt2(-10.0, 0.0),
                scale: 1.0,
                rotation: 0.0,
            };
            for (_, grid_instance) in model.grids.iter_mut() {
                grid_instance.apply_transform(&position_delta);
            }
        }
        // Rotate grids 90 degrees
        Key::Up => {
            for (_, grid_instance) in model.grids.iter_mut() {
                grid_instance.rotate_in_place(90.0)
            }
        }
        // Rotate grids -90 degrees
        Key::Down => {
            for (_, grid_instance) in model.grids.iter_mut() {
                grid_instance.rotate_in_place(-90.0);
            }
        }
        Key::P => {
            model.debug_flag = !model.debug_flag;
        }
        _ => (),
    }
}

fn update(app: &App, model: &mut Model, _update: Update) {
    let now = Instant::now();
    let duration = now - model.last_update;
    model.last_update = now;
    // FPS calculation
    if model.debug_flag {
        model.fps = 1.0 / duration.as_secs_f32();
    }

    // Update the current model-wide glyph here -- solves timing issues
    if model.needs_glyph_update {
        update_glyph(app, model);
        model.needs_glyph_update = false;
    }

    // Clear the window
    let draw = &model.draw;
    draw.background().color(BLACK);

    // frames processing progress bar:
    if model.exit_requested {
        handle_exit_state(app, model);
        return; // Important: return here to not continue with normal rendering
    }

    // Set background base style
    let bg_style = DrawStyle {
        color: rgb(0.2, 0.2, 0.2),
        stroke_weight: 5.0,
    };

    /*
    let glyph_style = DrawStyle {
        color: rgb(0.7, 0.1, 0.1),
        stroke_weight: 5.0,
    };
    */

    //let start_time = std::time::Instant::now();

    // Loop over each GridInstance and Draw

    /*
    for (_, grid_instance) in model.grids.iter_mut() {
        grid_instance.update(app.time, duration.as_secs_f32());

        let ready_segments =
            grid_instance.get_renderable_segments(app.time, &model.effect_target_style, &bg_style);

        // drawing operations
        if grid_instance.visible {
            grid_instance.draw_segments(draw, ready_segments);
        }

        //let grid_duration = grid_start.elapsed();
        //println!("Grid {} update time: {:?}", name, grid_duration);
    }
    */

    for (_, grid_instance) in model.grids.iter_mut() {
        grid_instance.update(
            &model.effect_target_style,
            &bg_style,
            app.time,
            duration.as_secs_f32(),
        );

        grid_instance.update_background_segments();

        grid_instance.trigger_screen_update(draw);
    }

    if model.debug_flag {
        // Draw (+,+) axes
        let draw = &model.draw;
        draw.line()
            .points(pt2(0.0, 0.0), pt2(50.0, 0.0))
            .color(RED)
            .stroke_weight(1.0);
        draw.line()
            .points(pt2(0.0, 0.0), pt2(0.0, 50.0))
            .color(BLUE)
            .stroke_weight(1.0);

        // Visualize FPS (Optional)
        draw.text(&format!("FPS: {:.1}", model.fps))
            .x_y(1100.0, 290.0)
            .color(RED);
    }

    // Rnder to texture and handle frame recording
    render_and_capture(app, model);

    //let total_duration = start_time.elapsed();
    //println!("Total update time: {:?}", total_duration);
}

// Draw the state of Model into the given Frame
fn view(_app: &App, model: &Model, frame: Frame) {
    //resize texture to screen
    let mut encoder = frame.command_encoder();

    model
        .texture_reshaper
        .encode_render_pass(frame.texture_view(), &mut encoder);
}

// ******************************* State-triggered functions *****************************

fn update_glyph(app: &App, model: &mut Model) {
    let color_hsl = hsl(model.random.gen(), model.random.gen(), 0.4);
    let glyph_style = DrawStyle {
        color: Rgb::from(color_hsl),
        stroke_weight: 5.0,
    };
    model.effect_target_style = glyph_style;

    model.glyphs.update_all_grids(
        &mut model.grids,
        &model.project,
        &model.transition_engine,
        app.time,
    );

    /*
    model
        .glyphs
        .update_all_grids(&mut model.grids, &model.project, app.time);
    */
}

// ******************************* Rendering and Capture *****************************

fn render_and_capture(app: &App, model: &mut Model) {
    let window = app.main_window();
    let device = window.device();
    let ce_desc = wgpu::CommandEncoderDescriptor {
        label: Some("Texture renderer"),
    };
    let mut encoder = device.create_command_encoder(&ce_desc);
    let texture_view = model.texture.view().build();

    model.draw_renderer.encode_render_pass(
        device,
        &mut encoder,
        &model.draw,
        2.0,
        model.texture.size(),
        &texture_view,
        None,
    );

    // Capture the texture for FrameRecorder
    if model.frame_recorder.is_recording() {
        model
            .frame_recorder
            .capture_frame(device, &mut encoder, &model.texture);
    }

    window.queue().submit(Some(encoder.finish()));
    device.poll(wgpu::Maintain::Wait);
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

    let (processed, total) = model.frame_recorder.get_queue_status();

    // Draw progress text
    let text = format!("{} / {}\nframes saved", processed, total);
    draw.text(&text).color(WHITE).font_size(32).x_y(0.0, 50.0);

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
        .x_y(-bar_width / 2.0 + (bar_width * progress) / 2.0, -50.0);
}

fn render_progress(app: &App, model: &mut Model) {
    let window = app.main_window();
    let device = window.device();
    let ce_desc = wgpu::CommandEncoderDescriptor {
        label: Some("Progress renderer"),
    };
    let mut encoder = device.create_command_encoder(&ce_desc);
    let texture_view = model.texture.view().build();

    model.draw_renderer.encode_render_pass(
        device,
        &mut encoder,
        &model.draw,
        2.0,
        model.texture.size(),
        &texture_view,
        None,
    );
    window.queue().submit(Some(encoder.finish()));
}

// ******************************* Debug stuff *******************************

fn make_three_grids(app: &App, model: &mut Model) {
    let grid_1 = GridInstance::new(
        app,
        &model.project,
        "Grid Left".to_string(),
        pt2(-600.0, 0.0),
        0.0,
    );
    let grid_2 = GridInstance::new(
        app,
        &model.project,
        "Grid Center".to_string(),
        pt2(0.0, 0.0),
        0.0,
    );
    let grid_3 = GridInstance::new(
        app,
        &model.project,
        "Grid Right".to_string(),
        pt2(600.0, 0.0),
        0.0,
    );

    model.grids.insert(grid_1.id.clone(), grid_1);
    model.grids.insert(grid_2.id.clone(), grid_2);
    model.grids.insert(grid_3.id.clone(), grid_3);

    for (_, grid) in model.grids.iter() {
        grid.print_grid_info();
    }
}
