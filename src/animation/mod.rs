pub mod movement;
pub mod slide_movement;
pub mod stretch;
pub mod stroke_order;
pub mod transition;

pub use movement::{EasingType, MovementChange, MovementEngine, TimedMovement};
pub use slide_movement::SlideAnimation;
pub use stretch::StretchAnimation;
pub use transition::{
    Transition, TransitionAnimationType, TransitionEngine, TransitionTriggerType, TransitionUpdates,
};

use nannou::prelude::*;

pub trait Animation {
    fn should_update(&mut self, dt: f32) -> bool; // True when ready to advance
    fn advance(&mut self, current_position: Point2, time: f32) -> Option<MovementChange>; // Advance the animation, returning updates
    fn is_complete(&self) -> bool; // True when animation is finished
}
