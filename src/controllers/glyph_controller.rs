// src/controller/glyph_controller.rs
/// GlyphController coordinates between the Project and the GridInstance
/// Gets the Glyph's segment_ids from Project
/// Uses that to build foreground and background RenderableSegments from GridInstance
use std::collections::HashSet;

use crate::models::data_model::{Glyph, Project, Show};

#[derive(Default)]
pub struct GlyphController {}

impl GlyphController {
    pub fn new() -> Self {
        Self {}
    }

    // returns a blank HashSet representing zero active segments
    pub fn no_glyph(&self) -> HashSet<String> {
        HashSet::new()
    }

    pub fn get_show<'a>(&self, project: &'a Project, show_name: &str) -> Option<&'a Show> {
        project.get_show(show_name)
    }

    pub fn get_glyph_segments_by_index(
        &self,
        project: &Project,
        show_name: &str,
        index: usize,
    ) -> HashSet<String> {
        if let Some(show) = self.get_show(project, show_name) {
            let show_order = &show.show_order;
            let show_element = show_order.get(&(index as u32));
            if let Some(show_element) = show_element {
                let glyph_name = &show_element.name;
                self.get_glyph(project, glyph_name)
                    .map(|glyph| glyph.segments.iter().cloned().collect())
                    .unwrap_or_default()
            } else {
                self.no_glyph()
            }
        } else {
            self.no_glyph()
        }
    }

    pub fn get_glyph<'a>(&self, project: &'a Project, name: &str) -> Option<&'a Glyph> {
        project.get_glyph(name)
    }
}
