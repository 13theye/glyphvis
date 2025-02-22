// src/effets/initializer.rs
// Effects Initializer

use super::effects_manager::*;
use super::grid_fx::PulseEffect;
use super::*;

pub fn fx_initialize() -> EffectsManager {
    let mut effects_manager = EffectsManager::new();

    effects_manager.add(
        "pulse".to_string(),
        EffectType::Grid(Box::new(PulseEffect {
            frequency: 0.5,
            min_brightness: 0.31,
            max_brightness: 0.33,
        })),
    );
    /*

    effects_manager.add(
        "colorcycle".to_string(),
        EffectType::Grid(Box::new(ColorCycleEffect {
            frequency: 0.5,
            saturation: 0.5,
            brightness: 0.5,
        })),
    ); */

    effects_manager
}
