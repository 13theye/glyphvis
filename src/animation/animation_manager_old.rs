// src/animation/animation_manager.rs
/// time based movement.
/// Grid animations move the grid with a transform.
/// Segment animations don't actually move anything, but use neighbor logic
/// to make the Glyphs appear to move.
use crate::views::DrawStyle;
use std::collections::HashMap;

pub enum AnimationType {
    Transform(Box<dyn Animation>),
    Segment(Box<dyn SegmentAnimation>),
}

// the base Effect trait which all effects must implement
pub trait Animation {
    fn apply(&self, style: &DrawStyle, time: f32) -> DrawStyle;
    fn is_animation_finished(&self) -> bool;
}

pub trait SegmentAnimation: Animation {
    fn apply_to_segment(
        &self,
        segment_id: &str,
        base_style: &DrawStyle,
        current_time: f32,
    ) -> DrawStyle;
    fn animate_segment(&mut self, segment_id: &str, time: f32);
    fn deanimate_segment(&mut self, segment_id: &str);
    fn is_segment_animating(&self, segment_id: &str) -> bool;
    fn is_animation_finished(&self) -> bool;
}

impl<T: SegmentAnimation> Animation for T {
    // This won't be used for segment animations
    fn apply(&self, style: &DrawStyle, _time: f32) -> DrawStyle {
        style.clone()
    }

    fn is_finished(&self) -> bool {
        AnimationEffect::is_animation_finished(self)
    }
}

// Effect instance with metadata
struct AnimationInstance {
    animation: AnimationType,
    is_active: bool,
    start_time: f32,
}

#[derive(Default)]
pub struct AnimationManager {
    animations: HashMap<String, AnimationInstance>,
}

impl AnimationManager {
    pub fn new() -> Self {
        Self {
            animations: HashMap::new(),
        }
    }

    // Add a new effect
    pub fn add(&mut self, name: String, animation: AnimationType, time: f32) {
        self.animations.insert(
            name,
            AnimationInstance {
                animation,
                is_active: true,
                start_time: time,
            },
        );
    }

    // Remove an effect
    pub fn remove(&mut self, name: &str) {
        self.animations.remove(name);
    }

    // toggle an effect
    pub fn activate(&mut self, name: &str) {
        if let Some(instance) = self.animations.get_mut(name) {
            instance.is_active = true;
        }
    }

    pub fn deactivate(&mut self, name: &str) {
        if let Some(instance) = self.animations.get_mut(name) {
            instance.is_active = false;
        }
    }

    // Apply all active effects
    pub fn apply_segment_animation(
        &self,
        segment_id: &str,
        base_style: DrawStyle,
        time: f32,
    ) -> DrawStyle {
        if self.effects.is_empty() {
            return base_style;
        }

        let mut current_style = base_style.clone();

        for instance in self.effects.values() {
            if !instance.is_active {
                continue;
            }

            match &instance.effect {
                EffectType::Grid(_) => {
                    //current_style = effect.apply(&base_style, time);
                    continue;
                }
                EffectType::Segment(effect) => {
                    if effect.is_segment_active(segment_id) {
                        current_style = effect.apply_to_segment(segment_id, &base_style, time);
                    }
                }
            }
        }

        current_style
    }

    // Apply all active effects
    pub fn apply_grid_effects(
        &self,
        segment_id: &str,
        base_style: DrawStyle,
        time: f32,
    ) -> DrawStyle {
        if self.effects.is_empty() {
            return base_style;
        }

        let mut current_style = base_style.clone();

        for instance in self.effects.values() {
            if !instance.is_active {
                continue;
            }

            match &instance.effect {
                EffectType::Grid(effect) => {
                    current_style = effect.apply(&base_style, time);
                }
                EffectType::Segment(effect) => {
                    if effect.is_segment_active(segment_id) {
                        //current_style = effect.apply_to_segment(segment_id, &base_style, time);
                        continue;
                    }
                }
            }
        }

        current_style
    }

    // for segment-specific operations
    pub fn activate_segment(&mut self, segment_id: &str, effect_name: &str, time: f32) {
        if let Some(instance) = self.effects.get_mut(effect_name) {
            if let EffectType::Segment(effect) = &mut instance.effect {
                effect.activate_segment(segment_id, time);
            }
        }
    }

    // Clean up finished effects
    pub fn cleanup(&mut self) {
        self.effects.retain(|_, instance| match &instance.effect {
            EffectType::Grid(effect) => !effect.is_finished(),
            EffectType::Segment(effect) => !effect.is_finished(),
        });
    }

    // Utility functions
    pub fn is_active(&self, name: &str) -> bool {
        self.effects
            .get(name)
            .map(|instance| instance.is_active)
            .unwrap_or(false)
    }

    pub fn has_effect(&self, name: &str) -> bool {
        self.effects.contains_key(name)
    }
}
