// src/animation/segment_animations/glyph_transition.rs

use rand::{thread_rng, Rng};
use std::collections::{HashMap, HashSet};

use crate::animation::SegmentAnimation;
use crate::models::Project;

#[derive(Debug)]
struct SegmentPath {
    positions: Vec<(u32, u32)>, // List of grid positions to visit
    current_step: usize,        // Current position in path
    active: bool,               // Whether segment should be visible
}

pub struct GlyphTransition {
    // Configuration
    start_glyph: String,
    end_glyph: String,
    wandering: f32,     // 0.0 to 1.0, how much segments can deviate
    steps: usize,       // Total steps in animation
    step_duration: f32, // Time between steps

    // State
    segment_paths: HashMap<String, SegmentPath>,
    current_time: f32,
    start_time: Option<f32>,
    finished: bool,
}

impl GlyphTransition {
    pub fn new(
        project: &Project,
        start_glyph: &str,
        end_glyph: &str,
        wandering: f32,
        steps: usize,
        step_duration: f32,
    ) -> Self {
        let mut transition = Self {
            start_glyph: start_glyph.to_string(),
            end_glyph: end_glyph.to_string(),
            wandering: wandering.clamp(0.0, 1.0),
            steps,
            step_duration,
            segment_paths: HashMap::new(),
            current_time: 0.0,
            start_time: None,
            finished: false,
        };

        transition.initialize_paths(project);
        transition
    }

    fn initialize_paths(&mut self, project: &Project) {
        let mut rng = thread_rng();

        // Get start and end segment sets
        let start_segments = if let Some(glyph) = project.get_glyph(&self.start_glyph) {
            glyph
                .get_parsed_segments()
                .into_iter()
                .collect::<HashSet<_>>()
        } else {
            HashSet::new()
        };

        let end_segments = if let Some(glyph) = project.get_glyph(&self.end_glyph) {
            glyph
                .get_parsed_segments()
                .into_iter()
                .collect::<HashSet<_>>()
        } else {
            HashSet::new()
        };

        // Create paths for all segments
        for (col, row, id) in start_segments.union(&end_segments).cloned() {
            let in_start = start_segments.contains(&(col, row, id.clone()));
            let in_end = end_segments.contains(&(col, row, id.clone()));

            let mut path = Vec::new();

            if in_start && in_end {
                // Segment exists in both - maybe add some wandering for visual interest
                if self.wandering > 0.0 {
                    path = self.generate_wandering_path((col, row), (col, row));
                } else {
                    path = vec![(col, row); self.steps];
                }
            } else if in_start {
                // Segment needs to disappear - find a neighboring segment to merge into
                let end_pos = self.find_nearest_end_position((col, row), &end_segments);
                path = self.generate_wandering_path((col, row), end_pos);
            } else if in_end {
                // Segment needs to appear - find a segment to split from
                let start_pos = self.find_nearest_start_position((col, row), &start_segments);
                path = self.generate_wandering_path(start_pos, (col, row));
            }

            self.segment_paths.insert(
                format!("{},{} : {}", col, row, id),
                SegmentPath {
                    positions: path,
                    current_step: 0,
                    active: in_start,
                },
            );
        }
    }

    fn generate_wandering_path(&self, start: (u32, u32), end: (u32, u32)) -> Vec<(u32, u32)> {
        let mut rng = thread_rng();
        let mut path = Vec::with_capacity(self.steps);

        // Start with direct path
        for i in 0..self.steps {
            let progress = i as f32 / (self.steps - 1) as f32;
            let base_x = start.0 as f32 + (end.0 as f32 - start.0 as f32) * progress;
            let base_y = start.1 as f32 + (end.1 as f32 - start.1 as f32) * progress;

            // Add random deviation based on wandering parameter
            let deviation = self.wandering * (1.0 - (2.0 * progress - 1.0).abs());
            let dx = (rng.gen::<f32>() - 0.5) * deviation;
            let dy = (rng.gen::<f32>() - 0.5) * deviation;

            path.push(((base_x + dx).round() as u32, (base_y + dy).round() as u32));
        }

        path
    }

    fn find_nearest_end_position(
        &self,
        pos: (u32, u32),
        end_segments: &HashSet<(u32, u32, String)>,
    ) -> (u32, u32) {
        // Simple Manhattan distance for now
        end_segments
            .iter()
            .map(|(x, y, _)| (*x, *y))
            .min_by_key(|&(x, y)| {
                ((x as i32 - pos.0 as i32).abs() + (y as i32 - pos.1 as i32).abs()) as u32
            })
            .unwrap_or(pos)
    }

    fn find_nearest_start_position(
        &self,
        pos: (u32, u32),
        start_segments: &HashSet<(u32, u32, String)>,
    ) -> (u32, u32) {
        // Simple Manhattan distance for now
        start_segments
            .iter()
            .map(|(x, y, _)| (*x, *y))
            .min_by_key(|&(x, y)| {
                ((x as i32 - pos.0 as i32).abs() + (y as i32 - pos.1 as i32).abs()) as u32
            })
            .unwrap_or(pos)
    }
}

impl SegmentAnimation for GlyphTransition {
    fn update(&mut self, time: f32) -> bool {
        if self.start_time.is_none() {
            self.start_time = Some(time);
            return false;
        }

        let elapsed = time - self.start_time.unwrap();
        let current_step = (elapsed / self.step_duration) as usize;

        if current_step >= self.steps {
            self.finished = true;
            return true;
        }

        // Update all segment positions
        for path in self.segment_paths.values_mut() {
            path.current_step = current_step;
        }

        false
    }

    fn get_active_segments(&self, _time: f32) -> HashSet<String> {
        let mut active_segments = HashSet::new();

        for (id, path) in &self.segment_paths {
            if path.active {
                active_segments.insert(id.clone());
            }
        }

        active_segments
    }

    fn reset(&mut self) {
        self.start_time = None;
        self.finished = false;
        for path in self.segment_paths.values_mut() {
            path.current_step = 0;
        }
    }

    fn is_finished(&self) -> bool {
        self.finished
    }
}
