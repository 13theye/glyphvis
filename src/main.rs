// src/main.rs
use nannou::prelude::*;
use rand::Rng;
use std::collections::HashMap;
use std::time::Instant;

use glyphvis::{
    animation::{EasingType, MovementEngine, TransitionEngine, TransitionTrigger},
    config::*,
    controllers::{OscCommand, OscController, OscSender},
    effects::FadeEffect,
    models::Project,
    services::{FrameRecorder, OutputFormat},
    views::{BackgroundManager, DrawStyle, GridInstance},
};

struct Model {
    // Core components:
    project: Project,
    grids: HashMap<String, GridInstance>, //(grid_id : CachedGrid)
    background: BackgroundManager,

    // Comms components:
    osc_controller: OscController,
    osc_sender: OscSender,

    // Rendering components:
    texture: wgpu::Texture,
    draw: nannou::Draw,
    draw_renderer: nannou::draw::Renderer,
    texture_reshaper: wgpu::TextureReshaper,
    random: rand::rngs::ThreadRng,

    // Transitions & Animation
    transition_engine: TransitionEngine,

    // Style
    default_stroke_weight: f32,

    // Frame recording:
    frame_recorder: FrameRecorder,
    exit_requested: bool,

    // FPS
    last_update: Instant,
    fps: f32,

    // Message
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

    // Create OSC controller
    let osc_controller =
        OscController::new(config.osc.rx_port).expect("Failed to create OSC Controller");
    let osc_sender = OscSender::new(config.osc.rx_port).expect("Failed to create OSC Sender");

    // Create window
    let window_id = app
        .new_window()
        .title("glyphvis 0.1.7")
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

    let default_transition_config = TransitionConfig {
        steps: config.animation.transition.steps,
        frame_duration: config.animation.transition.frame_duration,
        wandering: config.animation.transition.wandering,
        density: config.animation.transition.density,
    };

    let output_format = OutputFormat::JPEG(config.frame_recorder.jpeg_quality);

    // Create the frame recorder
    let frame_recorder = FrameRecorder::new(
        device,
        &texture,
        &config.resolve_output_dir_as_str(),
        config.frame_recorder.frame_limit,
        output_format,
    );

    Model {
        project,
        grids: HashMap::new(), //grid,
        background: BackgroundManager::default(),
        osc_controller,
        osc_sender,

        texture,
        draw,
        draw_renderer,
        texture_reshaper,
        random: rand::thread_rng(),

        default_stroke_weight: config.style.default_stroke_weight,

        transition_engine: TransitionEngine::new(default_transition_config),

        frame_recorder,
        exit_requested: false,

        // FPS
        last_update: Instant::now(),
        fps: 0.0,

        debug_flag: false,
    }
}

fn key_pressed(_app: &App, model: &mut Model, key: Key) {
    match key {
        // show next glyph
        Key::Space => {
            // Send glyph change for each grid
            for name in model.grids.keys() {
                model.osc_sender.send_next_glyph(name, 0);
            }
        }
        Key::Backslash => {
            // Send glyph change for each grid
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
                model.osc_sender.send_glyph(name, 1, 0);
            }
        }
        Key::Key2 => {
            for name in model.grids.keys() {
                model.osc_sender.send_glyph(name, 2, 0);
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
        Key::RShift => {
            for name in model.grids.keys() {
                if name == "grid_2" {
                    model.osc_sender.send_transition_trigger(name);
                }
            }
        }

        /***************** Below functions aren't implemented in OSC ****************** */
        Key::P => {
            model.debug_flag = !model.debug_flag;
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

    // Process OSC messages
    model.osc_controller.process_messages();
    launch_commands(app, model);

    // Coordinate simulataneous style changes on multiple grids
    coordinate_colorful_grid_styles(app, model);

    // Handle the background
    model.background.draw(&model.draw, app.time);

    // Frames processing progress bar:
    if model.exit_requested {
        handle_exit_state(app, model);
        return; // Important: return here to not continue with normal rendering
    }

    /*********************  Main update method for grids **********************/
    for (_, grid_instance) in model.grids.iter_mut() {
        let dt = duration.as_secs_f32();
        grid_instance.update(&model.draw, &model.transition_engine, app.time, dt);
    }
    /*************************************************************************/

    // Handle FPS and origin display
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

    // Render to texture and handle frame recording
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

// ******************************* OSC Launcher *******************************

fn launch_commands(app: &App, model: &mut Model) {
    for command in model.osc_controller.take_commands() {
        match command {
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
                    grid.init_backbone_effect("backbone", Box::new(effect));
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
                    pt2(position.0, position.1),
                    rotation,
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
                    grid.build_movement(x, y, duration, &movement_engine, app.time);
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
            OscCommand::GridGlyph {
                grid_name,
                glyph_index,
                immediate,
            } => {
                if let Some(grid) = model.grids.get_mut(&grid_name) {
                    grid.stage_glyph_by_index(&model.project, glyph_index);
                    grid.next_glyph_change_is_immediate = immediate;
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
                immediate,
            } => {
                if let Some(grid) = model.grids.get_mut(&grid_name) {
                    grid.stage_next_glyph(&model.project);
                    grid.next_glyph_change_is_immediate = immediate;
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
                immediate,
            } => {
                if let Some(grid) = model.grids.get_mut(&grid_name) {
                    grid.no_glyph();
                    grid.next_glyph_change_is_immediate = immediate;
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
                println!("/grid/setpowereffect has been deprecated in v0.1.7.");
                println!("Please use /grid/seteffect.");
                println!("grid name: {}, setting: {}", grid_name, setting);
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
