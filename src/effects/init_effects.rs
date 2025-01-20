// src/effets/effects_init.rs
// The Effects Initializer

use nannou::prelude::*;
use super::effects_manager::*;
use super::power_on_effect::PowerOnEffect;
//use super::grid_effects::{ PulseEffect, ColorCycleEffect };

pub fn init_effects(app: &App) -> EffectsManager {
    let mut effects_manager = EffectsManager::new();

    let power_on_effect = PowerOnEffect::new(
        rgb(1.0, 0.0, 0.0), // currently not used
        0.01,
        0.3,
    );

    effects_manager.add(
        "power_on".to_string(),
        EffectType::Segment(Box::new(power_on_effect)),
        app.time,
    );  
    /*
    effects_manager.add(
        "pulse".to_string(),
        EffectType::Grid(Box::new(PulseEffect {
            frequency: 1.0,
            min_brightness: 0.2,
            max_brightness: 0.6,
        })),
        app.time,
    );   
    
    
    effects_manager.add(
        "colorcycle".to_string(),
        EffectType::Grid(Box::new(ColorCycleEffect {
            frequency: 1.0,
            saturation: 1.0,
            brightness: 1.0,
        })),
        app.time,
    );
 */

    effects_manager
}