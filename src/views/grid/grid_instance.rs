// src/views/grid_manager.rs
//
// The GridInstance main updating entity in the visualisation.
//
// Its holds the state information that makes a a grid instance unique,
// and provides methods for updating that state.
// It is also the interface between the Grid "hardware" and the rest of
// the system.

use nannou::prelude::*;
use std::collections::{HashMap, HashSet};

use crate::{
    animation::{
        Movement, MovementEngine, MovementUpdate, Transition, TransitionEngine, TransitionTrigger,
        TransitionUpdates,
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
    pub grid: CachedGrid,
    pub graph: SegmentGraph,

    // glyph state
    show: String,
    current_glyph_index: usize,
    index_max: usize,

    // effects state
    active_transition: Option<Transition>,
    pub transition_config: Option<TransitionConfig>,
    pub transition_trigger_type: TransitionTrigger,
    pub transition_trigger_received: bool,
    pub transition_use_stroke_order: bool,
    pub next_glyph_change_is_immediate: bool,
    pub use_power_on_effect: bool,
    pub colorful_flag: bool, // enables random-ish color effect target style

    // update messages for a the next frame
    // String is the segment_id
    // StyleUpdateMsg is the update message for the segment
    update_batch: HashMap<String, StyleUpdateMsg>,

    // inside-grid state
    pub target_segments: Option<HashSet<String>>,
    pub current_active_segments: HashSet<String>,
    pub target_style: DrawStyle,

    // backbone state
    backbone_effects: HashMap<String, Box<dyn BackboneEffect>>,
    pub backbone_style: DrawStyle,

    // grid transform state
    pub active_movement: Option<Movement>,
    spawn_location: Point2,

    // state for "instantaneous" movements
    last_position: Point2,
    target_position: Point2,
    position_update_time: f32,
    movement_duration: f32,

    pub current_location: Point2,
    pub current_rotation: f32,
    pub current_scale: f32,
    pub is_visible: bool,
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
        println!("Initial position: {}", position);

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

            backbone_effects: HashMap::new(),
            backbone_style: DrawStyle {
                color: rgba(0.19, 0.19, 0.19, 1.0),
                stroke_weight: 5.1,
            },

            active_transition: None,
            transition_config: None,
            transition_trigger_type: TransitionTrigger::Auto,
            transition_trigger_received: false,
            transition_use_stroke_order: true,
            next_glyph_change_is_immediate: false,
            use_power_on_effect: false,
            colorful_flag: false,

            update_batch: HashMap::new(),

            active_movement: None,
            target_position: position,
            last_position: position,
            position_update_time: 0.0,
            movement_duration: 1.0 / 60.0,

            spawn_location: position,
            current_location: position,
            current_rotation: rotation,
            current_scale: 1.0,
            is_visible: false,
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
                    None => self.no_glyph(),
                },
                None => self.no_glyph(),
            },
            None => self.no_glyph(),
        }
    }

    pub fn no_glyph(&mut self) {
        self.target_segments = Some(HashSet::new());
    }

    pub fn stage_next_glyph(&mut self, project: &Project) {
        self.advance_index(self.current_glyph_index);
        self.stage_glyph_by_index(project, self.current_glyph_index);
    }

    fn advance_index(&mut self, index: usize) {
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
        // Continue with existing update logic
        // 1. Generate new transitions
        if self.has_target_segments() {
            self.build_transition(transition_engine);
        }

        // 2. Update positioning
        // a. Handle time-based position interpolation (duration = 0.0)
        if self.has_zero_duration_movement() {
            self.apply_zero_duration_movement(time);
        }

        // b. handle duration > 0.0 movements
        if self.has_active_movement() {
            if let Some(update) = self.process_active_movement(dt) {
                self.apply_movement_update(&update);
            }
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
        self.message_backbone_updates();

        // 6. Draw
        self.draw_grid_segments(draw);
    }

    fn draw_grid_segments(&mut self, draw: &Draw) {
        self.grid
            .draw_grid_segments(draw, &self.update_batch, self.is_visible);
        self.clear_update_batch();
    }

    /************************** Update messages and state ******************************/

    fn message_segments_on(&mut self, segments: &HashSet<String>, target_style: &DrawStyle) {
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

    fn message_segments_instant_on(
        &mut self,
        segments: &HashSet<String>,
        target_style: &DrawStyle,
    ) {
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

    fn message_segments_off(&mut self, segments: &HashSet<String>, backbone_style: &DrawStyle) {
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

    fn message_backbone_updates(&mut self) {
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

    /*********************** Segment Transitions ******************************/

    // Build the transition
    pub fn build_transition(&mut self, engine: &TransitionEngine) {
        // Handle target segments
        let target_segments = self.target_segments.as_ref().unwrap();

        let changes = if self.transition_use_stroke_order {
            engine.generate_stroke_order_changes(self, target_segments)
        } else {
            engine.generate_changes(
                self,
                target_segments,
                self.next_glyph_change_is_immediate, // when true, all segments change at once
            )
        };

        self.active_transition = Some(Transition::new(
            changes,
            engine.default_config.frame_duration,
        ));

        // reset target segments
        self.target_segments = None;
    }

    // Obtain TransitionUpdates by advancing the Transition
    fn process_active_transition(&mut self, dt: f32) -> Option<TransitionUpdates> {
        // Early return if no active transition
        let transition = self.active_transition.as_mut().unwrap();

        // Determine if transition should advance based on trigger type
        let should_advance = self.next_glyph_change_is_immediate
            || match self.transition_trigger_type {
                TransitionTrigger::Auto => transition.should_auto_advance(dt),
                TransitionTrigger::Manual => self.transition_trigger_received,
            };

        if !should_advance {
            return None;
        }

        // Get updates
        let updates = transition.advance();

        // Reset trigger flag
        self.transition_trigger_received = false;

        // Clear transition if complete
        if transition.is_complete() {
            self.active_transition = None;
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
                self.message_segments_on(&updates.segments_on, &target_style);
            } else {
                self.message_segments_instant_on(&updates.segments_on, &target_style);
            }
        }

        if !updates.segments_off.is_empty() {
            self.message_segments_off(&updates.segments_off, &backbone_style);
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

    /**************************** Grid movement & transform **********************************/

    pub fn apply_transform(&mut self, transform: &Transform2D) {
        // update self.current_location here only.
        // the rotation and and scale states aren't as straightforward.
        self.current_location += transform.translation;
        self.grid.apply_transform(transform);
    }

    pub fn reset_location(&mut self) {
        let transform = Transform2D {
            translation: self.spawn_location - self.current_location,
            scale: 1.0,
            rotation: 0.0,
        };
        self.apply_transform(&transform);
    }

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

    pub fn build_movement(
        &mut self,
        target_x: f32,
        target_y: f32,
        duration: f32,
        engine: &MovementEngine,
        time: f32,
    ) {
        // Create target point from coordinates
        let target_position = pt2(target_x, target_y);

        // If duration is specified, use the existing MovementEngine
        if duration > 0.0 {
            let start_transform = Transform2D {
                translation: self.current_location,
                scale: self.current_scale,
                rotation: self.current_rotation,
            };

            let end_transform = Transform2D {
                translation: target_position,
                scale: self.current_scale,
                rotation: self.current_rotation,
            };

            let changes = engine.generate_movement(start_transform, end_transform);
            self.active_movement = Some(Movement::new(changes, 1.0 / 60.0));
        } else {
            // For immediate movements (duration = 0.0), use time-based interpolation
            self.last_position = self.current_location;
            self.target_position = target_position;
            self.position_update_time = time;
            self.movement_duration = 1.0 / 60.0;
        }
    }

    fn process_active_movement(&mut self, dt: f32) -> Option<MovementUpdate> {
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

    fn apply_movement_update(&mut self, update: &MovementUpdate) {
        self.apply_transform(&update.transform);
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

    /******************** Backbone style and effects **************************** */

    fn generate_backbone_style(&self, time: f32) -> DrawStyle {
        let mut current_style = self.backbone_style.clone();

        for effect in self.backbone_effects.values() {
            if effect.is_finished(time) {
                continue;
            }

            current_style = effect.update(&current_style, time);
        }

        current_style
    }

    fn cleanup_backbone_effects(&mut self, time: f32) {
        for effect_type in self.get_finished_effects(time) {
            println!("Removing effect {}", effect_type);
            self.backbone_effects.remove(&effect_type);
        }
    }

    fn get_finished_effects(&self, time: f32) -> Vec<String> {
        let mut finished_effects = Vec::new();
        for effect_type in self.backbone_effects.keys() {
            if let Some(effect) = self.backbone_effects.get(effect_type) {
                if effect.is_finished(time) {
                    finished_effects.push(effect_type.clone());
                }
            }
        }
        finished_effects
    }

    pub fn init_backbone_effect(&mut self, effect_type: &str, effect: Box<dyn BackboneEffect>) {
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

    pub fn receive_transition_trigger(&mut self) {
        if self.has_active_transition() {
            self.transition_trigger_received = true;
        }
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
