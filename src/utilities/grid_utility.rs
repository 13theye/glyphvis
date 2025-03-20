// src/utilities/grid_utility.rs
// a collection of utility functions for building the grid
// and calculating the positions of SVG elements
//
// Three categoies of functions:
// 1. Parsing
// 2. Arc calculations
// 3. Neighbor checking

use crate::{
    models::{EdgeType, PathElement, ViewBox},
    views::{CachedSegment, SegmentType},
};

use nannou::prelude::*;
use std::f32::consts::PI;

// 1. Parsing
//
//

// Parse viewbox helper function (moved from grid_model.rs)
pub fn parse_viewbox(svg_content: &str) -> Option<ViewBox> {
    let viewbox_data: Vec<String> = svg_content
        .lines()
        .filter(|line| line.contains("<svg id"))
        .filter_map(|line| {
            if let Some(viewbox_start) = line.find("viewBox=\"") {
                if let Some(viewbox_end) = line[viewbox_start + 9..].find('\"') {
                    return Some(
                        line[viewbox_start + 9..viewbox_start + 9 + viewbox_end].to_string(),
                    );
                }
            }
            None
        })
        .collect();

    if viewbox_data.is_empty() {
        eprintln!("Error: No SVG element with viewBox attribute found");
        eprintln!("SVG content:\n{}", svg_content);
        std::process::exit(1);
    }

    viewbox_data.first().map(|data| {
        let viewbox_values: Vec<f32> = data
            .split_whitespace()
            .filter_map(|value| value.parse::<f32>().ok())
            .collect();

        if viewbox_values.len() != 4 {
            eprintln!("Error: ViewBox must contain exactly 4 values");
            eprintln!(
                "Found viewBox=\"{}\" with {} values",
                data,
                viewbox_values.len()
            );
            eprintln!("Values parsed: {:?}", viewbox_values);
            std::process::exit(1);
        }

        ViewBox {
            min_x: viewbox_values[0],
            min_y: viewbox_values[1],
            width: viewbox_values[2],
            height: viewbox_values[3],
        }
    })
}

// 2. Arc Calculations
//
//

// Calculate the center, start angle, and sweep angle for an SVG arc
pub fn generate_arc_points(
    center: Point2,
    rx: f32,
    ry: f32,
    start_angle: f32,
    sweep_angle: f32,
    x_axis_rotation: f32,
    resolution: usize,
) -> Vec<Point2> {
    let mut points = Vec::with_capacity(resolution + 1);

    for i in 0..=resolution {
        let t = i as f32 / resolution as f32;
        let angle = start_angle + t * sweep_angle;

        // Calculate point with proper radii and rotation
        let x = center.x
            + rx * (angle.cos() * x_axis_rotation.to_radians().cos()
                - angle.sin() * x_axis_rotation.to_radians().sin());
        let y = center.y
            + ry * (angle.cos() * x_axis_rotation.to_radians().sin()
                + angle.sin() * x_axis_rotation.to_radians().cos());

        points.push(pt2(x, y));
    }
    //println!("points: {:?}", points);
    //return:
    points
}

pub fn calculate_arc_center(
    start: Point2,
    end: Point2,
    rx: f32,
    ry: f32,
    x_axis_rotation: f32,
    large_arc: bool,
    sweep: bool,
) -> (Point2, f32, f32) {
    // Returns (center, start_angle, sweep_angle)
    let debug_flag = false;
    if debug_flag {
        println!("\nCenter calculation:");
    }

    // Step 1: Transform to origin and unrotated coordinates
    let dx = (start.x - end.x) / 2.0;
    let dy = (start.y - end.y) / 2.0;

    if debug_flag {
        println!("  dx, dy: {:.2}, {:.2}", dx, dy);
    }

    let angle_rad = x_axis_rotation.to_radians();
    let cos_phi = angle_rad.cos();
    let sin_phi = angle_rad.sin();

    // Rotate to align with axes
    let x1p = cos_phi * dx + sin_phi * dy;
    let y1p = -sin_phi * dx + cos_phi * dy;

    if debug_flag {
        println!("  x1p, y1p: {:.2}, {:.2}", x1p, y1p);
    }

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

    let term =
        (rx_sq * ry_sq - rx_sq * y1p_sq - ry_sq * x1p_sq) / (rx_sq * y1p_sq + ry_sq * x1p_sq);

    let s = if term <= 0.0 { 0.0 } else { term.sqrt() };

    if debug_flag {
        println!("  term: {:.2}", term);
        println!("  s: {:.2}", s);
    }

    // Choose center based on sweep and large-arc flags
    let cxp = s * rx_final * y1p / ry_final;
    let cyp = -s * ry_final * x1p / rx_final;

    if debug_flag {
        println!("  cxp, cyp before flip: {:.2}, {:.2}", cxp, cyp);
    }

    // Handle sweep flag to make it clockwise by flipping the center.
    let (cxp, cyp) = if sweep { (-cxp, -cyp) } else { (cxp, cyp) };

    if debug_flag {
        println!("  cxp, cyp after sweep: {:.2}, {:.2}", cxp, cyp);
    }

    // Step 4: Transform center back to original coordinate space
    let cx = cos_phi * cxp - sin_phi * cyp + (start.x + end.x) / 2.0;
    let cy = sin_phi * cxp + cos_phi * cyp + (start.y + end.y) / 2.0;

    if debug_flag {
        println!("  final center: ({:.2}, {:.2})", cx, cy);
    }

    // Step 5: Calculate angles
    let start_vec_x = (x1p - cxp) / rx_final;
    let start_vec_y = (y1p - cyp) / ry_final;
    let end_vec_x = (-x1p - cxp) / rx_final;
    let end_vec_y = (-y1p - cyp) / ry_final;

    let start_angle = (start_vec_y).atan2(start_vec_x);
    let mut sweep_angle = (end_vec_y).atan2(end_vec_x) - start_angle;

    if debug_flag {
        println!(
            "  start_angle: {:.2}° ({:.2} rad)",
            start_angle.to_degrees(),
            start_angle
        );
        println!(
            "  sweep_angle: {:.2}° ({:.2} rad)",
            sweep_angle.to_degrees(),
            sweep_angle
        );
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

// 3. Neighbor Checking
//
//

// Helper functions for overlap checking
/// Checks if two paths overlap based on their edge types and positions
pub fn check_segment_alignment(
    segment1: &CachedSegment,
    segment2: &CachedSegment,
    direction: Option<&str>,
) -> bool {
    let edge_type1 = &segment1.edge_type;
    let edge_type2 = &segment2.edge_type;

    // Check if segments align based on their edge types and positions
    let types_match = match edge_type1 {
        EdgeType::North => *edge_type2 == EdgeType::South,
        EdgeType::South => *edge_type2 == EdgeType::North,
        EdgeType::East => *edge_type2 == EdgeType::West,
        EdgeType::West => *edge_type2 == EdgeType::East,
        EdgeType::Northwest => matches!(
            (direction, edge_type2),
            (Some("Northwest"), EdgeType::Southeast)
                | (Some("West"), EdgeType::Northeast)
                | (Some("North"), EdgeType::Southwest)
        ),
        EdgeType::Northeast => matches!(
            (direction, edge_type2),
            (Some("North"), EdgeType::Southeast)
                | (Some("East"), EdgeType::Northwest)
                | (Some("Northeast"), EdgeType::Southwest)
        ),
        EdgeType::Southwest => matches!(
            (direction, edge_type2),
            (Some("West"), EdgeType::Southeast)
                | (Some("Southwest"), EdgeType::Northeast)
                | (Some("South"), EdgeType::Northwest)
        ),
        EdgeType::Southeast => matches!(
            (direction, edge_type2),
            (Some("East"), EdgeType::Southwest)
                | (Some("South"), EdgeType::Northeast)
                | (Some("Southeast"), EdgeType::Northwest)
        ),
        EdgeType::None => false,
    };

    if !types_match {
        false
    } else {
        let path1 = &segment1.original_path;
        let path2 = &segment2.original_path;
        // then check coordinate alignment
        match (path1, path2) {
            (
                PathElement::Line {
                    x1: x1a,
                    y1: y1a,
                    x2: x2a,
                    y2: y2a,
                },
                PathElement::Line {
                    x1: x1b,
                    y1: y1b,
                    x2: x2b,
                    y2: y2b,
                },
            ) => match edge_type1 {
                EdgeType::North | EdgeType::South => {
                    let matches_forward = x1a == x1b && x2a == x2b;
                    let matches_reversed = x1a == x2b && x2a == x1b;
                    matches_forward || matches_reversed
                }
                EdgeType::East | EdgeType::West => {
                    let matches_forward = y1a == y1b && y2a == y2b;
                    let matches_reversed = y1a == y2b && y2a == y1b;
                    matches_forward || matches_reversed
                }
                _ => false,
            },
            (
                PathElement::Circle {
                    cx: cxa, cy: cya, ..
                },
                PathElement::Circle {
                    cx: cxb, cy: cyb, ..
                },
            ) => {
                match edge_type1 {
                    EdgeType::North | EdgeType::South => cxa == cxb,
                    EdgeType::East | EdgeType::West => cya == cyb,
                    EdgeType::Northwest
                    | EdgeType::Northeast
                    | EdgeType::Southwest
                    | EdgeType::Southeast => {
                        // For corners, check if centers align based on position
                        match direction {
                            Some("East") | Some("West") => cya == cyb,
                            Some("North") | Some("South") => cxa == cxb,
                            _ => true, // For direct diagonal neighbors, already checked edge type
                        }
                    }
                    EdgeType::None => false,
                }
            }
            _ => false, // Arcs never overlap
        }
    }
}

pub fn get_neighbor_coords(
    x: u32,
    y: u32,
    edge_type: EdgeType,
    width: u32,
    height: u32,
) -> Option<(u32, u32)> {
    match edge_type {
        EdgeType::North => {
            if y > 1 {
                Some((x, y - 1))
            } else {
                None
            }
        }
        EdgeType::South => {
            if y < height {
                Some((x, y + 1))
            } else {
                None
            }
        }
        EdgeType::East => {
            if x < width {
                Some((x + 1, y))
            } else {
                None
            }
        }
        EdgeType::West => {
            if x > 1 {
                Some((x - 1, y))
            } else {
                None
            }
        }
        // Add cases for corners
        _ => None,
    }
}

pub fn get_neighbor_direction(
    x: u32,
    y: u32,
    neighbor_x: u32,
    neighbor_y: u32,
) -> Option<&'static str> {
    match (neighbor_x as i32 - x as i32, neighbor_y as i32 - y as i32) {
        (0, -1) => Some("North"),
        (0, 1) => Some("South"),
        (1, 0) => Some("East"),
        (-1, 0) => Some("West"),
        _ => None,
    }
}

// Determine the SegmentType of a given Arc element
pub fn classify_arc(start_x: &f32, start_y: &f32, end_x: &f32, end_y: &f32) -> SegmentType {
    // Top-left arc: starts high, ends left
    if *start_y < *end_y && *end_x < *start_x {
        return SegmentType::ArcTopLeft;
    }
    // Top-right arc: starts high, ends right
    else if *start_y < *end_y && *end_x > *start_x {
        return SegmentType::ArcTopRight;
    }
    // Bottom-left arc: starts low, ends left
    else if *start_y > *end_y && *end_x < *start_x {
        return SegmentType::ArcBottomLeft;
    }
    // Bottom-right arc: starts low, ends right
    else if *start_y > *end_y && *end_x > *start_x {
        return SegmentType::ArcBottomRight;
    }
    SegmentType::Unknown
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

    const TEST_GRID_DIMS: (u32, u32) = (4, 4);

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
            assert_eq!(
                result, expected,
                "Failed for x:{}, y:{}, edge_type:{:?}",
                x, y, edge_type
            );
        }
    }

    #[test]
    fn test_get_neighbor_direction() {
        let tests = vec![
            ((1, 1), (1, 0), Some("North")),
            ((1, 1), (1, 2), Some("South")),
            ((1, 1), (2, 1), Some("East")),
            ((1, 1), (0, 1), Some("West")),
            ((1, 1), (2, 2), None), // Diagonal
        ];

        for ((x, y), (nx, ny), expected) in tests {
            let result = get_neighbor_direction(x, y, nx, ny);
            assert_eq!(
                result, expected,
                "Failed for ({}, {}) -> ({}, {})",
                x, y, nx, ny
            );
        }
    }

    #[test]
    fn test_check_segment_alignment() {
        // Test basic edge alignments
        let segment1 = CachedSegment::new(
            "test1".to_string(),
            (1, 1),
            &PathElement::Line {
                x1: 0.0,
                y1: 0.0,
                x2: 10.0,
                y2: 0.0,
            },
            EdgeType::North,
            &create_test_viewbox(),
            TEST_GRID_DIMS,
        );

        let segment2 = CachedSegment::new(
            "test2".to_string(),
            (1, 2),
            &PathElement::Line {
                x1: 0.0,
                y1: 0.0,
                x2: 10.0,
                y2: 0.0,
            },
            EdgeType::South,
            &create_test_viewbox(),
            TEST_GRID_DIMS,
        );

        assert!(check_segment_alignment(&segment1, &segment2, Some("North")));
    }

    #[test]
    fn test_segment_alignment_mismatches() {
        let viewbox = create_test_viewbox();

        // Test non-matching lines on same edge
        let line1 = CachedSegment::new(
            "line1".to_string(),
            (1, 1),
            &PathElement::Line {
                x1: 0.0,
                y1: 0.0,
                x2: 50.0,
                y2: 0.0,
            },
            EdgeType::North,
            &viewbox,
            TEST_GRID_DIMS,
        );

        let line2 = CachedSegment::new(
            "line2".to_string(),
            (1, 2),
            &PathElement::Line {
                x1: 25.0,
                y1: 0.0,
                x2: 75.0,
                y2: 0.0,
            },
            EdgeType::South,
            &viewbox,
            TEST_GRID_DIMS,
        );

        // These lines share the North/South edge but don't align exactly
        assert!(!check_segment_alignment(&line1, &line2, Some("North")));

        // Test offset circles on same edge
        let circle1 = CachedSegment::new(
            "circle1".to_string(),
            (1, 1),
            &PathElement::Circle {
                cx: 0.0,
                cy: 0.0,
                r: 5.0,
            },
            EdgeType::North,
            &viewbox,
            TEST_GRID_DIMS,
        );

        let circle2 = CachedSegment::new(
            "circle2".to_string(),
            (1, 2),
            &PathElement::Circle {
                cx: 10.0, // Offset by 10 units
                cy: 0.0,
                r: 5.0,
            },
            EdgeType::South,
            &viewbox,
            TEST_GRID_DIMS,
        );

        // These circles are both on the North/South edge but at different x positions
        assert!(!check_segment_alignment(&circle1, &circle2, Some("North")));

        // Test vertical line misalignment
        let vert_line1 = CachedSegment::new(
            "vert1".to_string(),
            (1, 1),
            &PathElement::Line {
                x1: 100.0,
                y1: 0.0,
                x2: 100.0,
                y2: 50.0,
            },
            EdgeType::East,
            &viewbox,
            TEST_GRID_DIMS,
        );

        let vert_line2 = CachedSegment::new(
            "vert2".to_string(),
            (2, 1),
            &PathElement::Line {
                x1: 0.0,
                y1: 25.0, // Different y range
                x2: 0.0,
                y2: 75.0,
            },
            EdgeType::West,
            &viewbox,
            TEST_GRID_DIMS,
        );

        // These lines share East/West edge but don't align vertically
        assert!(!check_segment_alignment(
            &vert_line1,
            &vert_line2,
            Some("East")
        ));
    }
}
