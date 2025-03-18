// src/views/grid/segment_graph.rs
//
// SegmentGraph holds all the relationships between segment endpoints in a Grid.

use crate::views::{CachedGrid, DrawCommand};
use nannou::prelude::*;
use std::collections::HashMap;

const CONNECTION_THRESHOLD: f32 = 0.001; // Small threshold for floating point comparison
const VERBOSE: bool = true;

#[derive(Debug, Clone)]
pub struct SegmentConnection {
    segment_id: String,
    connection_point: Point2,
}

#[derive(Debug)]
pub struct SegmentNode {
    id: String,
    tile_pos: (u32, u32),
    commands: Vec<DrawCommand>,
    connections: Vec<SegmentConnection>,
}

impl SegmentNode {
    fn get_endpoints(&self) -> Vec<Point2> {
        let mut points = Vec::new();

        for command in &self.commands {
            match command {
                DrawCommand::Line { start, end } => {
                    points.push(*start);
                    points.push(*end);
                }
                DrawCommand::Arc { points: arc_points } => {
                    if let Some(first) = arc_points.first() {
                        points.push(*first);
                    }
                    if let Some(last) = arc_points.last() {
                        points.push(*last);
                    }
                }
                DrawCommand::Circle { center, radius: _ } => {
                    points.push(*center);
                }
            }
        }

        points
    }
}

#[derive(Debug)]
pub struct SegmentGraph {
    nodes: HashMap<String, SegmentNode>,
}

impl SegmentGraph {
    pub fn new(grid: &CachedGrid) -> Self {
        let mut nodes = HashMap::new();

        // First create nodes for each segment
        for (id, segment) in grid.segments() {
            nodes.insert(
                id.clone(),
                SegmentNode {
                    id: id.clone(),
                    tile_pos: segment.tile_coordinate(),
                    commands: segment.draw_commands().clone(),
                    connections: Vec::new(),
                },
            );
        }

        // Then find connections between segments
        let mut graph = Self { nodes };
        graph.build_connections();
        graph
    }

    fn build_connections(&mut self) {
        // Collect all SegmentNodes by tile position
        let mut nodes_by_pos: HashMap<(u32, u32), Vec<String>> = HashMap::new();
        for (id, node) in &self.nodes {
            nodes_by_pos
                .entry(node.tile_pos)
                .or_default()
                .push(id.clone());
        }

        let mut new_connections: HashMap<String, Vec<SegmentConnection>> = HashMap::new();

        // For each segment
        for (id1, segment1) in &self.nodes {
            let (x, y) = segment1.tile_pos;
            let endpoints1 = segment1.get_endpoints();

            // get segments from current and neighboring tiles
            let neighbor_positions = [
                (x, y),                   // Self
                (x.saturating_add(1), y), // Right
                (x.saturating_sub(1), y), // Left
                (x, y.saturating_add(1)), // Up
                (x, y.saturating_sub(1)), // Down
            ];

            // Check each neighbor position
            for pos in neighbor_positions {
                if let Some(neighbor_segments) = nodes_by_pos.get(&pos) {
                    for id2 in neighbor_segments {
                        if *id1 == *id2 {
                            continue;
                        }
                        if let Some(segment2) = self.nodes.get(id2) {
                            let endpoints2 = segment2.get_endpoints();

                            // Check all endpoint pairs for connections
                            for p1 in &endpoints1 {
                                for p2 in &endpoints2 {
                                    let distance = p1.distance(*p2);
                                    if distance <= CONNECTION_THRESHOLD {
                                        // Found a connection - add it to both segments
                                        let connection_point = (*p1 + *p2) / 2.0;

                                        // Add connection both ways directly
                                        new_connections.entry(id1.clone()).or_default().push(
                                            SegmentConnection {
                                                segment_id: id2.clone(),
                                                connection_point,
                                            },
                                        );
                                        /*
                                        new_connections.entry(id2.clone()).or_default().push(
                                            SegmentConnection {
                                                segment_id: id1.clone(),
                                                connection_point,
                                            },
                                        );*/
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        // Replace all connections at once
        for node in self.nodes.values_mut() {
            node.connections = new_connections.remove(&node.id).unwrap_or_default();
        }

        // Print final connections
        if VERBOSE {
            self.print_connections();
        }
    }
    /*
    fn build_connections_old(&mut self) {
        // Create a list of all segment IDs
        let segment_ids: Vec<String> = self.nodes.keys().cloned().collect();

        if VERBOSE {
            println!("\nBuilding connections:");
        }

        // For each pair of segments
        for i in 0..segment_ids.len() {
            let id1 = &segment_ids[i];
            let endpoints1 = if let Some(node) = self.nodes.get(id1) {
                node.get_endpoints()
            } else {
                continue;
            };

            for j in (i + 1)..segment_ids.len() {
                let id2 = &segment_ids[j];
                let endpoints2 = if let Some(node) = self.nodes.get(id2) {
                    node.get_endpoints()
                } else {
                    continue;
                };

                if VERBOSE {
                    println!("\nComparing {} and {}:", id1, id2);
                    println!("  Points 1: {:?}", endpoints1);
                    println!("  Points 2: {:?}", endpoints2);
                }

                // Check all endpoint pairs for connections
                for p1 in &endpoints1 {
                    for p2 in &endpoints2 {
                        let distance = p1.distance(*p2);
                        if VERBOSE {
                            println!("    Distance between {:?} and {:?}: {}", p1, p2, distance);
                        }
                        if distance <= CONNECTION_THRESHOLD {
                            if VERBOSE {
                                println!("    CONNECTION FOUND!");
                            }
                            // Found a connection - add it to both segments
                            let connection_point = (*p1 + *p2) / 2.0; // Midpoint

                            // Add connection to first segment
                            if let Some(node) = self.nodes.get_mut(id1) {
                                node.connections.push(SegmentConnection {
                                    segment_id: id2.clone(),
                                    connection_point,
                                });
                            }

                            // Add connection to second segment
                            if let Some(node) = self.nodes.get_mut(id2) {
                                node.connections.push(SegmentConnection {
                                    segment_id: id1.clone(),
                                    connection_point,
                                });
                            }
                        }
                    }
                }
            }
        }

        // Print final connections
        if VERBOSE {
            self.print_connections();
        }
    }
    */

    pub fn find_path(&self, start: &str, end: &str) -> Option<Vec<String>> {
        use std::collections::{HashSet, VecDeque};

        // Simple BFS to find path
        let mut queue = VecDeque::new();
        let mut visited = HashSet::new();
        let mut came_from: HashMap<String, String> = HashMap::new();

        queue.push_back(start.to_string());
        visited.insert(start.to_string());

        while let Some(current) = queue.pop_front() {
            if current == end {
                // Reconstruct path
                let mut path = Vec::new();
                let mut current = current;
                while current != start {
                    path.push(current.clone());
                    current = came_from.get(&current)?.clone();
                }
                path.push(start.to_string());
                path.reverse();
                return Some(path);
            }

            // Add unvisited neighbors to queue
            if let Some(node) = self.nodes.get(&current) {
                for connection in &node.connections {
                    if !visited.contains(&connection.segment_id) {
                        queue.push_back(connection.segment_id.clone());
                        visited.insert(connection.segment_id.clone());
                        came_from.insert(connection.segment_id.clone(), current.clone());
                    }
                }
            }
        }

        None // No path found
    }

    pub fn get_node(&self, id: &str) -> Option<&SegmentNode> {
        self.nodes.get(id)
    }

    pub fn get_neighbors(&self, id: &str) -> Vec<String> {
        self.nodes
            .get(id)
            .map(|node| {
                node.connections
                    .iter()
                    .map(|conn| conn.segment_id.clone())
                    .collect()
            })
            .unwrap_or_default()
    }

    // Debug helper
    pub fn print_connections(&self) {
        println!("\nSegment Graph Connections:");
        for (id, node) in &self.nodes {
            println!("Segment {}: {} connections", id, node.connections.len());
            for conn in &node.connections {
                println!("  -> {} at {:?}", conn.segment_id, conn.connection_point);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Helper to create test graphs
    fn create_test_graph() -> SegmentGraph {
        let mut nodes = HashMap::new();

        // Simple path with gaps - each line about 30 units long with 4 unit gaps
        let commands_a = vec![DrawCommand::Line {
            start: pt2(0.0, 50.0),
            end: pt2(28.0, 50.0),
        }];

        let commands_b = vec![DrawCommand::Line {
            start: pt2(28.0, 50.0),
            end: pt2(68.0, 50.0),
        }];

        let commands_c = vec![DrawCommand::Line {
            start: pt2(68.0, 50.0),
            end: pt2(100.0, 50.0),
        }];

        nodes.insert(
            "A".to_string(),
            SegmentNode {
                id: "A".to_string(),
                tile_pos: (1, 1),
                commands: commands_a,
                connections: Vec::new(),
            },
        );

        nodes.insert(
            "B".to_string(),
            SegmentNode {
                id: "B".to_string(),
                tile_pos: (1, 1),
                commands: commands_b,
                connections: Vec::new(),
            },
        );

        nodes.insert(
            "C".to_string(),
            SegmentNode {
                id: "C".to_string(),
                tile_pos: (1, 1),

                commands: commands_c,
                connections: Vec::new(),
            },
        );

        let mut graph = SegmentGraph { nodes };
        graph.build_connections();
        graph
    }

    fn create_complex_test_graph() -> SegmentGraph {
        let mut nodes = HashMap::new();

        // Create a T-junction with:
        // - Horizontal line "H1" connecting to "H2"
        // - Vertical line "V" intersecting at the connection point
        // - Arc "A1" connecting to both "H1" and "V"

        // Horizontal line segment 1 (left side)
        let commands_h1 = vec![DrawCommand::Line {
            start: pt2(0.0, 50.0),
            end: pt2(48.0, 50.0),
        }];

        // Horizontal line segment 2 (right side)
        let commands_h2 = vec![DrawCommand::Line {
            start: pt2(50.0, 50.0),
            end: pt2(100.0, 50.0),
        }];

        // Vertical line intersecting at (50.0, 50.0)
        let commands_v = vec![DrawCommand::Line {
            start: pt2(52.0, 50.0),
            end: pt2(50.0, 50.0),
        }];

        // Quarter-circle arc connecting to H1 and V
        let arc_points = (0..=10)
            .map(|i| {
                let t = i as f32 / 10.0;
                let radius = 50.0;
                let x = t * radius;
                let y = 50.0 + (radius * radius - x * x).sqrt();
                pt2(x, y)
            })
            .collect();

        let commands_a1 = vec![DrawCommand::Arc { points: arc_points }];

        // Insert all nodes
        nodes.insert(
            "H1".to_string(),
            SegmentNode {
                id: "H1".to_string(),
                tile_pos: (1, 1),

                commands: commands_h1,
                connections: Vec::new(),
            },
        );

        nodes.insert(
            "H2".to_string(),
            SegmentNode {
                id: "H2".to_string(),
                tile_pos: (1, 1),

                commands: commands_h2,
                connections: Vec::new(),
            },
        );

        nodes.insert(
            "V".to_string(),
            SegmentNode {
                id: "V".to_string(),
                tile_pos: (1, 1),

                commands: commands_v,
                connections: Vec::new(),
            },
        );

        nodes.insert(
            "A1".to_string(),
            SegmentNode {
                id: "A1".to_string(),
                tile_pos: (1, 1),

                commands: commands_a1,
                connections: Vec::new(),
            },
        );

        let mut graph = SegmentGraph { nodes };
        graph.build_connections();
        graph
    }

    #[test]
    fn test_simple_segment_connections() {
        let graph = create_test_graph();

        // Check if A connects to B
        let node_a = graph.get_node("A").unwrap();
        assert_eq!(node_a.connections.len(), 1);
        assert_eq!(node_a.connections[0].segment_id, "B");

        // Check if B connects to both A and C
        let node_b = graph.get_node("B").unwrap();
        assert_eq!(node_b.connections.len(), 2);

        // Check if C connects to B
        let node_c = graph.get_node("C").unwrap();
        assert_eq!(node_c.connections.len(), 1);
        assert_eq!(node_c.connections[0].segment_id, "B");
    }

    #[test]
    fn test_simple_path_finding() {
        let graph = create_test_graph();

        // Test path from A to C
        let path = graph.find_path("A", "C").unwrap();
        assert_eq!(path, vec!["A", "B", "C"]);

        // Test path from C to A
        let path = graph.find_path("C", "A").unwrap();
        assert_eq!(path, vec!["C", "B", "A"]);
    }

    #[test]
    fn test_complex_connections() {
        let graph = create_complex_test_graph();

        // Test T-junction connections
        let node_h1 = graph.get_node("H1").unwrap();
        assert_eq!(node_h1.connections.len(), 0); // Connects to nothing

        let node_h2 = graph.get_node("H2").unwrap();
        assert_eq!(node_h2.connections.len(), 1); // Connects to V

        let node_v = graph.get_node("V").unwrap();
        assert_eq!(node_v.connections.len(), 2); // Connects to H2 and A1

        // Test arc connections
        let node_a1 = graph.get_node("A1").unwrap();
        assert_eq!(node_a1.connections.len(), 1); // Connects to V
    }

    #[test]
    fn test_complex_path_finding() {
        let graph = create_complex_test_graph();

        // Test path through T-junction
        let path = graph.find_path("H2", "A1").unwrap();
        assert_eq!(path.len(), 2); // Should find path H2 -> V -> A1

        // Test path using arc
        let path = graph.find_path("A1", "V").unwrap();
        assert!(path.len() <= 3); // Should find either H1 -> H2 -> V or H1 -> A1 -> V
    }
}
