// src/views/grid_manager.rs

use nannou::prelude::*;
use std::collections::{HashMap, HashSet};

use crate::animation::{Transition, TransitionEngine};
use crate::effects::{init_effects, EffectsManager};
use crate::models::Project;
use crate::views::{CachedGrid, DrawStyle, Layer, RenderableSegment, SegmentGraph, Transform2D};

pub struct GridInstance {
    // grid data
    pub id: String,
    pub grid: CachedGrid,
    pub graph: SegmentGraph,

    // effects state
    pub effects_manager: EffectsManager,
    pub active_transition: Option<Transition>,

    // inside-grid state
    pub current_active_segments: HashSet<String>,
    pub current_glyph_index: usize, // temporary way to access glyphs while testing
    /*
    transition_timeline: Option<SegmentTimeline>,
    transition_start_time: Option<f32>,
     */
    // overall grid state
    pub spawn_location: Point2,
    pub spawn_rotation: f32,
    pub current_location: Point2,
    pub current_rotation: f32,
    pub visible: bool,
}

impl GridInstance {
    pub fn new(app: &App, project: &Project, id: String, position: Point2, rotation: f32) -> Self {
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
            current_glyph_index: 0,

            /* will add this when timeline is implemented
            target_active_segments: None,
            transition_timeline: None,
            transition_start_time: None,
             */
            effects_manager: init_effects::init_effects(app),
            active_transition: None,

            spawn_location: position,
            spawn_rotation: rotation,
            current_location: position,
            current_rotation: rotation,
            visible: true,
        }
    }

    pub fn set_active_segments(&mut self, segments: HashSet<String>) {
        self.current_active_segments = segments;
    }

    pub fn get_renderable_segments(
        &self,
        time: f32,
        foreground_style: &DrawStyle,
        background_style: &DrawStyle,
    ) -> Vec<RenderableSegment> {
        let mut return_segments = Vec::new();
        let (grid_x, grid_y) = self.grid.dimensions;
        let background_style = self.effects_manager.apply_grid_effects(
            "background", // Use a constant ID for background
            background_style.clone(),
            time,
        );

        for y in 1..=grid_y {
            for x in 1..=grid_x {
                let segments = self.grid.get_segments_at(x, y);

                for segment in segments {
                    if self.current_active_segments.contains(&segment.id) {
                        let base_style = foreground_style.clone();
                        let final_style = self.effects_manager.apply_segment_effects(
                            &segment.id,
                            base_style,
                            time,
                        );

                        return_segments.push(RenderableSegment {
                            segment,
                            style: final_style,
                            layer: Layer::Foreground,
                        });
                    } else {
                        return_segments.push(RenderableSegment {
                            segment,
                            style: background_style.clone(),
                            layer: Layer::Background,
                        });
                    }
                }
            }
        }

        return_segments
    }

    /***************** Segment Transitions  *****************/

    pub fn start_transition(
        &mut self,
        target_segments: HashSet<String>,
        engine: &TransitionEngine,
    ) {
        let frames =
            engine.generate_frames(&self.current_active_segments, &target_segments, &self.graph);

        self.active_transition = Some(Transition::new(frames, engine.config.frame_duration));
    }

    pub fn update(&mut self, time: f32, dt: f32) {
        if let Some(transition) = &mut self.active_transition {
            if transition.update(dt) {
                // time to advance to next frame
                if let Some(new_segments) = transition.advance() {
                    // update active segments
                    let newly_active = new_segments.difference(&self.current_active_segments);
                    for segment_id in newly_active {
                        self.effects_manager
                            .activate_segment(segment_id, "power_on", time);
                    }
                    self.current_active_segments = new_segments.clone();
                }
            }
            if transition.is_complete() {
                self.active_transition = None;
            }
        }
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

    pub fn draw_segments(&self, draw: &Draw, segments: Vec<RenderableSegment>) {
        self.grid.draw_segments(draw, &segments);
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
