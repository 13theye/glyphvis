// src/views/grid/cached_grid.rs

// The SVG grid data structures are converted to draw commands and
// cached in the structures in this module.

use nannou::prelude::*;
use std::collections::HashMap;

use crate::models::{ ViewBox, EdgeType, PathElement, Project };
use crate::services::svg::{self, detect_edge_type};
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
    id: String,
    tile_pos: (u32, u32),
    draw_commands: Vec<DrawCommand>,
    original_path: PathElement,
    edge_type: EdgeType,
    transform: Transform2D,
}

impl CachedSegment {
    fn new(element_id: String, position: (u32, u32), path: &PathElement, 
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
    dimensions: (u32, u32),
    segments: HashMap<String, CachedSegment>,
    viewbox: ViewBox,
    transform: Transform2D,
}

impl CachedGrid {
    pub fn new(project: &Project) -> Self {
        // Parse viewbox from SVG
        let viewbox = parse_viewbox(&project.svg_base_tile)
            .expect("Failed to parse viewbox from SVG");

        // Parse the SVG & create basic grid elements
        let elements = svg::parser::parse_svg(&project.svg_base_tile);
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
    pub fn draw(&self, draw: &Draw) {
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
}

// Parse viewbox helper function (moved from grid_model.rs)
fn parse_viewbox(svg_content: &str) -> Option<ViewBox> {
    let viewbox_data: Vec<String> = svg_content
        .lines()
        .filter(|line| line.contains("<svg id"))
        .filter_map(|line| {
            if let Some(viewbox_start) = line.find("viewBox=\"") {
                if let Some(viewbox_end) = line[viewbox_start + 9..].find('\"') {
                    return Some(line[viewbox_start + 9..viewbox_start + 9 + viewbox_end].to_string());
                }
            }
            None
        })
        .collect();

    viewbox_data.get(0)
        .and_then(|data| {
            let viewbox_values: Vec<f32> = data
                .split(' ')
                .filter_map(|value| value.parse::<f32>().ok())
                .collect();
            
            if viewbox_values.len() == 4 {
                Some(ViewBox {
                    min_x: viewbox_values[0],
                    min_y: viewbox_values[1],
                    width: viewbox_values[2],
                    height: viewbox_values[3],
                })
            } else {
                None
            }
        })
}

// Calculate the center, start angle, and sweep angle for an SVG arc
fn generate_arc_points(
    center: Point2, rx: f32, ry: f32, start_angle: f32, sweep_angle: f32, 
    x_axis_rotation: f32, resolution: usize,
) -> Vec<Point2> {
    let mut points = Vec::with_capacity(resolution + 1);
    
    for i in 0..=resolution {
        let t = i as f32 / resolution as f32;
        let angle = start_angle + t * sweep_angle;
        
        // Calculate point with proper radii and rotation
        let x = center.x + rx * (angle.cos() * x_axis_rotation.to_radians().cos() - 
                                        angle.sin() * x_axis_rotation.to_radians().sin());
        let y = center.y + ry * (angle.cos() * x_axis_rotation.to_radians().sin() + 
                                        angle.sin() * x_axis_rotation.to_radians().cos());
        
        points.push(pt2(x, y));
    }
    //return:
    points
}

fn calculate_arc_center(
    start: Point2,
    end: Point2,
    rx: f32,
    ry: f32,
    x_axis_rotation: f32,
    large_arc: bool,
    sweep: bool
) -> (Point2, f32, f32) {  // Returns (center, start_angle, sweep_angle)
    let debug_flag = false;
    if debug_flag {println!("\nCenter calculation:");}

    // Step 1: Transform to origin and unrotated coordinates
    let dx = (start.x - end.x) / 2.0;
    let dy = (start.y - end.y) / 2.0;

    if debug_flag {println!("  dx, dy: {:.2}, {:.2}", dx, dy);}
    
    let angle_rad = x_axis_rotation.to_radians();
    let cos_phi = angle_rad.cos();
    let sin_phi = angle_rad.sin();
    
    // Rotate to align with axes
    let x1p = cos_phi * dx + sin_phi * dy;
    let y1p = -sin_phi * dx + cos_phi * dy;

    if debug_flag {println!("  x1p, y1p: {:.2}, {:.2}", x1p, y1p);}

    
    // Step 2: Ensure radii are large enough
    let rx_sq = rx * rx;
    let ry_sq = ry * ry;
    let x1p_sq = x1p * x1p;
    let y1p_sq = y1p * y1p;
    
    let radii_check = x1p_sq / rx_sq + y1p_sq / ry_sq;
    let (rx_final, ry_final) = if radii_check > 1.0 {
        let sqrt_scale = radii_check.sqrt();
        (rx * sqrt_scale, ry * sqrt_scale)
    } else {
        (rx, ry)
    };
    
    // Step 3: Calculate center parameters
    let rx_sq = rx_final * rx_final;
    let ry_sq = ry_final * ry_final;
    
    let term = (rx_sq * ry_sq - rx_sq * y1p_sq - ry_sq * x1p_sq) / 
                (rx_sq * y1p_sq + ry_sq * x1p_sq);
                
    let s = if term <= 0.0 { 0.0 } else { term.sqrt() };

    if debug_flag {
        println!("  term: {:.2}", term);
        println!("  s: {:.2}", s);
    }
    
    // Choose center based on sweep and large-arc flags
    let cxp = s * rx_final * y1p / ry_final;
    let cyp = -s * ry_final * x1p / rx_final;

    if debug_flag{println!("  cxp, cyp before flip: {:.2}, {:.2}", cxp, cyp);}
    
    // Handle sweep flag to make it clockwise by flipping the center.
    let (cxp, cyp) = if sweep {
        (-cxp, -cyp)
    } else {
        (cxp, cyp)
    };

    if debug_flag{println!("  cxp, cyp after sweep: {:.2}, {:.2}", cxp, cyp);}

    // Step 4: Transform center back to original coordinate space
    let cx = cos_phi * cxp - sin_phi * cyp + (start.x + end.x) / 2.0;
    let cy = sin_phi * cxp + cos_phi * cyp + (start.y + end.y) / 2.0;

    if debug_flag {println!("  final center: ({:.2}, {:.2})", cx, cy);}
    
    // Step 5: Calculate angles
    let start_vec_x = (x1p - cxp) / rx_final;
    let start_vec_y = (y1p - cyp) / ry_final;
    let end_vec_x = (-x1p - cxp) / rx_final;
    let end_vec_y = (-y1p - cyp) / ry_final;
    
    let start_angle = (start_vec_y).atan2(start_vec_x);
    let mut sweep_angle = (end_vec_y).atan2(end_vec_x) - start_angle;

    if debug_flag {
        println!("  start_angle: {:.2}° ({:.2} rad)", start_angle.to_degrees(), start_angle);
        println!("  sweep_angle: {:.2}° ({:.2} rad)", sweep_angle.to_degrees(), sweep_angle);
    }
    
    // Ensure sweep angle matches flags
    if !sweep && sweep_angle > 0.0 {
        sweep_angle -= 2.0 * PI;
    } else if sweep && sweep_angle < 0.0 {
        sweep_angle += 2.0 * PI;
    }
    
    // Force the short path for !large_arc
    if !large_arc && sweep_angle.abs() > PI {
        sweep_angle = if sweep_angle > 0.0 {
            sweep_angle - 2.0 * PI
        } else {
            sweep_angle + 2.0 * PI
        };
    }

    // return:
    (pt2(cx, cy), start_angle, sweep_angle)
}

// Helper functions for overlap checking
/// Checks if two paths overlap based on their edge types and positions
fn check_segment_alignment(
    segment1: &CachedSegment,
    segment2: &CachedSegment,
    direction: Option<&str>
) -> bool {

    let edge_type1 = segment1.edge_type;
    let edge_type2 = segment2.edge_type;

    // Check if segments align based on their edge types and positions
    let types_match = match edge_type1 {
        EdgeType::North => edge_type2 == EdgeType::South,
        EdgeType::South => edge_type2 == EdgeType::North,
        EdgeType::East => edge_type2 == EdgeType::West,
        EdgeType::West => edge_type2 == EdgeType::East,
        EdgeType::Northwest => matches!(
            (direction, edge_type2),
            (Some("Northwest"), EdgeType::Southeast) |
            (Some("West"), EdgeType::Northeast) |
            (Some("North"), EdgeType::Southwest)
        ),
        EdgeType::Northeast => matches!(
            (direction, edge_type2),
            (Some("North"), EdgeType::Southeast) |
            (Some("East"), EdgeType::Northwest) |
            (Some("Northeast"), EdgeType::Southwest)
        ),
        EdgeType::Southwest => matches!(
            (direction, edge_type2),
            (Some("West"), EdgeType::Southeast) |
            (Some("Southwest"), EdgeType::Northeast) |
            (Some("South"), EdgeType::Northwest)
        ),
        EdgeType::Southeast => matches!(
            (direction, edge_type2),
            (Some("East"), EdgeType::Southwest) |
            (Some("South"), EdgeType::Northeast) |
            (Some("Southeast"), EdgeType::Northwest)
        ),
        EdgeType::None => false
    };

    if !types_match {
        return false;
    } else {
        let path1 = &segment1.original_path;
        let path2 = &segment2.original_path;
        // then check coordinate alignment
        match (path1, path2) {
            (PathElement::Line { x1: x1a, y1: y1a, x2: x2a, y2: y2a },
             PathElement::Line { x1: x1b, y1: y1b, x2: x2b, y2: y2b }) => {
                match edge_type1 {
                    EdgeType::North | EdgeType::South => {
                        let matches_forward = x1a == x1b && x2a == x2b;
                        let matches_reversed = x1a == x2b && x2a == x1b;
                        matches_forward || matches_reversed
                    },
                    EdgeType::East | EdgeType::West => {
                        let matches_forward = y1a == y1b && y2a == y2b;
                        let matches_reversed = y1a == y2b && y2a == y1b;
                        matches_forward || matches_reversed
                    },
                    _ => false
                }
            },
            (PathElement::Circle { cx: cxa, cy: cya, .. },
             PathElement::Circle { cx: cxb, cy: cyb, .. }) => {
                match edge_type1 {
                    EdgeType::North | EdgeType::South => cxa == cxb,
                    EdgeType::East | EdgeType::West => cya == cyb,
                    EdgeType::Northwest | EdgeType::Northeast |
                    EdgeType::Southwest | EdgeType::Southeast => {
                        // For corners, check if centers align based on position
                        match direction {
                            Some("East") | Some("West") => cya == cyb,
                            Some("North") | Some("South") => cxa == cxb,
                            _ => true  // For direct diagonal neighbors, already checked edge type
                        }
                    },
                    EdgeType::None => false
                }
            },
            _ => false // Arcs never overlap
        }
    }
}

fn get_neighbor_coords(x: u32, y: u32, edge_type: EdgeType, width: u32, height: u32) 
    -> Option<(u32, u32)> 
{
    match edge_type {
        EdgeType::North => if y > 1 { Some((x, y - 1)) } else { None },
        EdgeType::South => if y < height { Some((x, y + 1)) } else { None },
        EdgeType::East => if x < width { Some((x + 1, y)) } else { None },
        EdgeType::West => if x > 1 { Some((x - 1, y)) } else { None },
        // Add cases for corners
        _ => None
    }
}

fn get_neighbor_direction(x: u32, y: u32, neighbor_x: u32, neighbor_y: u32) -> Option<&'static str> {
    match (
        neighbor_x as i32 - x as i32,
        neighbor_y as i32 - y as i32
    ) {
        (0, -1) => Some("North"),
        (0, 1) => Some("South"),
        (1, 0) => Some("East"),
        (-1, 0) => Some("West"),
        _ => None
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use std::f32::consts::PI;

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

    mod helper_function_tests {
        use super::*;

        #[test]
        fn test_get_neighbor_coords() {
            let tests = vec![
                // Format: (x, y, edge_type, width, height, expected)
                (2, 2, EdgeType::North, 4, 4, Some((2, 1))),
                (2, 2, EdgeType::South, 4, 4, Some((2, 3))),
                (2, 2, EdgeType::East, 4, 4, Some((3, 2))),
                (2, 2, EdgeType::West, 4, 4, Some((1, 2))),
                // Test edge cases
                (1, 1, EdgeType::West, 4, 4, None),
                (1, 1, EdgeType::North, 4, 4, None),
                (4, 4, EdgeType::South, 4, 4, None),
                (4, 4, EdgeType::East, 4, 4, None),
            ];

            for (x, y, edge_type, width, height, expected) in tests {
                let result = get_neighbor_coords(x, y, edge_type, width, height);
                assert_eq!(result, expected, 
                    "Failed for x:{}, y:{}, edge_type:{:?}", x, y, edge_type);
            }
        }

        #[test]
        fn test_get_neighbor_direction() {
            let tests = vec![
                ((1, 1), (1, 0), Some("North")),
                ((1, 1), (1, 2), Some("South")),
                ((1, 1), (2, 1), Some("East")),
                ((1, 1), (0, 1), Some("West")),
                ((1, 1), (2, 2), None),  // Diagonal
            ];

            for ((x, y), (nx, ny), expected) in tests {
                let result = get_neighbor_direction(x, y, nx, ny);
                assert_eq!(result, expected,
                    "Failed for ({}, {}) -> ({}, {})", x, y, nx, ny);
            }
        }

        #[test]
        fn test_check_segment_alignment() {
            // Test basic edge alignments
            let segment1 = CachedSegment::new(
                "test1".to_string(),
                (1, 1),
                &PathElement::Line { x1: 0.0, y1: 0.0, x2: 10.0, y2: 0.0 },
                EdgeType::North,
                &create_test_viewbox(),
            );

            let segment2 = CachedSegment::new(
                "test2".to_string(),
                (1, 2),
                &PathElement::Line { x1: 0.0, y1: 0.0, x2: 10.0, y2: 0.0 },
                EdgeType::South,
                &create_test_viewbox(),
            );

            assert!(check_segment_alignment(&segment1, &segment2, Some("North")));
        }
    }
}