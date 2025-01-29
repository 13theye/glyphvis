// src/effets/effects_init.rs
// The Effects Initializer

use super::effects_manager::*;
use super::grid_effects::{ColorCycleEffect, PulseEffect};
use nannou::prelude::*;

pub fn init_effects(app: &App) -> EffectsManager {
    let mut effects_manager = EffectsManager::new();

    effects_manager.add(
        "pulse".to_string(),
        EffectType::Grid(Box::new(PulseEffect {
            frequency: 2.0,
            min_brightness: 0.2,
            max_brightness: 0.6,
        })),
        app.time,
    );

    /*
    effects_manager.add(
        "colorcycle".to_string(),
        EffectType::Grid(Box::new(ColorCycleEffect {
            frequency: 0.5,
            saturation: 0.5,
            brightness: 0.5,
        })),
        app.time,

    ); */

    effects_manager
}
