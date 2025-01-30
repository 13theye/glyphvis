// src/config/config.rs
//
// loading to config.toml

use serde::Deserialize;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Deserialize)]
pub struct Config {
    pub paths: PathConfig,
    pub rendering: RenderConfig,
    pub window: WindowConfig,
    pub output: OutputConfig,
    pub style: StyleConfig,
    pub speed: SpeedConfig,
    pub animation: AnimationConfig,
}

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
pub struct OutputConfig {
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
pub struct AnimationConfig {
    pub power_on_flash_duration: f32,
    pub power_on_fade_duration: f32,
    pub power_off_fade_duration: f32,
}

impl Config {
    pub fn load() -> Result<Self, Box<dyn std::error::Error>> {
        // First try to load from the executable's directory
        if let Some(exe_config) = Self::load_from_exe_dir() {
            return Ok(exe_config);
        }

        // Fallback to loading from the current working directory
        Self::load_from_working_dir()
    }

    fn load_from_exe_dir() -> Option<Self> {
        let exe_path = std::env::current_exe().ok()?;
        let exe_dir = exe_path.parent()?;
        let config_path = exe_dir.join("config.toml");

        if config_path.exists() {
            let content = fs::read_to_string(&config_path).ok()?;
            toml::from_str(&content).ok()
        } else {
            None
        }
    }

    fn load_from_working_dir() -> Result<Self, Box<dyn std::error::Error>> {
        let content = fs::read_to_string("config.toml")?;
        Ok(toml::from_str(&content)?)
    }

    pub fn resolve_project_path(&self) -> PathBuf {
        if Path::new(&self.paths.project_file).is_absolute() {
            PathBuf::from(&self.paths.project_file)
        } else {
            // If path is relative, resolve it relative to the executable or working directory
            if let Some(exe_dir) = std::env::current_exe()
                .ok()
                .and_then(|p| p.parent().map(|p| p.to_path_buf()))
            {
                exe_dir.join(&self.paths.project_file)
            } else {
                PathBuf::from(&self.paths.project_file)
            }
        }
    }

    pub fn resolve_output_dir(&self) -> PathBuf {
        if Path::new(&self.paths.output_directory).is_absolute() {
            PathBuf::from(&self.paths.output_directory)
        } else {
            // If path is relative, resolve it relative to the executable or working directory
            if let Some(exe_dir) = std::env::current_exe()
                .ok()
                .and_then(|p| p.parent().map(|p| p.to_path_buf()))
            {
                exe_dir.join(&self.paths.output_directory)
            } else {
                PathBuf::from(&self.paths.output_directory)
            }
        }
    }

    pub fn resolve_output_dir_as_str(&self) -> String {
        let path = if Path::new(&self.paths.output_directory).is_absolute() {
            PathBuf::from(&self.paths.output_directory)
        } else {
            // If path is relative, resolve it relative to the executable or working directory
            std::env::current_exe()
                .ok()
                .and_then(|p| p.parent().map(|p| p.to_path_buf()))
                .map(|exe_dir| exe_dir.join(&self.paths.output_directory))
                .unwrap_or_else(|| PathBuf::from(&self.paths.output_directory))
        };

        path.to_string_lossy().into_owned() // Convert PathBuf to String safely
    }
}
