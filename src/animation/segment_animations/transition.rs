// src/animation/segment_animations/transition.rs

use crate::views::SegmentGraph;
use rand::{thread_rng, Rng};
use std::collections::{HashMap, HashSet, VecDeque};

pub struct TransitionConfig {
    pub steps: usize,        // Total number of frames to generate
    pub frame_duration: f32, // Time between frame changes
    pub wandering: f32,      // How much randomness in timing (0.0-1.0)
    pub density: f32,        // How many segments can change per frame (0.0-1.0)
}

pub struct Transition {
    frames: Vec<HashSet<String>>,
    current_frame: usize,
    frame_timer: f32,
    frame_duration: f32,
}

impl Transition {
    pub fn new(frames: Vec<HashSet<String>>, frame_duration: f32) -> Self {
        Self {
            frames,
            current_frame: 0,
            frame_timer: 0.0,
            frame_duration,
        }
    }

    pub fn update(&mut self, dt: f32) -> bool {
        self.frame_timer += dt;
        if self.frame_timer >= self.frame_duration {
            self.frame_timer -= self.frame_duration;
            true
        } else {
            false
        }
    }

    pub fn advance(&mut self) -> Option<&HashSet<String>> {
        if self.current_frame < self.frames.len() {
            self.current_frame += 1;
            Some(&self.frames[self.current_frame - 1])
        } else {
            None // Transition is complete
        }
    }

    pub fn is_complete(&self) -> bool {
        self.current_frame >= self.frames.len()
    }
}

pub struct TransitionEngine {
    pub config: TransitionConfig,
}

impl TransitionEngine {
    pub fn new(config: TransitionConfig) -> Self {
        Self { config }
    }

    pub fn generate_frames(
        &self,
        start_segments: &HashSet<String>,
        target_segments: &HashSet<String>,
        segment_graph: &SegmentGraph,
    ) -> Vec<HashSet<String>> {
        let mut rng = thread_rng();
        let mut frames = vec![start_segments.clone()];
        let mut pending_changes = Vec::new();
        let mut remaining_targets = target_segments.clone();

        // For segments that need to disappear, find nearest active segment in target set
        for seg in start_segments.difference(target_segments) {
            if let Some(nearest) = self.find_nearest_connected(seg, target_segments, segment_graph)
            {
                pending_changes.push((seg.clone(), nearest, false));
            }
        }
        // For segments that need to appear, find nearest active segment in start set
        for seg in target_segments.difference(start_segments) {
            if let Some(nearest) = self.find_nearest_connected(seg, start_segments, segment_graph) {
                pending_changes.push((seg.clone(), nearest, true));
            } else if start_segments.is_empty() {
                pending_changes.push((seg.clone(), seg.clone(), true));
            }
        }
        // Distribute changes across frames based on density
        let changes_per_frame =
            (pending_changes.len() as f32 * self.config.density).ceil() as usize;

        for frame in 1..self.config.steps - 1 {
            let mut current = frames.last().unwrap().clone();

            // Select random subset of changes for this frame
            let available_changes = pending_changes.len().min(changes_per_frame);
            if available_changes > 0 {
                for _ in 0..available_changes {
                    if rng.gen::<f32>() < self.config.wandering {
                        let idx = rng.gen_range(0..pending_changes.len());
                        let (seg, _, is_add) = &pending_changes[idx];

                        if *is_add {
                            current.insert(seg.clone());
                            remaining_targets.remove(seg);
                        } else {
                            current.remove(seg);
                        }

                        pending_changes.swap_remove(idx);
                    }
                }
            }

            // If we're running out of frames, start including remaining targets
            let frames_left = self.config.steps - frame - 1;
            let targets_per_remaining_frame =
                (remaining_targets.len() as f32 / frames_left as f32).ceil() as usize;

            // Take some remaining targets and add them to current frame
            for _ in 0..targets_per_remaining_frame {
                if let Some(seg) = remaining_targets.iter().next().cloned() {
                    current.insert(seg.clone());
                    remaining_targets.remove(&seg);
                }
            }
            frames.push(current);
        }
        let mut final_frame = frames.last().unwrap().clone();
        for seg in remaining_targets.iter() {
            println!("Remaining segment: {}", seg);
            final_frame.insert(seg.clone());
        }

        frames.push(final_frame);

        frames
    }

    fn find_nearest_connected(
        &self,
        segment: &str,
        active_segments: &HashSet<String>,
        graph: &SegmentGraph,
    ) -> Option<String> {
        // Get all neighbors from the graph
        let mut visited = HashSet::new();
        let mut queue = VecDeque::new();
        queue.push_back(segment.to_string());
        visited.insert(segment.to_string());

        // Breadth-first search through connected segments
        while let Some(current) = queue.pop_front() {
            // If this neighbor is in our target set, we found our match
            if active_segments.contains(&current) && current != segment {
                return Some(current);
            }

            // Add unvisited neighbors to queue
            for neighbor in graph.get_neighbors(&current) {
                if !visited.contains(&neighbor) {
                    visited.insert(neighbor.clone());
                    queue.push_back(neighbor);
                }
            }
        }

        None // No connected segment found in active set
    }
}
