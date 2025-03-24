// src/views/grid_instance.rs
//
// The GridInstance main updating entity in the visualisation.
//
// Its holds the state information that makes a grid instance unique,
// and provides methods for updating that state.
// It is also the interface between the Grid "hardware" and the rest of
// the system.

use nannou::prelude::*;
use std::collections::{HashMap, HashSet};

use crate::{
    animation::{
        Movement, MovementChange, MovementEngine, SlideAnimation, Transition,
        TransitionAnimationType, TransitionEngine, TransitionTriggerType, TransitionUpdates,
    },
    config::TransitionConfig,
    effects::BackboneEffect,
    models::Project,
    services::SegmentGraph,
    views::{CachedGrid, DrawStyle, SegmentAction, StyleUpdateMsg, Transform2D},
};

pub struct GridInstance {
    // grid data
    pub id: String,

    // The Generic grid defined from SVG data in the Project file and shared methods for
    // drawing each Grid
    pub grid: CachedGrid,

    // The network of relationships between segments
    pub graph: SegmentGraph,

    // glyph state:
    // The Show attached to this Grid.
    // The Grid displays Glyphs in this show, in order or by Index in the Show
    show: String,
    pub current_glyph_index: usize,
    index_max: usize,

    // effects state
    // The currently active transition
    active_transition: Option<Transition>,
    // Parameters that help define the next transition when created
    pub transition_config: Option<TransitionConfig>,
    pub transition_trigger_type: TransitionTriggerType,
    pub transition_next_animation_type: TransitionAnimationType,
    pub transition_trigger_received: bool,
    pub transition_use_stroke_order: bool,

    // Turns on/off the golden flash when a segment is activated. The segment then
    // fades to the target color.
    pub use_power_on_effect: bool,

    // enables random-ish color effect target style
    pub colorful_flag: bool,

    // Segment update messages for the next frame
    // String is the segment_id
    // StyleUpdateMsg is the update message for the segment
    update_batch: HashMap<String, StyleUpdateMsg>,

    // The Glyph segments that will be displayed after any Transition animation
    pub target_segments: Option<HashSet<String>>,

    // Currently active segments for this frame
    pub current_active_segments: HashSet<String>,

    // The target Active Segment style when an effect is complete
    pub target_style: DrawStyle,

    // backbone state (non-active segments)
    //
    backbone_effects: HashMap<String, Box<dyn BackboneEffect>>,
    pub backbone_style: DrawStyle,

    // grid transform state
    //
    // The currently active time-based movement animation
    pub active_movement: Option<Movement>,
    pub current_location: Point2,
    pub current_rotation: f32,
    pub current_scale: f32,
    pub is_visible: bool,
    spawn_location: Point2,

    // state for "instantaneous" movements -- helps interpolate position
    // so that OSC position commmands look sync'ed with refresh
    last_position: Point2,
    target_position: Point2,
    position_update_time: f32,

    // usually equal to time between updates (1.0/60.0)
    movement_duration: f32,

    // Slide animation states
    row_positions: HashMap<i32, f32>, // <index, position offset>
    col_positions: HashMap<i32, f32>, // <index, position offset>
    slide_animations: Vec<SlideAnimation>,
}

impl GridInstance {
    pub fn new(id: String, project: &Project, show: &str, position: Point2, rotation: f32) -> Self {
        let mut grid = CachedGrid::new(project);
        let graph = SegmentGraph::new(&grid);
        let transform = Transform2D {
            translation: position,
            scale: 1.0,
            rotation,
        };
        grid.apply_transform(&transform);

        let index_max = project
            .get_show(show)
            .map_or(0, |show| show.show_order.len());

        println!("\n(===== Creating GridInstance <{}> =====)", id);
        println!("Attached to Show: {}", show);
        println!("Initial position: {}\n", position);

        Self {
            id,
            grid,
            graph,

            show: show.to_string(),
            current_glyph_index: 1,
            index_max,

            target_segments: None,
            current_active_segments: HashSet::new(),
            target_style: DrawStyle::default(),

            active_transition: None,
            transition_config: None,
            transition_trigger_type: TransitionTriggerType::Auto,
            transition_next_animation_type: TransitionAnimationType::default(),
            transition_trigger_received: false,
            transition_use_stroke_order: true,
            use_power_on_effect: false,
            colorful_flag: false,

            update_batch: HashMap::new(),

            backbone_effects: HashMap::new(),
            backbone_style: DrawStyle {
                color: rgba(0.19, 0.19, 0.19, 1.0),
                stroke_weight: 5.1,
            },

            active_movement: None,
            current_location: position,
            current_rotation: rotation,
            current_scale: 1.0,
            is_visible: false,
            spawn_location: position,

            last_position: position,
            target_position: position,
            position_update_time: 0.0,
            movement_duration: 1.0 / 60.0,

            row_positions: HashMap::new(),
            col_positions: HashMap::new(),
            slide_animations: Vec::new(),
        }
    }

    /************************** Glyph System ********************************** */

    // if the glyph exists in the show, retrieve the segments and stage
    // in target_segments. Any anomalies result in no glyph
    pub fn stage_glyph_by_index(&mut self, project: &Project, index: usize) {
        match project.get_show(&self.show) {
            Some(show) => match show.show_order.get(&(index as u32)) {
                Some(show_element) => match project.get_glyph(&show_element.name) {
                    Some(glyph) => {
                        self.current_glyph_index = index;
                        self.target_segments = (!glyph.segments.is_empty())
                            .then(|| glyph.segments.iter().cloned().collect());
                    }
                    None => self.stage_empty_glyph(),
                },
                None => self.stage_empty_glyph(),
            },
            None => self.stage_empty_glyph(),
        }
    }

    pub fn stage_empty_glyph(&mut self) {
        self.target_segments = Some(HashSet::new());
    }

    pub fn stage_next_glyph(&mut self, project: &Project) {
        self.advance_glyph_index(self.current_glyph_index);
        self.stage_glyph_by_index(project, self.current_glyph_index);
    }

    fn advance_glyph_index(&mut self, index: usize) {
        if index + 1 > self.index_max {
            self.current_glyph_index = 1;
        } else {
            self.current_glyph_index += 1;
        }
    }

    /****************************** Update Flow ***************************** */

    // The highest level update orchestrator
    pub fn update(
        &mut self,
        draw: &Draw,
        transition_engine: &TransitionEngine,
        time: f32,
        dt: f32,
    ) {
        // 1. Generate new transitions
        if self.has_target_segments() {
            self.build_transition(transition_engine, self.transition_next_animation_type);
        }

        // 2. Update positioning
        // a. Handle time-based position interpolation (duration = 0.0)
        if self.has_zero_duration_movement() {
            self.apply_zero_duration_movement(time);
        }

        // b. handle duration > 0.0 movements
        if self.has_active_movement() {
            if let Some(update) = self.process_active_movement(dt) {
                self.apply_movement_change(&update);
            }
        }

        // c. handle slide animations
        if self.has_slide_animations() {
            self.update_slide_animations(time);
        }

        // 3. Stage any backbone style change
        if self.has_backbone_effects() {
            self.backbone_style = self.generate_backbone_style(time);
            self.cleanup_backbone_effects(time);
        }

        // 4. Advance any active transition & generate update messages
        if self.has_active_transition() {
            if let Some(updates) = self.process_active_transition(dt) {
                self.update_active_segments_state(&updates);
                self.generate_transition_update_messages(&updates);
            }
        }

        // 5. Generate update messages for remaining segments (backbone)
        self.stage_backbone_updates();

        // 6. Draw
        self.draw_grid_segments(draw);
    }

    fn draw_grid_segments(&mut self, draw: &Draw) {
        self.grid.draw(draw, &self.update_batch, self.is_visible);
        self.clear_update_batch();
    }

    /************************** Update messages and state ******************************/

    fn stage_segments_on(&mut self, segments: &HashSet<String>, target_style: &DrawStyle) {
        for segment_id in segments {
            self.update_batch.insert(
                segment_id.clone(),
                StyleUpdateMsg {
                    action: Some(SegmentAction::On),
                    target_style: Some(target_style.clone()),
                },
            );
        }
    }

    fn stage_segments_instant_on(&mut self, segments: &HashSet<String>, target_style: &DrawStyle) {
        for segment_id in segments {
            self.update_batch.insert(
                segment_id.clone(),
                StyleUpdateMsg {
                    action: Some(SegmentAction::InstantStyleChange),
                    target_style: Some(target_style.clone()),
                },
            );
        }
    }

    fn stage_segments_off(&mut self, segments: &HashSet<String>, backbone_style: &DrawStyle) {
        for segment_id in segments {
            self.update_batch.insert(
                segment_id.clone(),
                StyleUpdateMsg {
                    action: Some(SegmentAction::Off),
                    target_style: Some(backbone_style.clone()),
                },
            );
        }
    }

    fn stage_backbone_updates(&mut self) {
        for (segment_id, segment) in self.grid.segments.iter() {
            if !self.update_batch.contains_key(segment_id)
                && self.grid.segments[segment_id].is_background()
                && segment.is_idle()
            {
                self.update_batch.insert(
                    segment_id.clone(),
                    StyleUpdateMsg {
                        action: Some(SegmentAction::BackboneUpdate),
                        target_style: Some(self.backbone_style.clone()),
                    },
                );
            }
        }
    }

    fn clear_update_batch(&mut self) {
        self.update_batch.clear();
    }

    pub fn set_effect_target_style(&mut self, style: DrawStyle) {
        self.target_style = style;
    }

    /*********************** Glyph Transitions ******************************/

    // Build the transition
    pub fn build_transition(&mut self, engine: &TransitionEngine, typ: TransitionAnimationType) {
        // Only proceed if there are target segments
        if !self.has_target_segments() {
            return;
        }

        let changes = engine.generate_changes(self, typ);

        self.active_transition = Some(Transition::new(
            self.transition_next_animation_type,
            changes,
            engine.default_config.frame_duration,
        ));

        // reset target segments
        self.target_segments = None;
    }

    // Obtain TransitionUpdates by advancing the Transition
    // Todo: extract functionality requiring mutable self
    fn process_active_transition(&mut self, dt: f32) -> Option<TransitionUpdates> {
        // Exit if no active transition
        if !self.has_active_transition() {
            return None;
        }

        let transition = self.active_transition.as_mut().unwrap();

        // Determine if transition should advance based on trigger type
        let should_advance = transition.is_immediate_type()
            || match self.transition_trigger_type {
                TransitionTriggerType::Auto => transition.should_auto_advance(dt),
                TransitionTriggerType::Manual => self.transition_trigger_received,
            };

        // Exit if it's not yet time to advance the transition
        if !should_advance {
            return None;
        }

        // Get updates
        let updates = transition.advance();

        // Reset trigger flag
        self.transition_trigger_received = false;

        // Clear transition if complete; reset Power On effect
        if transition.is_complete() {
            self.active_transition = None;
            self.use_power_on_effect = false;
        }

        updates
    }

    // Update the active segments state based on TransitionUpdates
    fn update_active_segments_state(&mut self, updates: &TransitionUpdates) {
        for segment_id in &updates.segments_on {
            self.current_active_segments.insert(segment_id.clone());
        }

        for segment_id in &updates.segments_off {
            self.current_active_segments.remove(segment_id);
        }
    }

    // Create style update messages
    fn generate_transition_update_messages(&mut self, updates: &TransitionUpdates) {
        let target_style = self.target_style.clone();
        let backbone_style = self.backbone_style.clone();

        if !updates.segments_on.is_empty() {
            if self.use_power_on_effect {
                self.stage_segments_on(&updates.segments_on, &target_style);
            } else {
                self.stage_segments_instant_on(&updates.segments_on, &target_style);
            }
        }

        if !updates.segments_off.is_empty() {
            self.stage_segments_off(&updates.segments_off, &backbone_style);
        }
    }

    pub fn update_transition_config(
        &mut self,
        steps: Option<usize>,
        frame_duration: Option<f32>,
        wandering: Option<f32>,
        density: Option<f32>,
        default_config: &TransitionConfig,
    ) {
        let config = TransitionConfig {
            steps: steps.unwrap_or(default_config.steps),
            frame_duration: frame_duration.unwrap_or(default_config.frame_duration),
            wandering: wandering.unwrap_or(default_config.wandering),
            density: density.unwrap_or(default_config.density),
        };
        self.transition_config = Some(config);
    }

    // a pathway to bypass the transition system and flash effect.
    // updates colors instantly for already active segments
    pub fn instant_color_change(&mut self, new_color: Rgba<f32>) {
        let new_style = DrawStyle {
            color: new_color,
            stroke_weight: self.target_style.stroke_weight,
        };

        // Update target style for future transitions
        self.target_style = new_style.clone();

        // create update messages for active segments
        for segment_id in &self.current_active_segments {
            self.update_batch.insert(
                segment_id.clone(),
                StyleUpdateMsg::new(SegmentAction::InstantStyleChange, new_style.clone()),
            );
        }
    }

    // process OSC /grid/transitiontrigger
    pub fn receive_transition_trigger(&mut self) {
        match self.transition_trigger_type {
            TransitionTriggerType::Auto => {
                self.transition_trigger_type = TransitionTriggerType::Manual;
                if self.has_active_transition() {
                    self.transition_trigger_received = true;
                }
            }
            TransitionTriggerType::Manual => {
                if self.has_active_transition() {
                    self.transition_trigger_received = true;
                }
            }
        }
    }

    /**************************** Grid movement & transform **********************************/

    pub fn rotate_in_place(&mut self, angle: f32) {
        let angle_delta = angle - self.current_rotation;

        // 1. Transform to pivot-relative space
        let to_local = Transform2D {
            translation: -self.current_location,
            scale: 1.0,
            rotation: 0.0,
        };

        // 2. Just rotation
        let rotate = Transform2D {
            translation: Vec2::ZERO,
            scale: 1.0,
            rotation: angle_delta,
        };

        // 3. Transform back
        let to_world = Transform2D {
            translation: self.current_location,
            scale: 1.0,
            rotation: 0.0,
        };

        // Apply each transform in sequence
        self.grid.apply_transform(&to_local);
        self.grid.apply_transform(&rotate);
        self.grid.apply_transform(&to_world);

        // Update location's rotation (but not position)
        self.current_rotation = angle;
    }

    pub fn scale_in_place(&mut self, new_scale: f32) {
        // clamp scale value to a minimum of 0.001
        let safe_scale = if new_scale < 0.001 { 0.001 } else { new_scale };

        let scale_factor = safe_scale / self.current_scale;

        // 1. Transform to pivot-relative space
        let to_local = Transform2D {
            translation: -self.current_location,
            scale: 1.0,
            rotation: 0.0,
        };

        // 2. Just scaling
        let scale = Transform2D {
            translation: Vec2::ZERO,
            scale: scale_factor,
            rotation: 0.0,
        };

        // 3. Transform back
        let to_world = Transform2D {
            translation: self.current_location,
            scale: 1.0,
            rotation: 0.0,
        };

        // Apply each transform in sequence
        self.grid.apply_transform(&to_local);
        self.grid.apply_transform(&scale);
        self.grid.apply_transform(&to_world);

        // Scale current and any future stroke weights
        self.grid.scale_stroke_weights(scale_factor);
        self.backbone_style.stroke_weight *= scale_factor;
        self.target_style.stroke_weight *= scale_factor;

        // Update scale state
        self.current_scale = safe_scale;
    }

    // Sets up a Movement over a specified duration
    pub fn stage_movement(
        &mut self,
        target_x: f32,
        target_y: f32,
        duration: f32,
        engine: &MovementEngine,
        time: f32,
    ) {
        // If duration is specified, use the existing MovementEngine
        if duration > 0.0 {
            self.active_movement = Some(engine.build_timed_movement(self, target_x, target_y));
        } else {
            // For immediate movements (duration = 0.0), use time-based interpolation
            self.stage_zero_duration_movement(target_x, target_y, time);
        }
    }

    fn stage_zero_duration_movement(&mut self, target_x: f32, target_y: f32, time: f32) {
        self.last_position = self.current_location;
        self.target_position = pt2(target_x, target_y);
        self.position_update_time = time;
        self.movement_duration = 1.0 / 60.0;
    }

    fn process_active_movement(&mut self, dt: f32) -> Option<MovementChange> {
        let movement = self.active_movement.as_mut().unwrap();

        if movement.update(dt) {
            let update = movement.advance();
            if movement.is_complete() {
                self.active_movement = None;
            }
            update
        } else {
            None
        }
    }

    fn apply_movement_change(&mut self, change: &MovementChange) {
        self.apply_transform(&change.transform);
    }

    fn apply_zero_duration_movement(&mut self, time: f32) {
        let elapsed = time - self.position_update_time;
        let progress = (elapsed / self.movement_duration).clamp(0.0, 1.0);

        if progress < 1.0 {
            // Calculate the interpolated position
            let interp_x =
                self.last_position.x + (self.target_position.x - self.last_position.x) * progress;
            let interp_y =
                self.last_position.y + (self.target_position.y - self.last_position.y) * progress;
            let interp_position = pt2(interp_x, interp_y);

            // Calculate the delta from current position
            let delta = interp_position - self.current_location;

            // Apply the transform
            if delta.length() > 0.01 {
                let transform = Transform2D {
                    translation: delta,
                    scale: 1.0,
                    rotation: 0.0,
                };
                self.apply_transform(&transform);
            }
        } else {
            // We've reached the end of the interpolation
            // Set exact position to avoid floating point errors
            let delta = self.target_position - self.current_location;
            if delta.length() > 0.01 {
                let transform = Transform2D {
                    translation: delta,
                    scale: 1.0,
                    rotation: 0.0,
                };
                self.apply_transform(&transform);
            }

            // Mark interpolation as complete
            self.last_position = self.target_position;
        }
    }

    fn apply_transform(&mut self, transform: &Transform2D) {
        // update self.current_location here only.
        // the rotation and and scale states aren't as straightforward.
        self.current_location += transform.translation;
        self.grid.apply_transform(transform);
    }

    // go back to where grid spawned
    pub fn reset_location(&mut self) {
        let transform = Transform2D {
            translation: self.spawn_location - self.current_location,
            scale: 1.0,
            rotation: 0.0,
        };
        self.apply_transform(&transform);
    }

    /**************************** Row/column Slide Effect *****************************/
    pub fn slide(&mut self, axis: &str, index: i32, position: f32, time: f32) {
        // Validate axis
        if axis != "x" && axis != "y" {
            println!("Slide axis value must be x or y. Current value is {}", axis);
        }

        // Transform axis to char
        let axis_char = match axis {
            "x" => 'x',
            "y" => 'y',
            _ => '0', // this case shouldn't happen
        };

        // Get current row/col positions
        let positions = match axis_char {
            'x' => &mut self.row_positions,
            'y' => &mut self.col_positions,
            _ => return,
        };

        // Get current position (default to 0.0 if not set)
        let current_position = *positions.get(&index).unwrap_or(&0.0);

        // Update stored position
        positions.insert(index, position);

        // Find existing animation or create new
        let existing_index = self
            .slide_animations
            .iter()
            .position(|anim| anim.axis == axis_char && anim.index == index);

        if let Some(idx) = existing_index {
            // Update existing animation
            let anim = &mut self.slide_animations[idx];
            anim.start_position = anim.current_position;
            anim.target_position = position;
            anim.start_time = time;
        } else {
            // Create new animation
            let animation = SlideAnimation {
                axis: axis_char,
                index,
                start_position: current_position,
                current_position,
                target_position: position,
                start_time: time,
                duration: 1.0 / 60.0,
            };

            self.slide_animations.push(animation);
        }
    }

    fn update_slide_animations(&mut self, time: f32) {
        let mut transforms_to_apply: Vec<(i32, char, Transform2D)> = Vec::new();
        let mut completed = Vec::new();

        // Calculate all transforms without applying them yet
        for (i, animation) in self.slide_animations.iter_mut().enumerate() {
            let elapsed = time - animation.start_time;
            let progress = (elapsed / animation.duration).clamp(0.0, 1.0);

            if progress < 1.0 {
                // Calculate interpolated position
                let new_position = animation.start_position
                    + (animation.target_position - animation.start_position) * progress;

                // Calculate movement delta from last frame
                let delta = new_position - animation.current_position;

                // Create transform if there's significant movement
                if delta.abs() > 0.001 {
                    let translation = match animation.axis {
                        'x' => vec2(delta, 0.0),
                        'y' => vec2(0.0, delta),
                        _ => vec2(0.0, 0.0),
                    };

                    let transform = Transform2D {
                        translation,
                        scale: 1.0,
                        rotation: 0.0,
                    };

                    transforms_to_apply.push((animation.index, animation.axis, transform));
                }

                // Update current position
                animation.current_position = new_position;
            } else {
                // Ensure we reach exactly the target position
                let delta = animation.target_position - animation.current_position;

                if delta.abs() > 0.001 {
                    let translation = match animation.axis {
                        'x' => vec2(delta, 0.0),
                        'y' => vec2(0.0, delta),
                        _ => vec2(0.0, 0.0),
                    };

                    let transform = Transform2D {
                        translation,
                        scale: 1.0,
                        rotation: 0.0,
                    };

                    transforms_to_apply.push((animation.index, animation.axis, transform));
                }

                animation.current_position = animation.target_position;
                completed.push(i);
            }
        }

        // Apply all calculated transforms
        for (index, axis, transform) in transforms_to_apply {
            match axis {
                'x' => {
                    // Get row segments from CachedGrid and apply transform
                    let segments = self.grid.row_mut(index);
                    for segment in segments {
                        segment.apply_transform(&transform);
                    }
                }
                'y' => {
                    // Get column segments from CachedGrid and apply transform
                    let segments = self.grid.col_mut(index);
                    for segment in segments {
                        segment.apply_transform(&transform);
                    }
                }
                _ => {}
            }
        }

        // Remove completed animations
        for i in completed.iter().rev() {
            self.slide_animations.remove(*i);
        }
    }

    /******************** Backbone style and effects **************************** */

    fn generate_backbone_style(&self, time: f32) -> DrawStyle {
        let mut style = self.backbone_style.clone();

        for effect in self.backbone_effects.values() {
            if effect.is_finished(time) {
                continue;
            }
            style = effect.update(&style, time);
        }
        style
    }

    fn cleanup_backbone_effects(&mut self, time: f32) {
        for effect_type in self.finished_effects(time) {
            println!("Removing effect {}", effect_type);
            self.backbone_effects.remove(&effect_type);
        }
    }

    fn finished_effects(&self, time: f32) -> Vec<String> {
        let mut finished = Vec::new();
        for effect_type in self.backbone_effects.keys() {
            if let Some(effect) = self.backbone_effects.get(effect_type) {
                if effect.is_finished(time) {
                    finished.push(effect_type.clone());
                }
            }
        }
        finished
    }

    pub fn add_backbone_effect(&mut self, effect_type: &str, effect: Box<dyn BackboneEffect>) {
        self.backbone_effects
            .insert(effect_type.to_string(), effect);
    }

    /*********************** Utility Methods **************************** */

    pub fn has_target_segments(&self) -> bool {
        self.target_segments.is_some()
    }

    pub fn has_active_transition(&self) -> bool {
        self.active_transition.is_some()
    }

    pub fn has_active_movement(&self) -> bool {
        self.active_movement.is_some()
    }

    pub fn has_zero_duration_movement(&self) -> bool {
        self.last_position != self.target_position
    }

    pub fn has_backbone_effects(&self) -> bool {
        !self.backbone_effects.is_empty()
    }

    pub fn has_slide_animations(&self) -> bool {
        !self.slide_animations.is_empty()
    }

    /*********************** Debug Helper ******************************* */

    pub fn print_grid_info(&self) {
        println!("<====== Grid Instance: {} ======>", self.id);
        println!("\nGrid Info:");
        println!("Location: {:?}", self.current_location);
        println!("Dimensions: {:?}", self.grid.dimensions);
        println!("Viewbox: {:?}", self.grid.viewbox);
        println!("Segment count: {}\n", self.grid.segments.len());
    }
}
