// src/controller/glyph_controller.rs
/// GlyphController coordinates between the Project and the GridInstance
/// Gets the Glyph's segment_ids from Project
/// Uses that to build foreground and background RenderableSegments from GridInstance
use std::collections::{HashMap, HashSet};

use crate::animation::TransitionEngine;
//use crate::effects::EffectsManager;
use crate::models::data_model::{Glyph, Project};
use crate::views::GridInstance;

pub struct GlyphController {
    glyph_names: Vec<String>,
}

impl GlyphController {
    pub fn new(project: &Project) -> Self {
        let mut glyph_names: Vec<String> = project.glyphs.keys().cloned().collect();
        glyph_names.sort();
        println!("Loaded {} glyphs", glyph_names.len());

        Self { glyph_names }
    }

    pub fn next_glyph(
        &mut self,
        project: &Project,
        grid_instance: &mut GridInstance,
        transition_engine: &TransitionEngine,
        current_time: f32,
    ) {
        grid_instance.current_glyph_index =
            (grid_instance.current_glyph_index + 1) % self.glyph_names.len();

        let target_segments = self.get_active_segments(project, grid_instance.current_glyph_index);

        // Instead of directly setting segments, start a transition to them
        grid_instance.start_transition(target_segments, transition_engine);
    }

    pub fn get_current_glyph<'a>(&self, project: &'a Project, index: usize) -> Option<&'a Glyph> {
        let current_name = &self.glyph_names[index];
        project.get_glyph(current_name)
    }

    pub fn get_glyph<'a>(&self, project: &'a Project, name: &str) -> Option<&'a Glyph> {
        project.get_glyph(name)
    }

    pub fn get_active_segments(&self, project: &Project, index: usize) -> HashSet<String> {
        self.get_current_glyph(project, index)
            .map(|glyph| glyph.segments.iter().cloned().collect())
            .unwrap_or_default()
    }

    pub fn update_all_grids(
        &mut self,
        grids: &mut HashMap<String, GridInstance>,
        project: &Project,
        transition_engine: &TransitionEngine,
        time: f32,
    ) {
        for grid in grids.values_mut() {
            let segment_ids: HashSet<String> =
                self.get_active_segments(project, grid.current_glyph_index);

            grid.start_transition(segment_ids, transition_engine);
        }
    }
    /*  Moving this to grid_instance
    /// Gets all segments that should be rendered for the current glyph
    pub fn
    _segments<'a>(
        &self,
        project: &Project,
        grid_instance: &'a GridInstance,
        foreground_style: &DrawStyle,
        background_style: &DrawStyle,
        effect_manager: &EffectsManager,
        time: f32,
    ) -> Vec<RenderableSegment<'a>> {
        let mut return_segments = Vec::new();
        let active_segment_ids = self.get_active_segments(project);
        let grid = &grid_instance.grid;
        let (grid_x, grid_y) = grid.dimensions;

        // iterate over tiles
        for y in 1..=grid_y {
            for x in 1..=grid_x {
                let segments = grid.get_segments_at(x, y);

                for segment in segments {
                    if active_segment_ids.contains(&segment.id) {
                        let base_style = foreground_style.clone();
                        // Apply effect if one is provided
                        let final_style =
                            effect_manager.apply_segment_effects(&segment.id, base_style, time);

                        return_segments.push(RenderableSegment {
                            segment,
                            style: final_style,
                            layer: Layer::Foreground,
                        });
                    } else {
                        let base_style = background_style.clone();
                        // Apply effect if one is provided
                        let final_style =
                            effect_manager.apply_grid_effects(&segment.id, base_style, time);

                        return_segments.push(RenderableSegment {
                            segment,
                            style: final_style,
                            layer: Layer::Background,
                        });
                    };
                }
            }
        }

        return_segments
    }
    */
}
