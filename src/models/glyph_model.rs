// src/render/glyph_model.rs
use nannou::prelude::*;
use std::collections::HashSet;

use crate::models::data_model::{Project, Glyph};
use crate::models::grid_model::Grid;
use crate::render::renderer::RenderableSegment;

use crate::render::RenderParams;
use crate::effects::segment_effects::SegmentEffect;

pub struct GlyphModel {
    glyph_names: Vec<String>,
    current_glyph_index: usize,
}

impl GlyphModel {
    pub fn new(project: &Project) -> Self {
        let glyph_names: Vec<String> = project.glyphs.keys().cloned().collect();
        println!("Loaded {} glyphs", glyph_names.len());

        Self {
            glyph_names,
            current_glyph_index: 0,
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

    /// Gets all segments that should be rendered for the current glyph
    pub fn get_renderable_segments<'a>(
        &self,
        project: &Project,
        grid: &'a Grid,
        params: RenderParams,
        effect: Option<&dyn SegmentEffect>,
        time: f32,
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
                        let mut element_params = if debug_flag {
                            let g = debug_color(x, y);
                            RenderParams {
                                color: rgb(0.9, g, 0.0),
                                stroke_weight: params.stroke_weight,
                            }
                        } else {
                            params.clone()
                        };

                        // Apply effect if one is provided
                        if let Some(effect) = effect {
                            element_params = effect.apply(&element_params, time);
                        }

                        segments.push(RenderableSegment {
                            element,
                            params: element_params,
                        });
                    }
                }
            }
        }
        segments
    }
}