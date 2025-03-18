// src/animation/transition.rs
//
// The Glyph Transition Manager
//
// A Transition a timeline of on/off msgs that makes the Grid
// tranistion from one Glyph to another.
// It doesn't need to finish to smoothly start transitioning to
// the next glyph.

use crate::{config::TransitionConfig, services::SegmentGraph, views::GridInstance};
use rand::{thread_rng, Rng};
use std::collections::{HashSet, VecDeque};

pub struct TransitionUpdates {
    pub segments_on: HashSet<String>,
    pub segments_off: HashSet<String>,
}

#[derive(Debug)]
pub struct SegmentChange {
    pub segment_id: String,
    pub turn_on: bool,
}

pub struct Transition {
    changes: Vec<Vec<SegmentChange>>,
    current_step: usize,
    frame_timer: f32,
    frame_duration: f32,
}

impl Transition {
    pub fn new(changes: Vec<Vec<SegmentChange>>, frame_duration: f32) -> Self {
        Self {
            changes,
            current_step: 0,
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

    pub fn advance(&mut self) -> Option<TransitionUpdates> {
        if self.current_step < self.changes.len() {
            let current_changes = &self.changes[self.current_step];

            let mut segments_on = HashSet::new();
            let mut segments_off = HashSet::new();

            // Process all changes for this step
            for change in current_changes {
                if change.turn_on {
                    segments_on.insert(change.segment_id.clone());
                } else {
                    segments_off.insert(change.segment_id.clone());
                }
            }

            self.current_step += 1;
            Some(TransitionUpdates {
                segments_on,
                segments_off,
            })
        } else {
            None
        }
    }

    pub fn is_complete(&self) -> bool {
        self.current_step >= self.changes.len()
    }
}

pub struct TransitionEngine {
    pub default_config: TransitionConfig,
}

// The thing that generates the Transition
impl TransitionEngine {
    pub fn new(config: TransitionConfig) -> Self {
        Self {
            default_config: config,
        }
    }

    pub fn get_default_config(&self) -> &TransitionConfig {
        &self.default_config
    }

    pub fn generate_changes(
        &self,
        grid_instance: &GridInstance,
        target_segments: &HashSet<String>,
        immediate: bool, // when true, all segments change at once
    ) -> Vec<Vec<SegmentChange>> {
        let grid = grid_instance.grid();
        let start_segments = grid_instance.current_active_segments();
        let target_style = grid_instance.target_style();
        let segment_graph = grid_instance.graph();

        let config = if let Some(config) = grid_instance.transition_config() {
            config
        } else {
            &self.default_config
        };

        let mut rng = thread_rng();
        let mut changes_by_step: Vec<Vec<SegmentChange>> =
            (0..config.steps).map(|_| Vec::new()).collect();
        let mut pending_changes = Vec::new();

        // For segments that need to disappear
        for seg in start_segments.difference(target_segments) {
            if let Some(nearest) = self.find_nearest_connected(seg, start_segments, segment_graph) {
                pending_changes.push((seg.clone(), nearest, false));
            } else if target_segments.is_empty() {
                pending_changes.push((seg.clone(), seg.clone(), false));
            }
        }

        let mut filtered_segments = target_segments.clone();
        // Filter out segments that are already in the target state and have the same style
        if !immediate {
            filtered_segments.retain(|seg| {
                let current_style = grid.segments()[seg].get_current_style();
                if current_style == *target_style {
                    false // Remove if styles match
                } else {
                    true // Keep if no previous style
                }
            });
        }

        // For segments that need to appear
        for seg in filtered_segments {
            if let Some(nearest) = self.find_nearest_connected(&seg, start_segments, segment_graph)
            {
                pending_changes.push((seg.clone(), nearest, true));
            } else if start_segments.is_empty() {
                pending_changes.push((seg.clone(), seg.clone(), true));
            }
        }

        if immediate {
            let mut single_step = Vec::new();
            for (seg, _, is_add) in pending_changes {
                single_step.push(SegmentChange {
                    segment_id: seg,
                    turn_on: is_add,
                });
            }
            return vec![single_step];
        }

        // Calculate changes per step based on density
        let changes_per_step = (pending_changes.len() as f32 * config.density).ceil() as usize;

        // Distribute changes across steps, keeping neighbor groups together
        for step_changes in changes_by_step.iter_mut().take(config.steps) {
            let available_changes = pending_changes.len().min(changes_per_step);
            let mut changes_this_step = 0;

            while changes_this_step < available_changes && !pending_changes.is_empty() {
                if rng.gen::<f32>() < config.wandering {
                    // Find a random change and its neighbors
                    let idx = rng.gen_range(0..pending_changes.len());
                    let (seg, nearest, is_add) = pending_changes.remove(idx);

                    // Add the change
                    step_changes.push(SegmentChange {
                        segment_id: seg.clone(),
                        turn_on: is_add,
                    });
                    changes_this_step += 1;

                    // Try to add its neighbors in the same step
                    pending_changes.retain(|(neighbor_seg, neighbor_nearest, neighbor_is_add)| {
                        if *neighbor_nearest == nearest && changes_this_step < available_changes {
                            step_changes.push(SegmentChange {
                                segment_id: neighbor_seg.clone(),
                                turn_on: *neighbor_is_add,
                            });
                            changes_this_step += 1;
                            false // Remove from pending_changes
                        } else {
                            true // Keep in pending_changes
                        }
                    });
                }
            }
        }

        // Any remaining changes go in the last step
        if !pending_changes.is_empty() {
            if let Some(last) = changes_by_step.last_mut() {
                for (seg, _, is_add) in pending_changes {
                    last.push(SegmentChange {
                        segment_id: seg,
                        turn_on: is_add,
                    });
                }
            }
        }

        // Remove any empty steps at the end
        while let Some(true) = changes_by_step.last().map(|step| step.is_empty()) {
            changes_by_step.pop();
        }
        changes_by_step
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
