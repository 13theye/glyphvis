// src/render/glyph_renderer.rs
use nannou::prelude::*;
use std::collections::HashSet;

use crate::models::data_model::{Project, Glyph};
use crate::models::grid_model::Grid;
use crate::render::grid_renderer::RenderableSegment;
use super::RenderParams;

pub struct GlyphRenderer {
    glyph_names: Vec<String>,
    current_glyph_index: usize,
    glyph_params: RenderParams,  // Only need params for active segments
}

impl GlyphRenderer {
    pub fn new(project: &Project) -> Self {
        let glyph_names: Vec<String> = project.glyphs.keys().cloned().collect();
        println!("Loaded {} glyphs", glyph_names.len());

        Self {
            glyph_names,
            current_glyph_index: 0,
            glyph_params: RenderParams {
                color: rgb(0.9, 0.0, 0.0),  // Bright color for active glyph segments
                stroke_weight: 10.0,
            },
        }
    }

    pub fn next_glyph(&mut self) {
        self.current_glyph_index = (self.current_glyph_index + 1) % self.glyph_names.len();
        let current_name = &self.glyph_names[self.current_glyph_index];
        println!("Showing glyph: {}", current_name);
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

    pub fn get_render_params(&self) -> &RenderParams {
        &self.glyph_params
    }

    /// Gets all segments that should be rendered for the current glyph
    pub fn get_renderable_segments<'a>(
        &self,
        project: &Project,
        grid: &'a Grid,
        debug_flag: bool,
    ) -> Vec<RenderableSegment<'a>> {
        let mut segments = Vec::new();
        let active_segment_ids = self.get_active_segments(project);
        
        let debug_color = |x: u32, y: u32| -> f32 {
            ((x + y) as f32) / (grid.height + grid.width) as f32
        };

        for y in 1..=grid.height {
            for x in 1..=grid.width {
                let elements = grid.get_elements_at(x, y);
                
                for element in elements {
                    let segment_id = format!("{},{} : {}", x, y, element.id);
                    if active_segment_ids.contains(&segment_id) {
                        let params = if debug_flag {
                            let g = debug_color(x, y);
                            RenderParams {
                                color: rgb(0.9, g, 0.0),
                                stroke_weight: self.glyph_params.stroke_weight,
                            }
                        } else {
                            self.glyph_params.clone()
                        };

                        segments.push(RenderableSegment {
                            element,
                            params,
                        });
                    }
                }
            }
        }
        
        segments
    }
}