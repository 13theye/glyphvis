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
        Movement, MovementEngine, MovementUpdate, Transition, TransitionEngine, TransitionUpdates,
    },
    config::TransitionConfig,
    effects::BackboneEffect,
    models::Project,
    services::SegmentGraph,
    views::{
        CachedGrid, DrawStyle, Layer, SegmentAction, SegmentState, StyleUpdateMsg, Transform2D,
    },
};

pub struct GridInstance {
    // grid data
    pub id: String,
    pub grid: CachedGrid,
    pub graph: SegmentGraph,

    // glyph state
    pub show: String,
    pub current_glyph_index: usize,
    index_max: usize,

    // effects state
    pub active_transition: Option<Transition>,
    pub transition_config: Option<TransitionConfig>,
    pub immediately_change: bool,
    pub colorful_flag: bool, // enables random-ish color effect target style

    // update messages for a the next frame
    // String is the segment_id
    // StyleUpdateMsg is the update message for the segment
    pub update_batch: HashMap<String, StyleUpdateMsg>,

    // inside-grid state
    pub current_active_segments: HashSet<String>,
    pub target_segments: Option<HashSet<String>>,
    pub target_style: DrawStyle,

    // backbone state
    pub backbone_effects: HashMap<String, Box<dyn BackboneEffect>>,
    pub backbone_style: DrawStyle,

    // grid transform state
    pub spawn_location: Point2,
    pub spawn_rotation: f32,
    pub current_location: Point2,
    pub current_rotation: f32,
    pub current_scale: f32,
    pub active_movement: Option<Movement>,
    pub visible: bool,
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

        Self {
            id,
            grid,
            graph,

            show: show.to_string(),
            current_glyph_index: 1,
            index_max,

            current_active_segments: HashSet::new(),
            target_segments: None,
            target_style: DrawStyle::default(),

            backbone_effects: HashMap::new(),
            backbone_style: DrawStyle {
                color: rgb(0.19, 0.19, 0.19),
                stroke_weight: 5.1,
            },

            active_transition: None,
            transition_config: None,
            immediately_change: false,
            colorful_flag: false,

            update_batch: HashMap::new(),

            spawn_location: position,
            spawn_rotation: rotation,
            current_location: position,
            current_rotation: rotation,
            current_scale: 1.0,
            active_movement: None,
            visible: true,
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

    /************************** New Update Flow ***************************** */

    // The highest level update orchestrator
    pub fn update(&mut self, draw: &Draw, time: f32, dt: f32) {
        // update Grid Instance State
        self.update_movement_state(dt);
        self.update_backbone_effects_state(time);

        // push updates to segments & update graphics
        self.create_segment_updates(dt);
        self.draw_grid_segments(draw);
    }

    fn update_movement_state(&mut self, dt: f32) {
        if let Some(update) = self.process_active_movement(dt) {
            self.update_movement(&update);
        }
    }

    fn update_backbone_effects_state(&mut self, time: f32) {
        self.update_backbone_style(time);
        self.cleanup_backbone_effects(time);
    }

    fn create_segment_updates(&mut self, dt: f32) {
        self.update_transition_segments(dt);
        self.update_backbone_segments();
    }

    fn update_transition_segments(&mut self, dt: f32) {
        if let Some(updates) = self.process_active_transition(dt) {
            self.update_active_segments(&updates);
            self.create_transition_update_messages(&updates);
        }
    }

    fn draw_grid_segments(&mut self, draw: &Draw) {
        self.grid
            .draw_grid_segments(draw, &self.update_batch, self.visible);
        self.clear_update_batch();
    }

    /************************** Update messages and state ******************************/

    fn turn_on_segments(&mut self, segments: &HashSet<String>, target_style: &DrawStyle) {
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

    fn turn_off_segments(&mut self, segments: &HashSet<String>, backbone_style: &DrawStyle) {
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

    pub fn update_active_segment_styles(&mut self, target_style: &DrawStyle) {
        for segment_id in &self.current_active_segments {
            self.update_batch.insert(
                segment_id.clone(),
                StyleUpdateMsg {
                    action: None,
                    target_style: Some(target_style.clone()),
                },
            );
        }
    }

    fn update_backbone_segments(&mut self) {
        for (segment_id, segment) in self.grid.segments.iter() {
            if !self.update_batch.contains_key(segment_id)
                && self.grid.segments[segment_id].layer == Layer::Background
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

    /*********************** Segment Transitions  *****************************/

    // Build the transition
    pub fn start_transition(&mut self, engine: &TransitionEngine) {
        // Handle target segments
        let target_segments = {
            if let Some(segments) = &self.target_segments {
                segments
            } else {
                return;
            }
        };

        let changes = engine.generate_changes(
            self,
            target_segments,
            self.immediately_change, // when true, all segments change at once
        );

        self.active_transition = Some(Transition::new(
            changes,
            engine.default_config.frame_duration,
        ));

        // reset target segments
        self.target_segments = None;
    }

    // Update Step 1: obtain TransitionUpdates by advancing the Transition
    fn process_active_transition(&mut self, dt: f32) -> Option<TransitionUpdates> {
        // Get transition updates if any exist
        if let Some(transition) = &mut self.active_transition {
            if transition.update(dt) {
                // Get updates and check completion
                let updates = transition.advance();
                if transition.is_complete() {
                    self.active_transition = None;
                }
                return updates;
            }
        }
        None
    }

    // Update Step 2: Update the active segments state based on TransitionUpdates
    fn update_active_segments(&mut self, updates: &TransitionUpdates) {
        for segment_id in &updates.segments_on {
            self.current_active_segments.insert(segment_id.clone());
        }

        for segment_id in &updates.segments_off {
            self.current_active_segments.remove(segment_id);
        }
    }

    // Update Step 3: Create style update messages
    fn create_transition_update_messages(&mut self, updates: &TransitionUpdates) {
        let target_style = self.target_style.clone();
        let backbone_style = self.backbone_style.clone();

        if !updates.segments_on.is_empty() {
            self.turn_on_segments(&updates.segments_on, &target_style);
        }

        if !updates.segments_off.is_empty() {
            self.turn_off_segments(&updates.segments_off, &backbone_style);
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
    pub fn instant_color_change(&mut self, new_color: Rgb<f32>) {
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

    /**************************** Grid movement **********************************/

    pub fn apply_transform(&mut self, transform: &Transform2D) {
        self.current_location += transform.translation;
        self.current_scale = transform.scale;
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
            rotation: angle,
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
        self.current_rotation += angle;
    }

    pub fn start_movement(
        &mut self,
        target_x: f32,
        target_y: f32,
        //target_scale: f32,
        //target_rotation: f32,
        engine: &MovementEngine,
    ) {
        let start_transform = Transform2D {
            translation: self.current_location,
            scale: self.current_scale,
            rotation: self.current_rotation,
        };

        let end_transform = Transform2D {
            translation: pt2(target_x, target_y),
            scale: self.current_scale,
            rotation: self.current_rotation,
        };

        let changes = engine.generate_movement(start_transform, end_transform);
        self.active_movement = Some(Movement::new(changes, 1.0 / 60.0));
    }

    fn process_active_movement(&mut self, dt: f32) -> Option<MovementUpdate> {
        if let Some(movement) = &mut self.active_movement {
            if movement.update(dt) {
                let update = movement.advance();
                if movement.is_complete() {
                    self.active_movement = None;
                }
                return update;
            }
        }
        None
    }

    fn update_movement(&mut self, update: &MovementUpdate) {
        self.apply_transform(&update.transform);
    }

    /******************** Backbone style and effects **************************** */

    fn apply_backbone_effects(&self, base_style: &DrawStyle, time: f32) -> DrawStyle {
        if self.backbone_effects.is_empty() {
            return base_style.clone();
        }

        let mut current_style = base_style.clone();

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

    fn update_backbone_style(&mut self, time: f32) {
        self.backbone_style = self.apply_backbone_effects(&self.backbone_style, time);
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
