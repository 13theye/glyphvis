// src/controllers/osc/mod.rs
// OSC Controller

use nannou_osc as osc;
use std::error::Error;

#[derive(Debug)]
pub enum OscCommand {
    RecorderStart {},
    RecorderStop {},
    GridBackboneFade {
        name: String,
        r: f32,
        g: f32,
        b: f32,
        a: f32,
        duration: f32,
    },
    GridCreate {
        name: String,
        show: String,
        position: (f32, f32),
        rotation: f32,
    },
    GridMove {
        name: String,
        x: f32,
        y: f32,
        duration: f32,
    },
    GridRotate {
        name: String,
        angle: f32,
    },
    GridScale {
        name: String,
        scale: f32,
    },
    GridSlide {
        name: String,
        axis: String,
        number: i32,
        distance: f32,
    },
    BackgroundFlash {
        r: f32,
        g: f32,
        b: f32,
        duration: f32,
    },
    BackgroundColorFade {
        r: f32,
        g: f32,
        b: f32,
        duration: f32,
    },
    GridGlyph {
        grid_name: String,
        glyph_index: usize,
        animation_type_msg: i32,
    },
    GridInstantGlyphColor {
        grid_name: String,
        r: f32,
        g: f32,
        b: f32,
        a: f32,
    },
    GridNextGlyph {
        grid_name: String,
        animation_type_msg: i32,
    },
    GridNextGlyphColor {
        grid_name: String,
        r: f32,
        g: f32,
        b: f32,
        a: f32,
    },
    GridNoGlyph {
        grid_name: String,
        animation_type_msg: i32,
    },
    GridOverwrite {
        grid_name: String,
    },
    GridToggleVisibility {
        grid_name: String,
    },
    GridSetVisibility {
        grid_name: String,
        setting: bool,
    },
    GridToggleColorful {
        grid_name: String,
    },
    GridSetColorful {
        grid_name: String,
        setting: bool,
    },
    GridSetPowerEffect {
        grid_name: String,
        setting: bool,
    },
    GridTransitionTrigger {
        grid_name: String,
    },
    GridTransitionAuto {
        grid_name: String,
    },
    TransitionUpdate {
        grid_name: String,
        steps: Option<usize>,
        frame_duration: Option<f32>,
        wandering: Option<f32>,
        density: Option<f32>,
    },
}

pub struct OscController {
    command_queue: Vec<OscCommand>,
    receiver: osc::Receiver,
}

impl OscController {
    pub fn new(port: u16) -> Result<Self, Box<dyn Error>> {
        let receiver = osc::receiver(port)?;

        Ok(Self {
            command_queue: Vec::new(),
            receiver,
        })
    }

    pub fn process_messages(&mut self) {
        for (packet, _addr) in self.receiver.try_iter() {
            for message in packet.into_msgs() {
                match message.addr.as_str() {
                    "/recorder/start" => {
                        self.command_queue.push(OscCommand::RecorderStart {});
                    }
                    "/recorder/stop" => {
                        self.command_queue.push(OscCommand::RecorderStop {});
                    }
                    "/grid/backbone_fade" => {
                        if let [osc::Type::String(name), osc::Type::Float(r), osc::Type::Float(g), osc::Type::Float(b), osc::Type::Float(a), osc::Type::Float(duration)] =
                            &message.args[..]
                        {
                            self.command_queue.push(OscCommand::GridBackboneFade {
                                name: name.clone(),
                                r: *r,
                                g: *g,
                                b: *b,
                                a: *a,
                                duration: *duration,
                            });
                        }
                    }
                    "/grid/create" => {
                        if let [osc::Type::String(name), osc::Type::String(show), osc::Type::Float(x), osc::Type::Float(y), osc::Type::Float(rot)] =
                            &message.args[..]
                        {
                            self.command_queue.push(OscCommand::GridCreate {
                                name: name.clone(),
                                show: show.clone(),
                                position: (*x, *y),
                                rotation: *rot,
                            });
                        }
                    }
                    "/grid/move" => {
                        if let [osc::Type::String(name), osc::Type::Float(x), osc::Type::Float(y), osc::Type::Float(duration)] =
                            &message.args[..]
                        {
                            self.command_queue.push(OscCommand::GridMove {
                                name: name.clone(),
                                x: *x,
                                y: *y,
                                duration: *duration,
                            });
                        }
                    }
                    "/grid/rotate" => {
                        if let [osc::Type::String(name), osc::Type::Float(angle)] =
                            &message.args[..]
                        {
                            self.command_queue.push(OscCommand::GridRotate {
                                name: name.clone(),
                                angle: *angle,
                            });
                        }
                    }
                    "/grid/scale" => {
                        if let [osc::Type::String(name), osc::Type::Float(scale)] =
                            &message.args[..]
                        {
                            self.command_queue.push(OscCommand::GridScale {
                                name: name.clone(),
                                scale: *scale,
                            });
                        }
                    }
                    "/grid/slide" => {
                        if let [osc::Type::String(name), osc::Type::String(axis), osc::Type::Int(number), osc::Type::Float(distance)] =
                            &message.args[..]
                        {
                            self.command_queue.push(OscCommand::GridSlide {
                                name: name.clone(),
                                axis: axis.clone(),
                                number: *number,
                                distance: *distance,
                            });
                        }
                    }
                    "/background/flash" => {
                        if let [osc::Type::Float(r), osc::Type::Float(g), osc::Type::Float(b), osc::Type::Float(duration)] =
                            &message.args[..]
                        {
                            self.command_queue.push(OscCommand::BackgroundFlash {
                                r: *r,
                                g: *g,
                                b: *b,
                                duration: *duration,
                            });
                        }
                    }
                    "/background/color_fade" => {
                        if let [osc::Type::Float(r), osc::Type::Float(g), osc::Type::Float(b), osc::Type::Float(duration)] =
                            &message.args[..]
                        {
                            self.command_queue.push(OscCommand::BackgroundColorFade {
                                r: *r,
                                g: *g,
                                b: *b,
                                duration: *duration,
                            });
                        }
                    }
                    "/grid/glyph" => {
                        if let [osc::Type::String(name), osc::Type::Int(index), osc::Type::Int(animation_type)] =
                            &message.args[..]
                        {
                            self.command_queue.push(OscCommand::GridGlyph {
                                grid_name: name.clone(),
                                glyph_index: *index as usize,
                                animation_type_msg: *animation_type,
                            });
                        }
                    }
                    "/grid/instantglyphcolor" => {
                        if let [osc::Type::String(name), osc::Type::Float(r), osc::Type::Float(g), osc::Type::Float(b), osc::Type::Float(a)] =
                            &message.args[..]
                        {
                            self.command_queue.push(OscCommand::GridInstantGlyphColor {
                                grid_name: name.clone(),
                                r: *r,
                                g: *g,
                                b: *b,
                                a: *a,
                            });
                        }
                    }
                    "/grid/nextglyph" => {
                        if let [osc::Type::String(name), osc::Type::Int(animation_type)] =
                            &message.args[..]
                        {
                            self.command_queue.push(OscCommand::GridNextGlyph {
                                grid_name: name.clone(),
                                animation_type_msg: *animation_type,
                            });
                        }
                    }
                    "/grid/nextglyphcolor" => {
                        if let [osc::Type::String(name), osc::Type::Float(r), osc::Type::Float(g), osc::Type::Float(b), osc::Type::Float(a)] =
                            &message.args[..]
                        {
                            self.command_queue.push(OscCommand::GridNextGlyphColor {
                                grid_name: name.clone(),
                                r: *r,
                                g: *g,
                                b: *b,
                                a: *a,
                            });
                        }
                    }
                    "/grid/noglyph" => {
                        if let [osc::Type::String(name), osc::Type::Int(animation_type)] =
                            &message.args[..]
                        {
                            self.command_queue.push(OscCommand::GridNoGlyph {
                                grid_name: name.clone(),
                                animation_type_msg: *animation_type,
                            });
                        }
                    }
                    "/grid/overwrite" => {
                        if let [osc::Type::String(name)] = &message.args[..] {
                            self.command_queue.push(OscCommand::GridOverwrite {
                                grid_name: name.clone(),
                            });
                        }
                    }
                    "/grid/transitiontrigger" => {
                        if let [osc::Type::String(name)] = &message.args[..] {
                            self.command_queue.push(OscCommand::GridTransitionTrigger {
                                grid_name: name.clone(),
                            });
                        }
                    }
                    "/grid/transitionauto" => {
                        if let [osc::Type::String(name)] = &message.args[..] {
                            self.command_queue.push(OscCommand::GridTransitionAuto {
                                grid_name: name.clone(),
                            });
                        }
                    }
                    "/grid/togglevisibility" => {
                        if let [osc::Type::String(name)] = &message.args[..] {
                            self.command_queue.push(OscCommand::GridToggleVisibility {
                                grid_name: name.clone(),
                            });
                        }
                    }
                    "/grid/setvisibility" => {
                        if let [osc::Type::String(name), osc::Type::Int(setting)] =
                            &message.args[..]
                        {
                            let setting_bool = *setting != 0;
                            self.command_queue.push(OscCommand::GridSetVisibility {
                                grid_name: name.clone(),
                                setting: setting_bool,
                            });
                        }
                    }
                    "/grid/togglecolorful" => {
                        if let [osc::Type::String(name)] = &message.args[..] {
                            self.command_queue.push(OscCommand::GridToggleColorful {
                                grid_name: name.clone(),
                            });
                        }
                    }
                    "/grid/setcolorful" => {
                        if let [osc::Type::String(name), osc::Type::Int(setting)] =
                            &message.args[..]
                        {
                            let setting_bool = *setting != 0;
                            self.command_queue.push(OscCommand::GridSetColorful {
                                grid_name: name.clone(),
                                setting: setting_bool,
                            });
                        }
                    }
                    "/grid/setpowereffect" => {
                        if let [osc::Type::String(name), osc::Type::Int(setting)] =
                            &message.args[..]
                        {
                            let setting_bool = *setting != 0;
                            self.command_queue.push(OscCommand::GridSetPowerEffect {
                                grid_name: name.clone(),
                                setting: setting_bool,
                            });
                        }
                    }
                    "/transition/update" => {
                        let mut grid_name = String::new();
                        let mut steps = None;
                        let mut frame_duration = None;
                        let mut wandering = None;
                        let mut density = None;

                        for (i, arg) in message.args.iter().enumerate() {
                            match (i, arg) {
                                (0, osc::Type::String(name)) => grid_name = name.clone(),
                                (1, osc::Type::Int(s)) => steps = Some(*s as usize),
                                (2, osc::Type::Float(f)) => frame_duration = Some(*f),
                                (3, osc::Type::Float(w)) => wandering = Some(*w),
                                (4, osc::Type::Float(d)) => density = Some(*d),
                                _ => (),
                            }
                        }

                        self.command_queue.push(OscCommand::TransitionUpdate {
                            grid_name,
                            steps,
                            frame_duration,
                            wandering,
                            density,
                        });
                    }
                    _ => println!("Unknown OSC address pattern: {}", message.addr),
                };
            }
        }
    }

    pub fn take_commands(&mut self) -> Vec<OscCommand> {
        std::mem::take(&mut self.command_queue)
    }
}

// src/osc_control.rs

pub struct OscSender {
    sender: osc::Sender,
    target_addr: String,
    target_port: u16,
}

impl OscSender {
    pub fn new(target_port: u16) -> Result<Self, Box<dyn Error>> {
        let target_addr = "127.0.0.1".to_string();
        let sender = osc::sender()?;

        Ok(Self {
            sender,
            target_addr,
            target_port,
        })
    }

    pub fn send_recorder_start(&self) {
        let addr = "/recorder/start".to_string();
        let args = Vec::new();
        self.sender
            .send((addr, args), (self.target_addr.as_str(), self.target_port))
            .ok();
    }

    pub fn send_recorder_stop(&self) {
        let addr = "/recorder/stop".to_string();
        let args = Vec::new();
        self.sender
            .send((addr, args), (self.target_addr.as_str(), self.target_port))
            .ok();
    }

    pub fn send_create_grid(&self, name: &str, show: &str, x: f32, y: f32, rotation: f32) {
        let addr = "/grid/create".to_string();
        let args = vec![
            osc::Type::String(name.to_string()),
            osc::Type::String(show.to_string()),
            osc::Type::Float(x),
            osc::Type::Float(y),
            osc::Type::Float(rotation),
        ];
        self.sender
            .send((addr, args), (self.target_addr.as_str(), self.target_port))
            .ok();
    }

    pub fn send_move_grid(&self, name: &str, x: f32, y: f32, duration: f32) {
        let addr = "/grid/move".to_string();
        let args = vec![
            osc::Type::String(name.to_string()),
            osc::Type::Float(x),
            osc::Type::Float(y),
            osc::Type::Float(duration),
        ];
        self.sender
            .send((addr, args), (self.target_addr.as_str(), self.target_port))
            .ok();
    }

    pub fn send_rotate_grid(&self, name: &str, angle: f32) {
        let addr = "/grid/rotate".to_string();
        let args = vec![osc::Type::String(name.to_string()), osc::Type::Float(angle)];
        self.sender
            .send((addr, args), (self.target_addr.as_str(), self.target_port))
            .ok();
    }

    pub fn send_scale_grid(&self, name: &str, scale: f32) {
        let addr = "/grid/scale".to_string();
        let args = vec![osc::Type::String(name.to_string()), osc::Type::Float(scale)];
        self.sender
            .send((addr, args), (self.target_addr.as_str(), self.target_port))
            .ok();
    }

    pub fn send_grid_slide(&self, name: &str, axis: &str, number: i32, distance: f32) {
        let addr = "/grid/slide".to_string();
        let args = vec![
            osc::Type::String(name.to_string()),
            osc::Type::String(axis.to_string()),
            osc::Type::Int(number),
            osc::Type::Float(distance),
        ];
        self.sender
            .send((addr, args), (self.target_addr.as_str(), self.target_port))
            .ok();
    }

    pub fn send_grid_backbone_fade(
        &self,
        grid_name: &str,
        r: f32,
        g: f32,
        b: f32,
        a: f32,
        duration: f32,
    ) {
        let addr = "/grid/backbone_fade".to_string();
        let args = vec![
            osc::Type::String(grid_name.to_string()),
            osc::Type::Float(r),
            osc::Type::Float(g),
            osc::Type::Float(b),
            osc::Type::Float(a),
            osc::Type::Float(duration),
        ];
        self.sender
            .send((addr, args), (self.target_addr.as_str(), self.target_port))
            .ok();
    }

    pub fn send_glyph(&self, grid_name: &str, index: i32, animation_type_msg: i32) {
        let addr = "/grid/glyph".to_string();
        let args = vec![
            osc::Type::String(grid_name.to_string()),
            osc::Type::Int(index),
            osc::Type::Int(animation_type_msg),
        ];
        self.sender
            .send((addr, args), (self.target_addr.as_str(), self.target_port))
            .ok();
    }

    pub fn send_next_glyph(&self, grid_name: &str, animation_type_msg: i32) {
        let addr = "/grid/nextglyph".to_string();
        let args = vec![
            osc::Type::String(grid_name.to_string()),
            osc::Type::Int(animation_type_msg),
        ];
        self.sender
            .send((addr, args), (self.target_addr.as_str(), self.target_port))
            .ok();
    }
    pub fn send_instant_glyph_color(&self, grid_name: &str, r: f32, g: f32, b: f32, a: f32) {
        let addr = "/grid/instantglyphcolor".to_string();
        let args = vec![
            osc::Type::String(grid_name.to_string()),
            osc::Type::Float(r),
            osc::Type::Float(g),
            osc::Type::Float(b),
            osc::Type::Float(a),
        ];
        self.sender
            .send((addr, args), (self.target_addr.as_str(), self.target_port))
            .ok();
    }
    pub fn send_next_glyph_color(&self, grid_name: &str, r: f32, g: f32, b: f32, a: f32) {
        let addr = "/grid/nextglyphcolor".to_string();
        let args = vec![
            osc::Type::String(grid_name.to_string()),
            osc::Type::Float(r),
            osc::Type::Float(g),
            osc::Type::Float(b),
            osc::Type::Float(a),
        ];
        self.sender
            .send((addr, args), (self.target_addr.as_str(), self.target_port))
            .ok();
    }
    pub fn send_no_glyph(&self, grid_name: &str, animation_type_msg: i32) {
        let addr = "/grid/noglyph".to_string();
        let args = vec![
            osc::Type::String(grid_name.to_string()),
            osc::Type::Int(animation_type_msg),
        ];
        self.sender
            .send((addr, args), (self.target_addr.as_str(), self.target_port))
            .ok();
    }
    pub fn send_grid_overwrite(&self, grid_name: &str) {
        let addr = "/grid/overwrite".to_string();
        let args = vec![osc::Type::String(grid_name.to_string())];
        self.sender
            .send((addr, args), (self.target_addr.as_str(), self.target_port))
            .ok();
    }

    pub fn send_transition_trigger(&self, grid_name: &str) {
        let addr = "/grid/transitiontrigger".to_string();
        let args = vec![osc::Type::String(grid_name.to_string())];
        self.sender
            .send((addr, args), (self.target_addr.as_str(), self.target_port))
            .ok();
    }

    pub fn send_transition_auto(&self, grid_name: &str) {
        let addr = "/grid/transitionauto".to_string();
        let args = vec![osc::Type::String(grid_name.to_string())];
        self.sender
            .send((addr, args), (self.target_addr.as_str(), self.target_port))
            .ok();
    }

    pub fn send_toggle_visibility(&self, grid_name: &str) {
        let addr = "/grid/togglevisibility".to_string();
        let args = vec![osc::Type::String(grid_name.to_string())];
        self.sender
            .send((addr, args), (self.target_addr.as_str(), self.target_port))
            .ok();
    }
    pub fn send_toggle_colorful(&self, grid_name: &str) {
        let addr = "/grid/togglecolorful".to_string();
        let args = vec![osc::Type::String(grid_name.to_string())];
        self.sender
            .send((addr, args), (self.target_addr.as_str(), self.target_port))
            .ok();
    }
    pub fn send_set_power_effect(&self, grid_name: &str, setting: i32) {
        let addr = "/grid/setpowereffect".to_string();
        let args = vec![
            osc::Type::String(grid_name.to_string()),
            osc::Type::Int(setting),
        ];
        self.sender
            .send((addr, args), (self.target_addr.as_str(), self.target_port))
            .ok();
    }
    pub fn send_background_flash(&self, r: f32, g: f32, b: f32, duration: f32) {
        let addr = "/background/flash".to_string();
        let args = vec![
            osc::Type::Float(r),
            osc::Type::Float(g),
            osc::Type::Float(b),
            osc::Type::Float(duration),
        ];
        self.sender
            .send((addr, args), (self.target_addr.as_str(), self.target_port))
            .ok();
    }
    pub fn send_background_color_fade(&self, r: f32, g: f32, b: f32, duration: f32) {
        let addr = "/background/color_fade".to_string();
        let args = vec![
            osc::Type::Float(r),
            osc::Type::Float(g),
            osc::Type::Float(b),
            osc::Type::Float(duration),
        ];
        self.sender
            .send((addr, args), (self.target_addr.as_str(), self.target_port))
            .ok();
    }
    pub fn send_update_transition_config(
        &self,
        grid_name: &str,
        steps: Option<usize>,
        frame_duration: Option<f32>,
        wandering: Option<f32>,
        density: Option<f32>,
    ) {
        let addr = "/transition/update".to_string();
        let mut args = vec![osc::Type::String(grid_name.to_string())];

        // Only add the values that are Some
        if let Some(s) = steps {
            args.push(osc::Type::Int(s as i32));
        }
        if let Some(f) = frame_duration {
            args.push(osc::Type::Float(f));
        }
        if let Some(w) = wandering {
            args.push(osc::Type::Float(w));
        }
        if let Some(d) = density {
            args.push(osc::Type::Float(d));
        }

        self.sender
            .send((addr, args), (self.target_addr.as_str(), self.target_port))
            .ok();
    }
}
