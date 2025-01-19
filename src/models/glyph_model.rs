// src/models/glyph_model.rs
// a structure that holds ready-to-render glyphs
// data model and constructors for Glyphs, which are on the same level as Grids.
// also applies effects at the Glyph level

use nannou::prelude::*;
use std::collections::HashSet;

use crate::models::data_model::{Project, Glyph};
use crate::views:: { CachedGrid, DrawStyle, RenderableSegment };

use crate::effects::grid_effects::GridEffect;

pub struct GlyphModel {
    glyph_names: Vec<String>,
    current_glyph_index: usize,
}

impl GlyphModel {
    pub fn new(project: &Project) -> Self {
        let mut glyph_names: Vec<String> = project.glyphs.keys().cloned().collect();
        glyph_names.sort();
        println!("Loaded {} glyphs", glyph_names.len());

        Self {
            glyph_names,
            current_glyph_index: 0,
        }
    }

    pub fn next_glyph(&mut self) {
        self.current_glyph_index = (self.current_glyph_index + 1) % self.glyph_names.len();
        //let current_name = &self.glyph_names[self.current_glyph_index];
        //println!("Showing glyph: {}", current_name);
    }

    pub fn get_current_glyph<'a>(&self, project: &'a Project) -> Option<&'a Glyph> {
        let current_name = &self.glyph_names[self.current_glyph_index];
        project.get_glyph(current_name)
    }

    pub fn get_active_segments(&self, project: &Project) -> HashSet<String> {
        self.get_current_glyph(project)
            .map(|glyph| glyph.segments.iter().cloned().collect())
            .unwrap_or_default()
    }

    /// Gets all segments that should be rendered for the current glyph
    pub fn get_renderable_segments<'a>(
        &self,
        project: &Project,
        grid: &'a CachedGrid,
        style: DrawStyle,
        effect: Option<&dyn GridEffect>,
        time: f32,
        bg_flag: bool,
        debug_flag: bool,
    ) -> Vec<RenderableSegment<'a>> {

        let mut return_segments = Vec::new();
        let active_segment_ids = self.get_active_segments(project);
        let (grid_x, grid_y) = grid.dimensions;
        
        // debug color function
        let debug_color = |x: u32, y: u32| -> f32 {
            ((x + y) as f32) / (grid_x + grid_y) as f32
        };

        // iterate over tiles
        for y in 1..=grid_y {
            for x in 1..=grid_x {
                let segments = grid.get_segments_at(x, y);
                
                for segment in segments {
                    if !bg_flag {
                        if active_segment_ids.contains(&segment.id) {
                            let mut segment_style = if debug_flag {
                                let g = debug_color(x, y);
                                DrawStyle {
                                    color: rgb(0.9, g, 0.0),
                                    stroke_weight: style.stroke_weight,
                                }
                            } else {
                                style.clone()
                            };

                            // Apply effect if one is provided
                            if let Some(effect) = effect {
                            segment_style = effect.apply(&segment_style, time);
                            }

                            return_segments.push(RenderableSegment {
                                segment,
                                style: segment_style,
                            });
                        }
                    } else {
                        if !active_segment_ids.contains(&segment.id) {
                            let mut segment_style = if debug_flag {
                                let g = debug_color(x, y);
                                DrawStyle {
                                    color: rgb(0.0, g, 1.0),
                                    stroke_weight: style.stroke_weight,
                                }
                            } else {
                                style.clone()
                            };

                            // Apply effect if one is provided
                            if let Some(effect) = effect {
                            segment_style = effect.apply(&segment_style, time);
                            }

                            return_segments.push(RenderableSegment {
                                segment,
                                style: segment_style,
                            });
                        }
                    }
                }
            }
        }
        return_segments
    }
}