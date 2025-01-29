// src/controller/glyph_controller.rs
/// GlyphController coordinates between the Project and the GridInstance
/// Gets the Glyph's segment_ids from Project
/// Uses that to build foreground and background RenderableSegments from GridInstance
use std::collections::HashSet;

use crate::models::data_model::{Glyph, Project};

pub struct GlyphController {
    glyph_names: Vec<String>,
    current_glyph_index: usize,
}

impl GlyphController {
    pub fn new(project: &Project) -> Self {
        let mut glyph_names: Vec<String> = project.glyphs.keys().cloned().collect();
        glyph_names.sort();
        println!("Loaded {} glyphs", glyph_names.len());

        Self {
            glyph_names,
            current_glyph_index: 0,
        }
    }

    pub fn no_glyph(&self) -> HashSet<String> {
        HashSet::new()
    }

    pub fn next_glyph(&mut self, project: &Project) -> HashSet<String> {
        self.current_glyph_index = (self.current_glyph_index + 1) % self.glyph_names.len();
        self.get_active_segments(project, self.current_glyph_index)
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
}
