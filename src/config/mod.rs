pub mod config_load;
pub mod config_types;
pub mod runtime;

pub use config_load::Config;
pub use config_types::{
    AnimationConfig, FrameRecorderConfig, MovementConfig, OscConfig, PathConfig, RenderConfig,
    SpeedConfig, StyleConfig, TransitionConfig, WindowConfig,
};
