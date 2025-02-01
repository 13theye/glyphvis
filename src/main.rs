// src/main.rs
use nannou::prelude::*;
use nannou_osc as osc;
use rand::Rng;
use std::collections::{HashMap, HashSet};
use std::time::Instant;

use glyphvis::{
    animation::{EasingType, MovementEngine, TransitionEngine},
    config::*,
    controllers::GlyphController,
    models::Project,
    services::{FrameRecorder, OutputFormat},
    views::{DrawStyle, GridInstance, Transform2D},
};

// const BPM: u32 = 120;

struct Model {
    // Core components:
    project: Project,
    grids: HashMap<String, GridInstance>, //(grid_id : CachedGrid)
    glyphs: GlyphController,
    receiver: osc::Receiver,

    // Rendering components:
    texture: wgpu::Texture,
    draw: nannou::Draw,
    draw_renderer: nannou::draw::Renderer,
    texture_reshaper: wgpu::TextureReshaper,
    random: rand::rngs::ThreadRng,

    // Time:
    bpm: u32,
    quantize: bool,

    // Segment Transitions
    transition_engine: TransitionEngine,

    // Message
    target_segments: Option<HashSet<String>>,
    immediately_change: bool, // when true, change glyphs w/o transition
    debug_flag: bool,

    // Style
    default_stroke_weight: f32,
    effect_target_style: DrawStyle, // for testing

    // Frame recording:
    frame_recorder: FrameRecorder,
    exit_requested: bool,

    // FPS
    last_update: Instant,
    fps: f32,

    // Config:
    static_config: Config,
}

fn main() {
    nannou::app(model).update(update).run();
}

fn model(app: &App) -> Model {
    // Load config
    let config = Config::load().expect("Failed to load config file");

    // Load project & config
    let project_path = config.resolve_project_path();
    let project = Project::load(project_path).expect("Failed to load project file");

    let glyphs = GlyphController::new(&project);

    // Create window
    let window_id = app
        .new_window()
        .title("glyphvis 0.1.0")
        .size(config.window.width, config.window.height)
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
        .size([
            config.rendering.texture_width,
            config.rendering.texture_height,
        ])
        // Our texture will be used as the RENDER_ATTACHMENT for our `Draw` render pass.
        // It will also be SAMPLED by the `TextureCapturer` and `TextureResizer`.
        .usage(wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING)
        // Use nannou's default multisampling sample count.
        .sample_count(config.rendering.texture_samples)
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

    // Set up BPM
    let bpm = config.speed.bpm;

    let transition_config = TransitionConfig {
        steps: 50,
        frame_duration: 0.1,
        wandering: 1.0,
        density: 0.00001,
    };

    let output_format = OutputFormat::JPEG(config.output.jpeg_quality);

    // Create the frame recorder
    let frame_recorder = FrameRecorder::new(
        device,
        &texture,
        &config.resolve_output_dir_as_str(),
        config.output.frame_limit,
        output_format,
    );

    // Create Osc Receiver
    let receiver = osc::receiver(config.osc.rx_port).expect("osc: Failed to bind to port");

    Model {
        project,
        grids: HashMap::new(), //grid,
        glyphs,
        receiver,

        texture,
        draw,
        draw_renderer,
        texture_reshaper,
        random: rand::thread_rng(),

        bpm,
        quantize: false,

        target_segments: None,
        immediately_change: false,
        debug_flag: false,

        default_stroke_weight: config.style.default_stroke_weight,
        effect_target_style: DrawStyle {
            color: rgb(1.0, 0.0, 0.0),
            stroke_weight: config.style.default_stroke_weight,
        },

        transition_engine: TransitionEngine::new(transition_config),

        frame_recorder,
        exit_requested: false,

        // FPS
        last_update: Instant::now(),
        fps: 0.0,

        // Static config: as loaded from file
        static_config: config,
    }
}

fn key_pressed(app: &App, model: &mut Model, key: Key) {
    match key {
        // show next glyph
        Key::Space => {
            model.target_segments = Some(model.glyphs.next_glyph(&model.project));
            model.immediately_change = false;
        }
        Key::N => {
            model.target_segments = Some(model.glyphs.next_glyph(&model.project));
            model.immediately_change = true;
        }
        // Return grids to where they spawned
        Key::Backslash => {
            for (_, grid_instance) in model.grids.iter_mut() {
                let movement_config = MovementConfig {
                    duration: 0.0,
                    easing: EasingType::EaseOut,
                };
                let movement_engine = MovementEngine::new(movement_config);

                grid_instance.start_movement(0.0, 0.0, &movement_engine);
            }
        }
        Key::C => {
            model.target_segments = Some(model.glyphs.no_glyph());
            model.immediately_change = true;
        }
        Key::X => {
            model.target_segments = Some(model.glyphs.no_glyph());
            model.immediately_change = false;
        }
        // Init grids or hide/show them
        Key::G => {
            if model.grids.is_empty() {
                make_three_grids(app, model);
            } else {
                for (name, grid_instance) in model.grids.iter_mut() {
                    if name != "grid_2" {
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
        // Trigger grid3 animation
        Key::Right => {
            let movement_config = MovementConfig {
                duration: 10.0,
                easing: EasingType::Linear,
            };
            let movement_engine = MovementEngine::new(movement_config);

            if let Some(grid) = model.grids.get_mut("grid_3") {
                grid.start_movement(700.0, 0.0, &movement_engine);
            }
        }
        // Move grids 10pts to the left
        Key::Left => {
            let movement_config = MovementConfig {
                duration: 10.0,
                easing: EasingType::Linear,
            };
            let movement_engine = MovementEngine::new(movement_config);

            if let Some(grid) = model.grids.get_mut("grid_1") {
                grid.start_movement(-700.0, 0.0, &movement_engine);
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

    if model.target_segments.is_some() {
        update_glyph(app, model);
    }

    // Clear the window
    let draw = &model.draw;
    draw.background().color(BLACK);

    // frames processing progress bar:
    if model.exit_requested {
        handle_exit_state(app, model);
        return; // Important: return here to not continue with normal rendering
    }

    // Set grid background base style
    let bg_style = DrawStyle {
        color: rgb(0.1, 0.1, 0.1),
        stroke_weight: model.default_stroke_weight,
    };

    // Main update loop for grids
    for (_, grid_instance) in model.grids.iter_mut() {
        grid_instance.update(
            &model.effect_target_style,
            &bg_style,
            app.time,
            duration.as_secs_f32(),
        );

        grid_instance.update_background_segments(&bg_style, app.time);

        // Send update messages to grid & draw
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

fn update_glyph(_app: &App, model: &mut Model) {
    /*let color_hsl = hsl(
        model.random.gen_range(0.0..=1.0),
        model.random.gen_range(0.2..=1.0),
        0.4,
    );


    let glyph_style = DrawStyle {
        color: Rgb::from(color_hsl),
        stroke_weight: model.default_stroke_weight,
    };
    */

    let glyph_style = DrawStyle {
        color: rgb(1.0, 0.0, 0.0),
        stroke_weight: model.default_stroke_weight,
    };
    model.effect_target_style = glyph_style;

    if let Some(target_segments) = &model.target_segments {
        for grid_instance in model.grids.values_mut() {
            grid_instance.start_transition(
                target_segments,
                &model.transition_engine,
                model.immediately_change,
            );
        }
    }

    model.target_segments = None;
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
        "grid_1".to_string(),
        pt2(0.0, 0.0),
        0.0,
    );
    let grid_2 = GridInstance::new(
        app,
        &model.project,
        "grid_2".to_string(),
        pt2(0.0, 0.0),
        0.0,
    );
    let grid_3 = GridInstance::new(
        app,
        &model.project,
        "grid_3".to_string(),
        pt2(0.0, 0.0),
        0.0,
    );

    model.grids.insert(grid_1.id.clone(), grid_1);
    model.grids.insert(grid_2.id.clone(), grid_2);
    model.grids.insert(grid_3.id.clone(), grid_3);

    for (_, grid) in model.grids.iter() {
        grid.print_grid_info();
    }
}
