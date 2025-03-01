use crate::views::DrawStyle;
use nannou::prelude::*;

pub mod backbone_fx;
pub mod background_fx;

pub use backbone_fx::FadeEffect;
pub use background_fx::{BackgroundColorFade, BackgroundFlash};

// the base Effect trait which all effects must implement
pub trait BackboneEffect {
    fn update(&self, style: &DrawStyle, time: f32) -> DrawStyle;
    fn is_finished(&self, time: f32) -> bool;
}

pub trait BackgroundEffect {
    fn start(&mut self, start_color: Rgb, target_color: Rgb, duration: f32, current_time: f32);
    fn update(&mut self, current_time: f32) -> Option<Rgb>;
    fn is_active(&self) -> bool;
}
