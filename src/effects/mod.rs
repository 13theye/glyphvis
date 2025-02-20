use crate::views::DrawStyle;
use nannou::prelude::*;

pub mod background_fx;
pub mod effects_manager;
pub mod grid_fx;
pub mod initializer;

pub use background_fx::{BackgroundColorFade, BackgroundFlash};
pub use effects_manager::*;
pub use initializer::fx_initialize;

pub enum EffectType {
    Grid(Box<dyn Effect>),
}

// the base Effect trait which all effects must implement
pub trait Effect {
    fn update(&self, style: &DrawStyle, time: f32) -> DrawStyle;
    fn is_finished(&self) -> bool;
}

// Effect instance with metadata
struct EffectInstance {
    effect: EffectType,
    is_active: bool,
}

pub trait BackgroundEffect {
    fn start(&mut self, start_color: Rgb, target_color: Rgb, duration: f32, current_time: f32);
    fn update(&mut self, current_time: f32) -> Option<Rgb>;
    fn is_active(&self) -> bool;
}
