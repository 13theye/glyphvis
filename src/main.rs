// src/main.rs
use nannou::prelude::*;
use rand::Rng;
use std::{
    collections::HashMap,
    io::{self, Write},
    rc::Rc,
    time::Instant,
};

use glyphvis::{
    animation::{
        EasingType, MovementEngine, TransitionAnimationType, TransitionEngine,
        TransitionTriggerType,
    },
    config::*,
    controllers::{OscCommand, OscController, OscSender},
    effects::FadeEffect,
    models::{Axis, Project},
    services::{FrameRecorder, SegmentGraph},
    views::{BackgroundManager, CachedGrid, DrawStyle, GridInstance},
};

struct Model {
    // Data from the Project file including all Glyph definitions
    project: Project,

    // Grids are the primary logical units that get rendered. A grid is a virtual segmented display for Hangeul characters.
    // By lighting up sets of segments, different characters are displayed.
    //
    // The CachedGrid is the generic grid structure.
    // Currently, one project holds a single grid type. The draw instructions are held in Model
    // as a CachedGrid. This helps avoid redundant calculations when GridInstances are created.
    base_grid: CachedGrid,

    // The Graph is the network of connections between segments. This is shared among Grids
    // of the same type as it is read-only.
    base_graph: Rc<SegmentGraph>,

    // A GridInstance manages the state of an individual grid and sends commands to its internal segments to turn on or off,
    // or display different colors.
    //
    // When a GridInstance is created, a Show from the Project file is attached. The GridInstance is hidden by default until it receives a command
    // to be shown. A GridInstance cannot be destroyed once created.
    grids: HashMap<String, GridInstance>, //(grid_id : GridInstance)

    // BackgroundManager handles Background color state
    background: BackgroundManager,

    // Handle to API that builds segment commands defining animation sequences between Glyphs.
    transition_engine: TransitionEngine,

    // OSC Comms components:
    // OscController checks incoming OSC commands for validity and maintains a queue holding
    // all commands received between updates.
    osc_controller: OscController,

    // Keyboard commands (with a few exceptions) use the internal OSC sender to execute commands.
    osc_sender: OscSender,

    // Rendering components:
    //
    // The full-resolution texture that is drawn every frame
    texture: wgpu::Texture,

    // Nannou API
    draw: nannou::Draw,
    draw_renderer: nannou::draw::Renderer,

    // The reshaper is used to resize the texture for the screen monitor display
    texture_reshaper: wgpu::TextureReshaper,

    // A random number generator
    random: rand::rngs::ThreadRng,

    // Segment default style as stored in config.toml
    // Need it here to pass into GridInstance when a Grid is created.
    default_stroke_weight: f32,
    default_backbone_stroke_weight: f32,

    // Frame recorder service saves JPGs of full resolution textures at 30fps
    frame_recorder: FrameRecorder,

    // Tracks if a Quit command has been issued, for a graceful exit that waits
    // for all queued framees to finish saving before halting the program
    exit_requested: bool,

    // FPS
    last_update: Instant,
    fps: f32,
    fps_update_interval: f32,
    last_fps_display_update: f32,
    frame_count: u32,
    frame_time_accumulator: f32,

    // When on, displays more verbose messages in the terminal
    debug_flag: bool,
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

    // Cache grid draw instructions and the segment graph
    let base_grid = CachedGrid::new(&project);
    let base_graph = Rc::new(SegmentGraph::new(&base_grid));

    // Create OSC controller
    let osc_controller =
        OscController::new(config.osc.rx_port).expect("Failed to create OSC Controller");
    let osc_sender = OscSender::new(config.osc.rx_port).expect("Failed to create OSC Sender");

    // Create window
    let window_id = app
        .new_window()
        .title("glyphvis 0.3.4b")
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
        // Use a spacious 16-bit linear sRGBA format suitable for high quality drawing: Rgba16Float
        // Use 8-bit for standard quality and better perforamnce: Rgba8Unorm Rgb10a2Unorm
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

    let default_transition_config = TransitionConfig {
        steps: config.animation.transition.steps,
        frame_duration: config.animation.transition.frame_duration,
        wandering: config.animation.transition.wandering,
        density: config.animation.transition.density,
    };

    let recorder_fps = config.frame_recorder.fps;

    // Create the frame recorder
    let frame_recorder = FrameRecorder::new(
        device,
        &texture,
        &config.resolve_output_dir_as_str(),
        config.frame_recorder.frame_limit,
        recorder_fps,
    );

    Model {
        project,
        base_grid,
        base_graph,

        grids: HashMap::new(), //grid,
        transition_engine: TransitionEngine::new(default_transition_config),
        background: BackgroundManager::default(),

        osc_controller,
        osc_sender,

        texture,
        draw,
        draw_renderer,
        texture_reshaper,
        random: rand::thread_rng(),

        default_stroke_weight: config.style.default_stroke_weight,
        default_backbone_stroke_weight: config.style.default_backbone_stroke_weight,

        frame_recorder,
        exit_requested: false,

        // FPS
        last_update: Instant::now(),
        fps: 0.0,
        fps_update_interval: 0.3,
        last_fps_display_update: 0.0,
        frame_count: 0,
        frame_time_accumulator: 0.0,

        debug_flag: false,
    }
}

fn update(app: &App, model: &mut Model, _update: Update) {
    let now = Instant::now();
    let duration = now - model.last_update;
    let dt = duration.as_secs_f32();
    model.last_update = now;

    // FPS calculations
    if model.debug_flag {
        calculate_fps(app, model, dt);
    }

    // Process OSC messages
    model.osc_controller.process_messages();
    launch_commands(app, model);

    // Coordinate simulataneous style changes on multiple grids
    coordinate_colorful_grid_styles(app, model);

    // Handle the background
    model.background.draw(&model.draw, app.time);

    // Clean up any completed recording threads
    model.frame_recorder.cleanup_completed_worker();

    // Frames processing progress bar:
    if model.exit_requested {
        handle_exit_state(app, model);
        return; // Important: return here to not continue with normal rendering
    }

    /*********************  Main update method for grids **********************/
    for (_, grid_instance) in model.grids.iter_mut() {
        grid_instance.update(&model.draw, &model.transition_engine, app.time, dt);
    }

    // Handle FPS and origin display
    if model.debug_flag {
        draw_fps(model);
    }

    // Render to texture and handle frame recording
    render_and_capture(app, model);

    // For benchmarking:
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

// ************************ FPS and debug display  *************************************

fn draw_fps(model: &Model) {
    let draw = &model.draw;
    // Draw (+,+) axes
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

fn init_fps(app: &App, model: &mut Model) {
    model.fps = 0.0;
    model.frame_count = 0;
    model.frame_time_accumulator = 0.0;
    model.last_fps_display_update = app.time;
}

fn calculate_fps(app: &App, model: &mut Model, dt: f32) {
    model.frame_count += 1;
    model.frame_time_accumulator += dt;
    let elapsed_since_last_fps_update = app.time - model.last_fps_display_update;
    if elapsed_since_last_fps_update >= model.fps_update_interval {
        if model.frame_count > 0 {
            let avg_frame_time = model.frame_time_accumulator / model.frame_count as f32;
            model.fps = if avg_frame_time > 0.0 {
                1.0 / avg_frame_time
            } else {
                0.0
            };
        }

        // Reset accumulators
        model.frame_count = 0;
        model.frame_time_accumulator = 0.0;
        model.last_fps_display_update = app.time;
    }
}

// ************************ Multi-grid style coordination  *****************************

fn coordinate_colorful_grid_styles(_app: &App, model: &mut Model) {
    let color_hsl = hsla(
        model.random.gen_range(0.0..=1.0),
        model.random.gen_range(0.2..=1.0),
        0.4,
        1.0,
    );

    let color = Rgba::from(color_hsl);

    for grid_instance in model.grids.values_mut() {
        if grid_instance.has_target_segments() && grid_instance.colorful_flag {
            grid_instance.set_effect_target_style(DrawStyle {
                color,
                // account for any grid scaling
                stroke_weight: model.default_stroke_weight * grid_instance.current_scale,
            });
        }
    }
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
        // Show progress information to the user
        print!(".");
        io::stdout().flush().unwrap();
        let (_, total) = model.frame_recorder.get_queue_status();
        if total > 0 {
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
    } else {
        // Worker thread has completed - safe to quit
        println!("Video processing complete.");
        app.quit();
    }
}

// ******************************* Keyboard Input *******************************

fn key_pressed(app: &App, model: &mut Model, key: Key) {
    match key {
        // show next glyph
        Key::Space => {
            // Send glyph change for each grid
            for name in model.grids.keys() {
                model.osc_sender.send_next_glyph(name, 2);
            }
        }
        Key::Backslash => {
            // Move to original position
            for name in model.grids.keys() {
                model.osc_sender.send_move_grid(name, 0.0, 0.0, 0.0)
            }
        }

        Key::N => {
            for (name, _) in model.grids.iter() {
                model.osc_sender.send_next_glyph(name, 1);
            }
        }
        Key::C => {
            for (name, _) in model.grids.iter() {
                model.osc_sender.send_no_glyph(name, 1);
            }
        }
        Key::X => {
            for (name, _) in model.grids.iter() {
                model.osc_sender.send_no_glyph(name, 0);
            }
        }
        Key::Key1 => {
            for name in model.grids.keys() {
                model.osc_sender.send_grid_overwrite(name);
            }
        }
        Key::Key2 => {
            for name in model.grids.keys() {
                model.osc_sender.send_glyph(name, 2, 0);
            }
        }
        Key::Key3 => {
            for name in model.grids.keys() {
                model.osc_sender.send_grid_slide(name, "y", 2, 50.0);
            }
        }
        Key::Key4 => {
            for name in model.grids.keys() {
                model.osc_sender.send_grid_slide(name, "y", 2, -50.0);
            }
        }
        Key::Key5 => {
            for name in model.grids.keys() {
                model.osc_sender.send_grid_slide(name, "x", 2, 50.0);
            }
        }
        Key::Key6 => {
            for name in model.grids.keys() {
                model.osc_sender.send_grid_slide(name, "x", 2, -50.0);
            }
        }
        Key::Key9 => {
            for name in model.grids.keys() {
                model
                    .osc_sender
                    .send_grid_backbone_fade(name, 0.19, 0.19, 0.19, 0.0, 0.0);
            }
        }
        Key::Key0 => {
            for name in model.grids.keys() {
                model
                    .osc_sender
                    .send_grid_backbone_fade(name, 0.19, 0.19, 0.19, 1.0, 3.0);
            }
        }
        Key::G => {
            if model.grids.is_empty() {
                // Create three test grids via OSC
                model
                    .osc_sender
                    .send_create_grid("grid_1", "wesa", 0.0, 0.0, 0.0);
                model
                    .osc_sender
                    .send_create_grid("grid_2", "wesa", 0.0, 0.0, 0.0);
                model
                    .osc_sender
                    .send_create_grid("grid_3", "wesa", 0.0, 0.0, 0.0);
                model.osc_sender.send_toggle_visibility("grid_1");
                model.osc_sender.send_toggle_visibility("grid_2");
                model.osc_sender.send_toggle_visibility("grid_3");
            } else {
                // Toggle visibility (you might want to add an OSC command for this)
                for name in model.grids.keys() {
                    model.osc_sender.send_toggle_visibility(name);
                }
            }
        }
        Key::E => {
            for (name, _) in model.grids.iter() {
                model.osc_sender.send_set_power_effect(name, 1);
            }
        }
        Key::D => {
            for name in model.grids.keys() {
                if name == "grid_2" {
                    model.osc_sender.send_grid_backbone_stroke(name, 10.0);
                }
            }
        }
        Key::H => {
            for name in model.grids.keys() {
                if name != "grid_2" {
                    model.osc_sender.send_toggle_colorful(name);
                }
            }
        }
        Key::I => {
            for name in model.grids.keys() {
                if name != "grid_2" {
                    model.osc_sender.send_instant_glyph_color(
                        name,
                        model.random.gen(),
                        model.random.gen(),
                        model.random.gen(),
                        1.0,
                    );
                }
            }
        }
        Key::J => {
            for name in model.grids.keys() {
                if name != "grid_2" {
                    model
                        .osc_sender
                        .send_next_glyph_color(name, 0.82, 0.0, 0.14, 1.0);
                }
            }
        }
        Key::B => {
            model.osc_sender.send_background_flash(1.0, 1.0, 1.0, 0.1);
        }
        Key::V => {
            model
                .osc_sender
                .send_background_color_fade(1.0, 0.0, 0.0, 10.0);
        }
        Key::M => {
            model
                .osc_sender
                .send_background_color_fade(0.0, 0.0, 0.0, 10.0);
        }
        Key::Comma => {
            model
                .osc_sender
                .send_background_color_fade(0.6, 0.2, 0.5, 10.0);
        }
        Key::Right => {
            model.osc_sender.send_move_grid("grid_3", 700.0, 0.0, 3.0);
        }
        Key::Left => {
            model.osc_sender.send_move_grid("grid_1", -700.0, 0.0, 3.0);
        }
        Key::Up => {
            for name in model.grids.keys() {
                model.osc_sender.send_scale_grid(name, 0.2);
            }
        }
        Key::Down => {
            for name in model.grids.keys() {
                model.osc_sender.send_scale_grid(name, 1.0);
            }
        }
        Key::T => {
            for name in model.grids.keys() {
                model.osc_sender.send_rotate_grid(name, 5.0);
            }
        }
        Key::Y => {
            for name in model.grids.keys() {
                model.osc_sender.send_rotate_grid(name, -5.0);
            }
        }
        Key::Z => {
            for grid_instance in model.grids.values_mut() {
                grid_instance.boundary_test(Axis::X);
            }
        }
        Key::RShift => {
            for name in model.grids.keys() {
                if name == "grid_2" {
                    model.osc_sender.send_transition_trigger(name);
                }
            }
        }
        Key::LShift => {
            for name in model.grids.keys() {
                if name == "grid_2" {
                    model.osc_sender.send_transition_auto(name);
                }
            }
        }

        Key::R => {
            if !model.frame_recorder.is_recording() {
                model.osc_sender.send_recorder_start();
            } else {
                model.osc_sender.send_recorder_stop();
            }
        }
        /***************** Below functions aren't implemented in OSC ****************** */
        Key::P => {
            model.debug_flag = !model.debug_flag;
            init_fps(app, model);
        }
        // Graceful quit that waits for frame queue to be processed
        Key::Q => {
            model.frame_recorder.signal_shutdown();
            model.exit_requested = true;
            println!("\nShutdown requested.");
            println!("Waiting for any recording threads to finish...")
        }
        _ => (),
    }
}

// ******************************* OSC Launcher *******************************

fn launch_commands(app: &App, model: &mut Model) {
    for command in model.osc_controller.take_commands() {
        match command {
            OscCommand::RecorderStart {} => {
                if !model.frame_recorder.is_recording() {
                    model.frame_recorder.toggle_recording();
                }
            }
            OscCommand::RecorderStop {} => {
                if model.frame_recorder.is_recording() {
                    model.frame_recorder.toggle_recording();
                }
            }
            OscCommand::BackgroundFlash { r, g, b, duration } => {
                model.background.flash(rgb(r, g, b), duration, app.time);
            }
            OscCommand::BackgroundColorFade { r, g, b, duration } => {
                model
                    .background
                    .color_fade(rgb(r, g, b), duration, app.time);
            }
            OscCommand::GridBackboneFade {
                name,
                r,
                g,
                b,
                a,
                duration,
            } => {
                if let Some(grid) = model.grids.get_mut(&name) {
                    let effect = FadeEffect {
                        base_style: grid.backbone_style.clone(),
                        target_style: DrawStyle {
                            color: rgba(r, g, b, a),
                            stroke_weight: grid.backbone_style.stroke_weight,
                        },
                        duration,
                        start_time: app.time,
                        is_active: true,
                    };
                    grid.add_backbone_effect("backbone", Box::new(effect));
                }
            }
            OscCommand::GridBackboneStroke {
                name,
                stroke_weight,
            } => {
                if let Some(grid) = model.grids.get_mut(&name) {
                    grid.set_backbone_stroke_weight(stroke_weight);
                }
            }
            OscCommand::GridCreate {
                name,
                show,
                position,
                rotation,
            } => {
                let grid = GridInstance::new(
                    name.clone(),
                    &model.project,
                    &show,
                    &model.base_grid,
                    Rc::clone(&model.base_graph),
                    pt2(position.0, position.1),
                    rotation,
                    model.default_stroke_weight,
                    model.default_backbone_stroke_weight,
                );
                model.grids.insert(name, grid);
            }

            OscCommand::GridMove {
                name,
                x,
                y,
                duration,
            } => {
                if let Some(grid) = model.grids.get_mut(&name) {
                    let movement_config = MovementConfig {
                        duration,
                        easing: EasingType::Linear,
                    };
                    let movement_engine = MovementEngine::new(movement_config);
                    grid.active_movement = None;
                    grid.stage_movement(x, y, duration, &movement_engine, app.time);
                }
            }
            OscCommand::GridRotate { name, angle } => {
                if let Some(grid) = model.grids.get_mut(&name) {
                    grid.rotate_in_place(angle);
                }
            }
            OscCommand::GridScale { name, scale } => {
                if let Some(grid) = model.grids.get_mut(&name) {
                    grid.scale_in_place(scale);
                }
            }
            OscCommand::GridSlide {
                name,
                axis,
                number,
                position,
            } => {
                if let Some(grid) = model.grids.get_mut(&name) {
                    let axis_validated = match Axis::try_from(axis.as_str()) {
                        Ok(axis) => axis,
                        Err(err) => {
                            println!("{}", err);
                            return;
                        }
                    };

                    grid.slide(axis_validated, number, position, app.time);
                }
            }
            OscCommand::GridGlyph {
                grid_name,
                glyph_index,
                animation_type_msg,
            } => {
                if let Some(grid) = model.grids.get_mut(&grid_name) {
                    grid.stage_glyph_by_index(&model.project, glyph_index);
                    grid.transition_next_animation_type =
                        transition_next_animation_type(animation_type_msg);
                }
            }
            OscCommand::GridInstantGlyphColor {
                grid_name,
                r,
                g,
                b,
                a,
            } => {
                if let Some(grid) = model.grids.get_mut(&grid_name) {
                    grid.instant_color_change(rgba(r, g, b, a));
                }
            }
            OscCommand::GridNextGlyph {
                grid_name,
                animation_type_msg,
            } => {
                if let Some(grid) = model.grids.get_mut(&grid_name) {
                    grid.stage_next_glyph(&model.project);
                    grid.transition_next_animation_type =
                        transition_next_animation_type(animation_type_msg);
                }
            }
            OscCommand::GridNextGlyphColor {
                grid_name,
                r,
                g,
                b,
                a,
            } => {
                if let Some(grid) = model.grids.get_mut(&grid_name) {
                    let style = DrawStyle {
                        color: rgba(r, g, b, a),
                        stroke_weight: model.default_stroke_weight * grid.current_scale,
                    };
                    grid.set_effect_target_style(style);
                }
            }
            OscCommand::GridNoGlyph {
                grid_name,
                animation_type_msg,
            } => {
                if let Some(grid) = model.grids.get_mut(&grid_name) {
                    grid.stage_empty_glyph();
                    grid.transition_next_animation_type =
                        transition_next_animation_type(animation_type_msg);
                }
            }
            OscCommand::GridOverwrite { grid_name } => {
                if let Some(grid) = model.grids.get_mut(&grid_name) {
                    let index = grid.current_glyph_index;
                    grid.use_power_on_effect = true;
                    grid.stage_glyph_by_index(&model.project, index);
                    grid.transition_next_animation_type = TransitionAnimationType::Overwrite;
                }
            }
            OscCommand::GridToggleVisibility { grid_name } => {
                if let Some(grid) = model.grids.get_mut(&grid_name) {
                    grid.is_visible = !grid.is_visible;
                }
            }
            OscCommand::GridTransitionTrigger { grid_name } => {
                if let Some(grid) = model.grids.get_mut(&grid_name) {
                    grid.receive_transition_trigger();
                }
            }
            OscCommand::GridTransitionAuto { grid_name } => {
                if let Some(grid) = model.grids.get_mut(&grid_name) {
                    grid.transition_trigger_type = TransitionTriggerType::Auto;
                }
            }
            OscCommand::GridSetVisibility { grid_name, setting } => {
                if let Some(grid) = model.grids.get_mut(&grid_name) {
                    grid.is_visible = setting;
                }
            }
            OscCommand::GridToggleColorful { grid_name } => {
                if let Some(grid) = model.grids.get_mut(&grid_name) {
                    grid.colorful_flag = !grid.colorful_flag;
                }
            }
            OscCommand::GridSetColorful { grid_name, setting } => {
                if let Some(grid) = model.grids.get_mut(&grid_name) {
                    grid.colorful_flag = setting;
                }
            }
            OscCommand::GridSetPowerEffect { grid_name, setting } => {
                if let Some(grid) = model.grids.get_mut(&grid_name) {
                    grid.use_power_on_effect = setting;
                }
            }
            OscCommand::TransitionUpdate {
                grid_name,
                steps,
                frame_duration,
                wandering,
                density,
            } => {
                if let Some(grid) = model.grids.get_mut(&grid_name) {
                    grid.update_transition_config(
                        steps,
                        frame_duration,
                        wandering,
                        density,
                        model.transition_engine.get_default_config(),
                    );
                }
            }
        }
    }
}

fn transition_next_animation_type(msg: i32) -> TransitionAnimationType {
    match msg {
        0 => TransitionAnimationType::Random,
        1 => TransitionAnimationType::Immediate,
        2 => TransitionAnimationType::Writing,
        3 => TransitionAnimationType::Overwrite,
        _ => TransitionAnimationType::Immediate,
    }
}
