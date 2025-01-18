// src/views/grid/cached_grid.rs

// The SVG grid data structures are converted to draw commands and
// cached in the structures in this module.
// 
// Types in this module:
// DrawCommand, CachedSegment, and CachedGrid

use nannou::prelude::*;
use std::collections::{ HashMap, HashSet };

use crate::models::{ ViewBox, EdgeType, PathElement, Project };
use crate::services::svg::{parse_svg, detect_edge_type};
use crate::services::grid::*;
use crate::views::Transform2D;

const ARC_RESOLUTION: usize = 128;

// DrawCommand is a single drawing operation that has been pre-processed from
// SVG path data
#[derive(Debug, Clone)]
pub enum DrawCommand {
    Line {
        start: Point2,
        end: Point2,
        stroke_weight: f32,
        color: Rgb<f32>,
    },
    Arc {
        points: Vec<Point2>,
        stroke_weight: f32,
        color: Rgb<f32>,
    },
    Circle {
        center: Point2,
        radius: f32,
        stroke_weight: f32,
        color: Rgb<f32>,
    },
}

impl DrawCommand {
    fn apply_transform(&mut self, transform: &Transform2D) {
        match self {
            DrawCommand::Line { start, end, .. } => {
                *start = transform.apply_to_point(*start);
                *end = transform.apply_to_point(*end);
            },
            DrawCommand::Arc { points, .. } => {
                for point in points {
                    *point = transform.apply_to_point(*point);
                }
            },
            DrawCommand::Circle { center, radius, .. } => {
                *center = transform.apply_to_point(*center);
                *radius *= transform.scale;
            },
        }
    }

    fn draw (&self, draw: &Draw) {
        match self {
            DrawCommand::Line { start, end, stroke_weight, color } => {
                draw.line()
                    .start(*start)
                    .end(*end)
                    .stroke_weight(*stroke_weight)
                    .color(*color)
                    .caps_round();
            },
            DrawCommand::Arc { points, stroke_weight, color } => {
                for window in points.windows(2) {
                    if let [p1, p2] = window {
                        draw.line()
                            .start(*p1)
                            .end(*p2)
                            .stroke_weight(*stroke_weight)
                            .color(*color)
                            .caps_round();
                    }
                }
            },
            DrawCommand::Circle { center, radius, stroke_weight, color } => {
                draw.ellipse()
                    .x_y(center.x, center.y)
                    .radius(*radius)
                    .stroke(*color)
                    .stroke_weight(*stroke_weight)
                    .color(*color)
                    .caps_round();
            },
        }
    }
}


// A CachedSegmeent contains pre-processed draw commands for a segment
#[derive(Debug, Clone)]
pub struct CachedSegment {
    pub id: String,
    pub tile_pos: (u32, u32),
    pub draw_commands: Vec<DrawCommand>,
    pub original_path: PathElement,
    pub edge_type: EdgeType,
    pub transform: Transform2D,
}

impl CachedSegment {
    pub fn new(element_id: String, position: (u32, u32), path: &PathElement, 
    edge_type: EdgeType, viewbox: &ViewBox) -> Self {
        // concert PathElement to Drawcommands
        let draw_commands = CachedSegment::generate_draw_commands(path, viewbox);

        Self {
            id: element_id,
            tile_pos: position,
            draw_commands,
            original_path: path.clone(),
            edge_type,
            transform: Transform2D::default(),
        }
    }

    fn generate_draw_commands(path: &PathElement, viewbox: &ViewBox) -> Vec<DrawCommand> {
        // concert SVG PathElements to DrawCommands

        let center_x = viewbox.width / 2.0;
        let center_y = viewbox.height / 2.0;

        // transform a point from SVG to Nannou Coordinates
        let transform_point = |svg_x: f32, svg_y: f32| -> Point2 {
            let local_x = svg_x - center_x;
            let local_y = center_y - svg_y;
            pt2(local_x, local_y)
        };

        match path {
            PathElement::Line { x1, y1, x2, y2 } => {
                vec![DrawCommand::Line {
                    start: transform_point(*x1, *y1),
                    end: transform_point(*x2, *y2),
                    stroke_weight: 1.0,
                    color: rgb(0.0, 0.0, 0.0),
                }]
            },
            PathElement::Arc {
                start_x, start_y, rx, ry, 
                x_axis_rotation,large_arc, sweep, 
                end_x, end_y,
            } => {
                let start = transform_point(*start_x, *start_y);
                let end = transform_point(*end_x, *end_y);

                // no need to translate b/c rx, ry are relative measures
                let (center, start_angle, sweep_angle) = calculate_arc_center(
                    start, end, *rx, *ry, *x_axis_rotation, *large_arc, *sweep
                );

                // Calculate all points
                let points = generate_arc_points(
                    center, *rx, *ry, start_angle, sweep_angle, *x_axis_rotation, ARC_RESOLUTION
                );

                vec![DrawCommand::Arc {
                    points,
                    stroke_weight: 1.0,
                    color: rgb(0.0, 0.0, 0.0),
                }]
            },
            PathElement::Circle {
                cx, cy, r
            } => {
                vec![DrawCommand::Circle {
                    center: transform_point(*cx, *cy),
                    radius: *r,
                    stroke_weight: 1.0,
                    color: rgb(0.0, 0.0, 0.0),
                }]
            },
        }
    }

    pub fn apply_transform(&mut self, transform: &Transform2D) {
        self.transform = transform.clone();
        for command in &mut self.draw_commands {
            command.apply_transform(transform);
        }
    }
}

// CachedGrid stores the pre-processed drawing commands for an entire grid
pub struct CachedGrid {
    pub dimensions: (u32, u32),
    pub segments: HashMap<String, CachedSegment>,
    pub viewbox: ViewBox,
    pub transform: Transform2D,
    pub active_glyph: Option<String>,
    pub active_segments: HashSet<String>,
}

impl CachedGrid {
    pub fn new(project: &Project) -> Self {
        // Parse viewbox from SVG
        let viewbox = parse_viewbox(&project.svg_base_tile)
            .expect("Failed to parse viewbox from SVG");

        // Parse the SVG & create basic grid elements
        let elements = parse_svg(&project.svg_base_tile);
        let mut segments = HashMap::new();

        // Create grid elements and detect edges
        println!("\n=== Generating Grid Elements ===");
        for y in 1..=project.grid_y {
            for x in 1..=project.grid_x {
                for element in &elements {
                    let edge_type = detect_edge_type(&element.path, &viewbox);
                    let segment = CachedSegment::new(
                        format!("{},{} : {}", x, y, element.id),
                        (x, y),
                        &element.path,
                        edge_type,
                        &viewbox,
                    );
                    segments.insert(segment.id.clone(), segment);
                }
            }
        }

        // Remove overlapping segments
        segments = CachedGrid::eliminate_overlaps(segments, project.grid_x, project.grid_y);

        Self {
            dimensions: (project.grid_x, project.grid_y),
            segments,
            viewbox: viewbox,
            transform: Transform2D::default(),
            active_glyph: None,
            active_segments: HashSet::new(),
        }
    }

    // Unlike Glyphmaker, where we draw all elements and then handle selection logic, 
    // in Glyphvis we decide on whether to draw an element at the beginning.
    fn eliminate_overlaps(
        segments: HashMap<String, CachedSegment>,
        grid_width: u32,
        grid_height: u32,
    ) -> HashMap<String, CachedSegment> {
        let mut final_segments = HashMap::new();

        // Group segments by position for easier overlap checking
        let mut segments_by_pos: HashMap<(u32, u32), Vec<&CachedSegment>> = HashMap::new();
        for segment in segments.values() {
            segments_by_pos
                .entry(segment.tile_pos)
                .or_default()
                .push(segment);
        }

        // Check each segment against its potential neighbors
        for segment in segments.values() {
            // Skip if it's not an edge
            if segment.edge_type == EdgeType::None {
                final_segments.insert(segment.id.clone(), segments.get(&segment.id).unwrap().clone());
                continue;
            }

            // Get potential neighbors based on edge type
            if let Some((neighbor_x, neighbor_y)) = get_neighbor_coords(
                segment.tile_pos.0,
                segment.tile_pos.1,
                segment.edge_type,
                grid_width,
                grid_height,
            ) {
                // check if neighbor has priority
                let neighbor_has_priority = 
                    neighbor_x < segment.tile_pos.0 ||
                    (neighbor_x == segment.tile_pos.1 && neighbor_y < segment.tile_pos.1);

                if neighbor_has_priority {
                    // Look for matching segments at neighbor position
                    if let Some(neighbor_segments) = segments_by_pos.get(&(neighbor_x, neighbor_y)) {
                        let mut should_keep = true;
                        
                        for neighbor in neighbor_segments {
                            let direction = get_neighbor_direction(
                                segment.tile_pos.0,
                                segment.tile_pos.1,
                                neighbor_x,
                                neighbor_y
                            );

                            if check_segment_alignment(segment, *neighbor, direction) {
                                should_keep = false;
                                break;
                            }
                        }

                        if should_keep {
                            final_segments.insert(segment.id.clone(), segments.get(&segment.id).unwrap().clone());
                        }
                    }
                } else {
                    // This segment has priority, keep it
                    final_segments.insert(segment.id.clone(), segments.get(&segment.id).unwrap().clone());
                }
            } else {
                // No valid neighbor position, keep the segment
                final_segments.insert(segment.id.clone(), segments.get(&segment.id).unwrap().clone());
            }
        }
        // return:
        final_segments
    }

    // Rendering methods
    pub fn draw_full_grid(&self, draw: &Draw) {
        for segment in self.segments.values() {
            for command in &segment.draw_commands {
                command.draw(draw);
            }
        }
    }

    pub fn apply_transform(&mut self, transform: &Transform2D) {
        self.transform = transform.clone();
        for segment in self.segments.values_mut() {
            segment.apply_transform(transform);
        }
    }

    // Utility methods
    pub fn get_segment(&self, id: &str) -> Option<&CachedSegment> {
        self.segments.get(id)
    }

    pub fn get_segments_at(&self, x: u32, y: u32) -> Vec<&CachedSegment> {
        self.segments
            .values()
            .filter(|segment| segment.tile_pos == (x, y))
            .collect()
    }

    pub fn set_glyph(&mut self, glyph_name: Option<&str>, project: &Project) {
        self.active_glyph = glyph_name.map(String::from);
        self.active_segments = match glyph_name {
            Some(name) => project.get_glyph(name)
                .map(| glyph| glyph.segments.iter().cloned().collect())
                .unwrap_or_default(),
            None => HashSet::new(),
        };
    }

    pub fn get_active_glyph(&self) -> Option<&str> {
        self.active_glyph.as_deref()
    }

    // TODO: THIS ISN'T QUITE RIGHT
    pub fn draw(&self, draw: &Draw) {
        if !self.validate_segment_points() {
            println!("WARNING: Invalid segment points detected, skipping draw");
            return;
        }

        for segment in self.segments.values() {
            if self.active_glyph.is_none() || self.active_segments.contains(&segment.id) {
                for command in &segment.draw_commands {
                    command.draw(draw);
                }
            }
        }
    }

    fn validate_segment_points(&self) -> bool {
        for segment in self.segments.values() {
            for command in &segment.draw_commands {
                match command {
                    DrawCommand::Line { start, end, .. } => {
                        if !start.x.is_finite() || !start.y.is_finite() ||
                           !end.x.is_finite() || !end.y.is_finite() {
                            println!("Invalid line points: start={:?}, end={:?}", start, end);
                            return false;
                        }
                    },
                    DrawCommand::Arc { points, .. } => {
                        for point in points {
                            if !point.x.is_finite() || !point.y.is_finite() {
                                println!("Invalid arc point: {:?}", point);
                                return false;
                            }
                        }
                    },
                    DrawCommand::Circle { center, radius, .. } => {
                        if !center.x.is_finite() || !center.y.is_finite() || !radius.is_finite() {
                            println!("Invalid circle: center={:?}, radius={}", center, radius);
                            return false;
                        }
                    }
                }
            }
        }
        true
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    // Helper to create a test viewbox
    fn create_test_viewbox() -> ViewBox {
        ViewBox {
            min_x: 0.0,
            min_y: 0.0,
            width: 100.0,
            height: 100.0,
        }
    }

    mod draw_command_tests {
        use super::*;

        #[test]
        fn test_draw_command_transform() {
            let transform = Transform2D {
                translation: Vec2::new(10.0, 10.0),
                scale: 2.0,
                rotation: 0.0,
            };

            // Test Line transformation
            let mut line = DrawCommand::Line {
                start: pt2(0.0, 0.0),
                end: pt2(5.0, 5.0),
                stroke_weight: 1.0,
                color: rgb(0.0, 0.0, 0.0),
            };
            line.apply_transform(&transform);
            match line {
                DrawCommand::Line { start, end, .. } => {
                    assert_eq!(start, pt2(10.0, 10.0));
                    assert_eq!(end, pt2(20.0, 20.0));
                },
                _ => panic!("Wrong variant"),
            }

            // Test Circle transformation
            let mut circle = DrawCommand::Circle {
                center: pt2(0.0, 0.0),
                radius: 5.0,
                stroke_weight: 1.0,
                color: rgb(0.0, 0.0, 0.0),
            };
            circle.apply_transform(&transform);
            match circle {
                DrawCommand::Circle { center, radius, .. } => {
                    assert_eq!(center, pt2(10.0, 10.0));
                    assert_eq!(radius, 10.0);
                },
                _ => panic!("Wrong variant"),
            }
        }
    }

    mod cached_segment_tests {
        use super::*;

        #[test]
        fn test_cached_segment_creation() {
            let viewbox = create_test_viewbox();
            let path = PathElement::Line {
                x1: 0.0,
                y1: 0.0,
                x2: 10.0,
                y2: 10.0,
            };
            
            let segment = CachedSegment::new(
                "test".to_string(),
                (1, 1),
                &path,
                EdgeType::None,
                &viewbox,
            );

            assert_eq!(segment.id, "test");
            assert_eq!(segment.tile_pos, (1, 1));
            assert_eq!(segment.edge_type, EdgeType::None);
            assert!(!segment.draw_commands.is_empty());
        }

        #[test]
        fn test_coordinate_transformation() {
            let viewbox = create_test_viewbox();
            
            // Test center point transformation
            let path = PathElement::Circle {
                cx: 50.0, // Center of viewbox
                cy: 50.0,
                r: 5.0,
            };
            
            let segment = CachedSegment::new(
                "center_test".to_string(),
                (1, 1),
                &path,
                EdgeType::None,
                &viewbox,
            );

            // Center point should be transformed to (0,0) in Nannou coordinates
            match &segment.draw_commands[0] {
                DrawCommand::Circle { center, .. } => {
                    assert_eq!(center.x, 0.0);
                    assert_eq!(center.y, 0.0);
                },
                _ => panic!("Expected Circle"),
            }
        }
    }

    mod cached_grid_tests {
        use super::*;

        fn create_test_project() -> Project {
            // Create minimal project for testing
            Project {
                svg_base_tile: r#"<svg id="test" viewBox="0 0 100 100">
                    <path id="line1" d="M0,0 L100,0"/>
                    <circle id="circle1" cx="50" cy="50" r="5"/>
                </svg>"#.to_string(),
                grid_x: 2,
                grid_y: 2,
                glyphs: HashMap::new(),
                shows: HashMap::new(),
            }
        }

        #[test]
        fn test_grid_creation() {
            let project = create_test_project();
            let grid = CachedGrid::new(&project);
            
            assert_eq!(grid.dimensions, (2, 2));
            assert!(!grid.segments.is_empty());
        }

        #[test]
        fn test_overlap_elimination() {
            let project = create_test_project();
            let grid = CachedGrid::new(&project);
            
            // Test that overlapping edges are properly eliminated
            // For example, if we have a horizontal line at y=0, it should only appear
            // in either the top or bottom tile, not both
            let top_segments = grid.get_segments_at(1, 1);
            let bottom_segments = grid.get_segments_at(1, 2);
            
            // Ensure we don't have the same edge in both tiles
            let top_edges: Vec<EdgeType> = top_segments.iter()
                .map(|s| s.edge_type)
                .collect();
            let bottom_edges: Vec<EdgeType> = bottom_segments.iter()
                .map(|s| s.edge_type)
                .collect();
            
            assert!(!(top_edges.contains(&EdgeType::South) && 
                     bottom_edges.contains(&EdgeType::North)));
        }
    }
}