// src/animation/animation_manager.rs
// time based movement.
// Grid animations move the grid with a transform.
// Segment animations don't actually move anything, but use neighbor logic
// to make the Glyphs appear to move.

use crate::views::Transform2D;
use std::collections::{HashMap, HashSet};

// Base trait for all animations
pub trait Animation {
    fn update(&mut self, time: f32) -> bool; // returns true when complete
    fn reset(&mut self);
    fn is_finished(&self) -> bool;
}

// For moving entire grids or segments
pub trait TransformAnimation: Animation {
    fn get_transform(&self, time: f32) -> Transform2D;
}

// For LCD-style segment animations
pub trait SegmentAnimation: Animation {
    fn get_active_segments(&self, time: f32) -> HashSet<String>;
}

pub struct AnimationManager {
    transform_animations: HashMap<String, Box<dyn TransformAnimation>>,
    segment_animations: HashMap<String, Box<dyn SegmentAnimation>>,
}

impl AnimationManager {
    fn update(&mut self, time: f32) {
        // Update all animations, remove completed ones
    }

    fn get_current_transform(&self, time: f32) -> Transform2D {
        // Combine all active transform animations
    }

    fn get_active_segments(&self, time: f32) -> HashSet<String> {
        // Combine all active segment animations
    }
}
