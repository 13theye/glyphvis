// src/config/types.rs
//
// Config types for the app

use crate::animation::EasingType;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct RenderConfig {
    pub texture_width: u32,
    pub texture_height: u32,
    pub texture_samples: u32,
    pub arc_resolution: u32,
}

#[derive(Debug, Deserialize)]
pub struct WindowConfig {
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Deserialize)]
pub struct FrameRecorderConfig {
    pub frame_limit: u32,
    pub jpeg_quality: u8,
}

#[derive(Debug, Deserialize)]
pub struct StyleConfig {
    pub default_stroke_weight: f32,
}

#[derive(Debug, Deserialize)]
pub struct SpeedConfig {
    pub bpm: u32,
}

#[derive(Debug, Deserialize)]
pub struct PathConfig {
    pub project_file: String,
    pub output_directory: String,
}

#[derive(Debug, Deserialize)]
pub struct OscConfig {
    pub rx_port: u16,
}

/************************* Animation Configs ********************/
#[derive(Debug, Deserialize)]
pub struct AnimationConfig {
    pub power_on: PowerOnConfig,
    pub power_off: PowerOffConfig,
    pub background_flash: BackgroundFlashConfig,
    pub transition: TransitionConfig,
}

#[derive(Debug, Deserialize)]
pub struct PowerOnConfig {
    pub flash_duration: f32,
    pub fade_duration: f32,
}

#[derive(Debug, Deserialize)]
pub struct PowerOffConfig {
    pub fade_duration: f32,
}

#[derive(Debug, Deserialize)]
pub struct BackgroundFlashConfig {
    pub flash_duration: f32,
    pub fade_duration: f32,
}

#[derive(Debug, Deserialize, Clone)]
pub struct TransitionConfig {
    pub steps: usize,        // Total number of frames to generate
    pub frame_duration: f32, // Time between frame changes
    pub wandering: f32,      // How much randomness in timing (0.0-1.0)
    pub density: f32,        // How many segments can change per frame (0.0-1.0)
}

#[derive(Debug, Clone)]
pub struct MovementConfig {
    pub duration: f32,
    pub easing: EasingType,
}
