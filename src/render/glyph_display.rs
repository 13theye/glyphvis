// src/render/glyph_display.rs
use nannou::prelude::*;
use crate::models::data_model::{Project, Glyph};
use std::collections::HashSet;
use super::{Transform2D, RenderParams};

pub struct GlyphDisplay {
    glyph_names: Vec<String>,
    current_glyph_index: usize,
    glyph_params: RenderParams,  // Only need params for active segments
}

impl GlyphDisplay {
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
}