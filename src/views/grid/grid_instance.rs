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
    animation::{Transition, TransitionEngine},
    effects::{init_effects, EffectsManager},
    models::Project,
    views::{
        CachedGrid, DrawStyle, Layer, SegmentAction, SegmentGraph, StyleUpdateMsg, Transform2D,
    },
};

pub struct GridInstance {
    // grid data
    pub id: String,
    pub grid: CachedGrid,
    pub graph: SegmentGraph,

    // effects state
    pub effects_manager: EffectsManager,
    pub active_transition: Option<Transition>,

    // update messages for an update frame
    pub update_batch: HashMap<String, StyleUpdateMsg>,

    // inside-grid state
    pub current_active_segments: HashSet<String>,

    // overall grid state
    pub spawn_location: Point2,
    pub spawn_rotation: f32,
    pub current_location: Point2,
    pub current_rotation: f32,
    pub visible: bool,
}

impl GridInstance {
    pub fn new(_app: &App, project: &Project, id: String, position: Point2, rotation: f32) -> Self {
        let mut grid = CachedGrid::new(project);
        let graph = SegmentGraph::new(&grid);
        let transform = Transform2D {
            translation: position,
            scale: 1.0,
            rotation,
        };
        grid.apply_transform(&transform);

        Self {
            id,
            grid,
            graph,

            current_active_segments: HashSet::new(),

            /* will add this when timeline is implemented
            target_active_segments: None,
            transition_timeline: None,
            transition_start_time: None,
             */
            effects_manager: init_effects::init_effects(),
            active_transition: None,

            update_batch: HashMap::new(),

            spawn_location: position,
            spawn_rotation: rotation,
            current_location: position,
            current_rotation: rotation,
            visible: true,
        }
    }

    /************************** New Update System ***************************** */

    pub fn turn_on_segments(&mut self, segments: HashSet<String>, target_style: &DrawStyle) {
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

    pub fn turn_off_segments(&mut self, segments: HashSet<String>, bg_style: &DrawStyle) {
        for segment_id in segments {
            self.update_batch.insert(
                segment_id.clone(),
                StyleUpdateMsg {
                    action: Some(SegmentAction::Off),
                    target_style: Some(bg_style.clone()),
                },
            );
        }
    }

    pub fn update_background_segments(&mut self, bg_style: &DrawStyle, time: f32) {
        for (segment_id, segment) in self.grid.segments.iter() {
            if !self.update_batch.contains_key(segment_id)
                && self.grid.segments[segment_id].layer == Layer::Background
                && segment.current_action.is_none()
            {
                self.update_batch.insert(
                    segment_id.clone(),
                    StyleUpdateMsg {
                        action: None,
                        target_style: Some(
                            self.effects_manager
                                .apply_grid_effects(bg_style.clone(), time),
                        ),
                    },
                );
            }
        }
    }

    pub fn update(&mut self, target_style: &DrawStyle, bg_style: &DrawStyle, _time: f32, dt: f32) {
        // First, get transition updates if any exist
        let transition_updates = if let Some(transition) = &mut self.active_transition {
            if transition.update(dt) {
                // Get updates and check completion
                let updates = transition.advance();
                if transition.is_complete() {
                    self.active_transition = None;
                }
                updates
            } else {
                None
            }
        } else {
            None
        };

        // Then apply any transition updates we collected
        if let Some(updates) = transition_updates {
            for segment_id in &updates.segments_on {
                self.current_active_segments.insert(segment_id.clone());
            }
            for segment_id in &updates.segments_off {
                self.current_active_segments.remove(segment_id);
            }

            // Convert frame difference into on/off messages
            if !updates.segments_on.is_empty() {
                self.turn_on_segments(updates.segments_on, target_style);
            }

            if !updates.segments_off.is_empty() {
                self.turn_off_segments(updates.segments_off, bg_style);
            }
        }
    }

    pub fn clear_update_batch(&mut self) {
        self.update_batch.clear();
    }

    pub fn trigger_screen_update(&mut self, draw: &Draw) {
        self.grid
            .trigger_screen_update(draw, &self.update_batch, self.visible);
        self.clear_update_batch();
    }

    /************************* Old update system ******************************/

    pub fn set_active_segments(&mut self, segments: HashSet<String>) {
        self.current_active_segments = segments;
    }

    /*********************** Segment Transitions  *****************************/

    pub fn start_transition(
        &mut self,
        target_segments: &HashSet<String>,
        engine: &TransitionEngine,
        immediate: bool, // when true, all segments change at once
    ) {
        let changes = engine.generate_changes(
            &self.current_active_segments,
            target_segments,
            &self.graph,
            immediate,
        );

        self.active_transition = Some(Transition::new(changes, engine.config.frame_duration));
    }

    /***************** Grid movement *****************/

    pub fn apply_transform(&mut self, transform: &Transform2D) {
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

    pub fn print_grid_info(&self) {
        println!("<====== Grid Instance: {} ======>", self.id);
        println!("\nGrid Info:");
        println!("Location: {:?}", self.current_location);
        println!("Dimensions: {:?}", self.grid.dimensions);
        println!("Viewbox: {:?}", self.grid.viewbox);
        println!("Segment count: {}\n", self.grid.segments.len());

        // Print first few segments for inspection
        /*
        for (i, (id, segment)) in self.grid.segments.iter().take(2).enumerate() {
            println!("\nSegment {}: {}", i, id);
            println!("Position: {:?}", segment.tile_pos);
            println!("Edge type: {:?}", segment.edge_type);

            for (j, cmd) in segment.draw_commands.iter().take(2).enumerate() {
                println!("  Command {}: {:?}", j, cmd);
            }

        }*/
    }
}
