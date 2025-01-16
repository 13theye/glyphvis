// src/models/data_model.rs
// the JSON-based project data model

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use std::fs;
use std::path::Path;

use std::error::Error;

#[derive(Debug, Serialize, Deserialize)]
pub struct Project {
    #[serde(rename = "svgBaseTile")]
    pub svg_base_tile: String,
    #[serde(rename = "gridX")]
    pub grid_x: u32,
    #[serde(rename = "gridY")]
    pub grid_y: u32,
    pub glyphs: HashMap<String, Glyph>,
    pub shows: HashMap<String, Show>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Glyph {
    pub name: String,
    pub segments: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Show {
    pub name: String,
    pub metadata: HashMap<String, serde_json::Value>,
    #[serde(rename = "showOrder")]
    pub show_order: HashMap<String, ShowElement>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ShowElement {
    pub name: String,
    #[serde(rename = "type")]
    pub element_type: String,
    pub position: u32,
    pub metadata: HashMap<String, serde_json::Value>,
}


impl Project {
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self, Box<dyn Error>> {
        let content = fs::read_to_string(path)?;
        let project: Project = serde_json::from_str(&content)?;
        Ok(project)
    }

    pub fn get_glyph(&self, name: &str) -> Option<&Glyph> {
        self.glyphs.get(name)
    }

    pub fn get_show(&self, name: &str) -> Option<&Show> {
        self.shows.get(name)
    }
}

impl Glyph {
    /// parse a segment string into its components
    /// format: "col, row : segment_type"
    pub fn parse_segment(segment: &str) -> Option<(u32, u32, String)> {
        let parts: Vec<&str> = segment.split(" : ").collect();
        if parts.len() != 2 {
            return None;
        }

        let coords: Vec<&str> = parts[0].split(',').collect();
        if coords.len() != 2 {
            return None;
        }

        let col = coords[0].trim().parse().ok()?;
        let row = coords[1].trim().parse().ok()?;
        let segment_type = parts[1].trim().to_string();

        Some((col, row, segment_type))
    }

    pub fn get_parsed_segments(&self) -> Vec<(u32, u32, String)> {
        self.segments
            .iter()
            .filter_map(|s| Self::parse_segment(s))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_segment() {
        let segment = "1,2 : ver-3-1";
        let parsed = Glyph::parse_segment(segment);
        assert_eq!(parsed, Some((1, 2, "ver-3-1".to_string())));
    }

    #[test]
    fn test_invalid_segment() {
        let segment = "1,2:ver-3-1";
        let parsed = Glyph::parse_segment(segment);
        assert_eq!(parsed, None);
    }
}


