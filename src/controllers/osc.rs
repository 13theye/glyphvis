// src/controllers/osc/mod.rs
// OSC Controller

use nannou_osc as osc;
use std::error::Error;

#[derive(Debug)]
pub enum OscCommand {
    BackboneColorFade {
        name: String,
        r: f32,
        g: f32,
        b: f32,
        a: f32,
        duration: f32,
    },
    CreateGrid {
        name: String,
        show: String,
        position: (f32, f32),
        rotation: f32,
    },
    MoveGrid {
        name: String,
        x: f32,
        y: f32,
        duration: f32,
    },
    RotateGrid {
        name: String,
        angle: f32,
    },
    ScaleGrid {
        name: String,
        scale: f32,
    },
    FlashBackground {
        r: f32,
        g: f32,
        b: f32,
        duration: f32,
    },
    ColorFadeBackground {
        r: f32,
        g: f32,
        b: f32,
        duration: f32,
    },
    DisplayGlyph {
        grid_name: String,
        glyph_index: usize,
        immediate: bool,
    },
    InstantGlyphColor {
        grid_name: String,
        r: f32,
        g: f32,
        b: f32,
        a: f32,
    },
    NextGlyph {
        grid_name: String,
        immediate: bool,
    },
    NextGlyphColor {
        grid_name: String,
        r: f32,
        g: f32,
        b: f32,
        a: f32,
    },
    NoGlyph {
        grid_name: String,
        immediate: bool,
    },
    ToggleVisibility {
        grid_name: String,
    },
    SetVisibility {
        grid_name: String,
        setting: bool,
    },
    ToggleColorful {
        grid_name: String,
    },
    SetColorful {
        grid_name: String,
        setting: bool,
    },
    UpdateTransitionConfig {
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
                    "/grid/backbone_fade" => {
                        if let [osc::Type::String(name), osc::Type::Float(r), osc::Type::Float(g), osc::Type::Float(b), osc::Type::Float(a), osc::Type::Float(duration)] =
                            &message.args[..]
                        {
                            self.command_queue.push(OscCommand::BackboneColorFade {
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
                            self.command_queue.push(OscCommand::CreateGrid {
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
                            self.command_queue.push(OscCommand::MoveGrid {
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
                            self.command_queue.push(OscCommand::RotateGrid {
                                name: name.clone(),
                                angle: *angle,
                            });
                        }
                    }
                    "/grid/scale" => {
                        if let [osc::Type::String(name), osc::Type::Float(scale)] =
                            &message.args[..]
                        {
                            self.command_queue.push(OscCommand::ScaleGrid {
                                name: name.clone(),
                                scale: *scale,
                            });
                        }
                    }
                    "/background/flash" => {
                        if let [osc::Type::Float(r), osc::Type::Float(g), osc::Type::Float(b), osc::Type::Float(duration)] =
                            &message.args[..]
                        {
                            self.command_queue.push(OscCommand::FlashBackground {
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
                            self.command_queue.push(OscCommand::ColorFadeBackground {
                                r: *r,
                                g: *g,
                                b: *b,
                                duration: *duration,
                            });
                        }
                    }
                    "/grid/glyph" => {
                        if let [osc::Type::String(name), osc::Type::Int(index), osc::Type::Int(immediately)] =
                            &message.args[..]
                        {
                            let immediate = *immediately != 0;
                            self.command_queue.push(OscCommand::DisplayGlyph {
                                grid_name: name.clone(),
                                glyph_index: *index as usize,
                                immediate,
                            });
                        }
                    }
                    "/grid/instantglyphcolor" => {
                        if let [osc::Type::String(name), osc::Type::Float(r), osc::Type::Float(g), osc::Type::Float(b), osc::Type::Float(a)] =
                            &message.args[..]
                        {
                            self.command_queue.push(OscCommand::InstantGlyphColor {
                                grid_name: name.clone(),
                                r: *r,
                                g: *g,
                                b: *b,
                                a: *a,
                            });
                        }
                    }
                    "/grid/nextglyph" => {
                        if let [osc::Type::String(name), osc::Type::Int(immediately)] =
                            &message.args[..]
                        {
                            let immediate = *immediately != 0;
                            self.command_queue.push(OscCommand::NextGlyph {
                                grid_name: name.clone(),
                                immediate,
                            });
                        }
                    }
                    "/grid/nextglyphcolor" => {
                        if let [osc::Type::String(name), osc::Type::Float(r), osc::Type::Float(g), osc::Type::Float(b), osc::Type::Float(a)] =
                            &message.args[..]
                        {
                            self.command_queue.push(OscCommand::NextGlyphColor {
                                grid_name: name.clone(),
                                r: *r,
                                g: *g,
                                b: *b,
                                a: *a,
                            });
                        }
                    }
                    "/grid/noglyph" => {
                        if let [osc::Type::String(name), osc::Type::Int(immediately)] =
                            &message.args[..]
                        {
                            let immediate = *immediately != 0;
                            self.command_queue.push(OscCommand::NoGlyph {
                                grid_name: name.clone(),
                                immediate,
                            });
                        }
                    }
                    "/grid/togglevisibility" => {
                        if let [osc::Type::String(name)] = &message.args[..] {
                            self.command_queue.push(OscCommand::ToggleVisibility {
                                grid_name: name.clone(),
                            });
                        }
                    }
                    "/grid/setvisibility" => {
                        if let [osc::Type::String(name), osc::Type::Int(setting)] =
                            &message.args[..]
                        {
                            let setting_bool = *setting != 0;
                            self.command_queue.push(OscCommand::SetVisibility {
                                grid_name: name.clone(),
                                setting: setting_bool,
                            });
                        }
                    }
                    "/grid/togglecolorful" => {
                        if let [osc::Type::String(name)] = &message.args[..] {
                            self.command_queue.push(OscCommand::ToggleColorful {
                                grid_name: name.clone(),
                            });
                        }
                    }
                    "/grid/setcolorful" => {
                        if let [osc::Type::String(name), osc::Type::Int(setting)] =
                            &message.args[..]
                        {
                            let setting_bool = *setting != 0;
                            self.command_queue.push(OscCommand::SetColorful {
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

                        self.command_queue.push(OscCommand::UpdateTransitionConfig {
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

    pub fn send_glyph(&self, grid_name: &str, index: i32, immediate: i32) {
        let addr = "/grid/glyph".to_string();
        let args = vec![
            osc::Type::String(grid_name.to_string()),
            osc::Type::Int(index),
            osc::Type::Int(immediate),
        ];
        self.sender
            .send((addr, args), (self.target_addr.as_str(), self.target_port))
            .ok();
    }

    pub fn send_next_glyph(&self, grid_name: &str, immediate: i32) {
        let addr = "/grid/nextglyph".to_string();
        let args = vec![
            osc::Type::String(grid_name.to_string()),
            osc::Type::Int(immediate),
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
    pub fn send_no_glyph(&self, grid_name: &str, immediate: i32) {
        let addr = "/grid/noglyph".to_string();
        let args = vec![
            osc::Type::String(grid_name.to_string()),
            osc::Type::Int(immediate),
        ];
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
