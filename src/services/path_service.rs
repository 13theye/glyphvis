// src/services/path_service.rs
// parses SVG raw data into PathElements
// performs edge detection and neighbor hinding calculations

use std::str::FromStr;
use std::collections::HashMap;
use crate::models::grid_model::ViewBox;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum EdgeType {
    North,
    South,
    East,
    West,
    Northwest,
    Northeast,
    Southwest,
    Southeast,
    None
}

#[derive(Debug, Clone)]
pub enum PathElement {
    Line {
        x1: f32,
        y1: f32,
        x2: f32,
        y2: f32,
    },
    Arc {
        start_x: f32,
        start_y: f32,
        rx: f32,
        ry: f32,
        x_axis_rotation: f32,
        large_arc: bool,
        sweep: bool,
        end_x: f32,
        end_y: f32,
    },
    Circle {
        cx: f32,
        cy: f32,
        r: f32,
    }
}

#[derive(Debug, Clone)]
pub struct GridElement {
    pub id: String,
    pub position: (u32, u32),
    pub path: PathElement,
    pub edge_type: EdgeType,
}

// supported SVG elements: path & circle
pub fn parse_svg_element(element: &str) -> Option<PathElement> {
    println!("Parsing SVG element: '{}'", element);
    if element.contains("<circle") {
        return parse_circle(element);
    }

    if let Some((_, second_part)) = element.split_once("id=") {
        // Look for d="..." pattern
        let d = second_part
            .split("d=\"")
            .nth(1)?
            .split('"')
            .next()?
            .trim();

        if d.contains('A') {
            parse_arc(d)
        } else {
            parse_line(d)
        }
    } else {None}
}

fn parse_line(d: &str) -> Option<PathElement> {
    let re = regex::Regex::new(r"M\s*([\d.-]+)[\s,]+([\d.-]+)\s*L\s*([\d.-]+)[\s,]+([\d.-]+)").ok()?;
    let caps = re.captures(d)?;
    
    Some(PathElement::Line {
        x1: f32::from_str(&caps[1]).ok()?,
        y1: f32::from_str(&caps[2]).ok()?,
        x2: f32::from_str(&caps[3]).ok()?,
        y2: f32::from_str(&caps[4]).ok()?,
    })
}

fn parse_arc(d: &str) -> Option<PathElement> {
    //println!("Trying to parse arc: '{}'", d);
    let re = regex::Regex::new(
        r"^M\s*([\d.-]+),([\d.-]+)\s*A\s*([\d.-]+),([\d.-]+)\s*([\d.-]+)\s+(0|1),(0|1)\s*([\d.-]+),([\d.-]+)$"
    ).ok()?;
    
    let caps = re.captures(d)?;
    
    Some(PathElement::Arc {
        start_x: f32::from_str(&caps[1]).ok()?,
        start_y: f32::from_str(&caps[2]).ok()?,
        rx: f32::from_str(&caps[3]).ok()?,
        ry: f32::from_str(&caps[4]).ok()?,
        x_axis_rotation: f32::from_str(&caps[5]).ok()?,
        large_arc: &caps[6] == "1",
        sweep: &caps[7] == "1",
        end_x: f32::from_str(&caps[8]).ok()?,
        end_y: f32::from_str(&caps[9]).ok()?,
    })
}

fn parse_circle(element: &str) -> Option<PathElement> {
    println!("Trying to parse circle: '{}'", element);
    let re = regex::Regex::new(r#"cx="([\d.-]+)".*cy="([\d.-]+)".*r="([\d.-]+)""#).ok()?;
    let caps = re.captures(element)?;
    
    Some(PathElement::Circle {
        cx: f32::from_str(&caps[1]).ok()?,
        cy: f32::from_str(&caps[2]).ok()?,
        r: f32::from_str(&caps[3]).ok()?,
    })
}

pub fn detect_edge_type(element: &PathElement, viewbox: &ViewBox) -> EdgeType {
    match element {
        PathElement::Line { x1, y1, x2, y2, .. } => {
            // Check for lines that run along edges
            if y1 == y2 {
                if *y1 == viewbox.min_y { return EdgeType::North; }
                if *y1 == viewbox.max_y() { return EdgeType::South; }
            }
            if x1 == x2 {
                if *x1 == viewbox.min_x { return EdgeType::West; }
                if *x1 == viewbox.max_x() { return EdgeType::East; }
            }
            EdgeType::None
        },
        PathElement::Circle { cx, cy, .. } => {
            // Check corners first
            if *cy == viewbox.min_y && *cx == viewbox.min_x { return EdgeType::Northwest; }
            if *cy == viewbox.min_y && *cx == viewbox.max_x() { return EdgeType::Northeast; }
            if *cy == viewbox.max_y() && *cx == viewbox.min_x { return EdgeType::Southwest; }
            if *cy == viewbox.max_y() && *cx == viewbox.max_x() { return EdgeType::Southeast; }
            
            // Then edges
            if *cy == viewbox.min_y { return EdgeType::North; }
            if *cy == viewbox.max_y() { return EdgeType::South; }
            if *cx == viewbox.min_x { return EdgeType::West; }
            if *cx == viewbox.max_x() { return EdgeType::East; }
            
            EdgeType::None
        },
        PathElement::Arc { .. } => {
            // Arcs themselves can't be on edges. { .. } means ignore the rest of the fields.
            EdgeType::None
        }
    }
}

/// Gets the relative direction of a neighbor based on grid coordinates
pub fn get_neighbor_direction(x: u32, y: u32, neighbor_x: u32, neighbor_y: u32) -> Option<&'static str> {
    let dx = neighbor_x as i32 - x as i32;
    let dy = neighbor_y as i32 - y as i32;
    
    match (dx, dy) {
        (0, -1) => Some("North"),
        (1, -1) => Some("Northeast"),
        (1, 0) => Some("East"),
        (1, 1) => Some("Southeast"),
        (0, 1) => Some("South"),
        (-1, 1) => Some("Southwest"),
        (-1, 0) => Some("West"),
        (-1, -1) => Some("Northwest"),
        _ => None
    }
}

/// Gets coordinates of priority neighbor for a given grid position and edge type
pub fn get_neighbor_coords(col: u32, row: u32, edge_type: EdgeType, grid_width: u32, grid_height: u32) -> Option<(u32, u32)> {
    match edge_type {
        EdgeType::North => {
            if row > 1 { Some((col, row - 1)) } else { None }
        },
        EdgeType::South => {
            if row < grid_height { Some((col, row + 1)) } else { None }
        },
        EdgeType::East => {
            if col < grid_width { Some((col + 1, row)) } else { None }
        },
        EdgeType::West => {
            if col > 1 { Some((col - 1, row)) } else { None }
        },
        EdgeType::Northwest => {
            if row > 1 && col > 1 { Some((col - 1, row - 1)) } else { None }
        },
        EdgeType::Northeast => {
            if row > 1 && col < grid_width { Some((col + 1, row - 1)) } else { None }
        },
        EdgeType::Southwest => {
            if row < grid_height && col > 0 { Some((col - 1, row + 1)) } else { None }
        },
        EdgeType::Southeast => {
            if row < grid_height && col < grid_width { Some((col + 1, row + 1)) } else { None }
        },
        EdgeType::None => None
    }
}

/// Checks if two paths align based on their edge types and coordinates
pub fn check_path_alignment(
    path1: &PathElement,
    edge_type1: EdgeType,
    path2: &PathElement,
    edge_type2: EdgeType,
    direction: Option<&str>
) -> bool {
    // First check edge type compatibility
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
    }

    // Then check coordinate alignment
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
        _ => false  // Arcs never overlap
    }
}

// this was ported from Glyphmaker is not currently used
pub fn adjust_viewbox_for_edges(viewbox: &mut ViewBox, edge_stroke_width: f32) {
    let half_stroke = edge_stroke_width / 2.0;
    viewbox.min_x -= half_stroke;
    viewbox.min_y -= half_stroke;
    viewbox.width += edge_stroke_width;
    viewbox.height += edge_stroke_width;
}

pub fn get_elements_at(elements: &HashMap<String, GridElement>, x: u32, y: u32) -> Vec<&GridElement> {
    elements
        .iter()
        .filter(|(_, element)| element.position == (x, y))
        .map(|(_, element)| element)
        .collect()
}

// Unlike Glyphmaker, where we draw all elements and then handle selection logic, 
// in Glyphvis we decide on whether to draw an element at the beginning.
pub fn should_draw_element(
    element: &GridElement,
    grid_width: u32,
    grid_height: u32,
    elements: &HashMap<String, GridElement>
) -> bool {
    //println!("\nEvaluating element {} at position {:?} with edge type {:?}", 
    //         element.id, element.position, element.edge_type);

    // If it's not an edge, always draw it
    if element.edge_type == EdgeType::None {
        //println!("Not an edge element, should draw");
        return true;
    }

    let (x, y) = element.position;

    // Get potential neighbor position
    if let Some((neighbor_x, neighbor_y)) = get_neighbor_coords(x, y, element.edge_type, grid_width, grid_height) {
        //println!("Found potential neighbor position: ({}, {})", neighbor_x, neighbor_y);
        
        // Check if neighbor has priority (lower grid position)
        let neighbor_has_priority = neighbor_x < x || (neighbor_x == x && neighbor_y < y);
        //println!("Neighbor has priority: {}", neighbor_has_priority);
        
        // If neighbor has priority and we find a matching path, don't draw this one
        if neighbor_has_priority {
            let neighbor_elements = get_elements_at(elements, neighbor_x, neighbor_y);
            //println!("Found {} neighbor elements", neighbor_elements.len());
            
            for neighbor in neighbor_elements {
                //println!("Checking neighbor {} with edge type {:?}", neighbor.id, neighbor.edge_type);
                let direction = get_neighbor_direction(x, y, neighbor_x, neighbor_y);
                if check_path_alignment(
                    &element.path,
                    element.edge_type,
                    &neighbor.path,
                    neighbor.edge_type,
                    direction
                ) {
                    //println!("Found matching neighbor path, should not draw");
                    return false;
                }
            }
        }
    }
    
    //println!("No matching neighbor found or neighbor doesn't have priority, should draw");
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    // Keep existing test helper
    fn create_viewbox() -> ViewBox {
        ViewBox {
            min_x: 0.0,
            min_y: 0.0,
            width: 100.0,
            height: 100.0,
        }
    }

    // Add new helper to create test elements
    fn create_test_elements() -> HashMap<String, GridElement> {
        let mut elements = HashMap::new();
        
        // Add a few test elements
        elements.insert("1,1 : test1".to_string(), GridElement {
            id: "test1".to_string(),
            position: (1, 1),
            path: PathElement::Line { x1: 0.0, y1: 0.0, x2: 100.0, y2: 0.0 },
            edge_type: EdgeType::North,
        });
        
        elements.insert("1,1 : test2".to_string(), GridElement {
            id: "test2".to_string(),
            position: (1, 1),
            path: PathElement::Circle { cx: 0.0, cy: 0.0, r: 5.0 },
            edge_type: EdgeType::Northwest,
        });
        
        elements.insert("2,1 : test3".to_string(), GridElement {
            id: "test3".to_string(),
            position: (2, 1),
            path: PathElement::Line { x1: 0.0, y1: 0.0, x2: 100.0, y2: 0.0 },
            edge_type: EdgeType::North,
        });

        elements
    }
    
    #[test]
    fn test_get_elements_at() {
        let elements = create_test_elements();
        
        // Test getting elements at 1,1
        let elements_at_origin = get_elements_at(&elements, 1, 1);
        assert_eq!(elements_at_origin.len(), 2);
        
        // Test getting elements at 2,1
        let elements_at_one = get_elements_at(&elements, 2, 1);
        assert_eq!(elements_at_one.len(), 1);
        
        // Test getting elements at empty position
        let elements_at_empty = get_elements_at(&elements, 2, 2);
        assert_eq!(elements_at_empty.len(), 0);
    }

    #[test]
    fn test_get_neighbor_coords() {
        let tests = vec![
            // Format: (x, y, edge_type, grid_width, grid_height, expected)
            (2, 2, EdgeType::North, 4, 4, Some((2, 1))),
            (2, 2, EdgeType::South, 4, 4, Some((2, 3))),
            (2, 2, EdgeType::East, 4, 4, Some((3, 2))),
            (2, 2, EdgeType::West, 4, 4, Some((1, 2))),
            (2, 2, EdgeType::Northwest, 4, 4, Some((1, 1))),
            (2, 2, EdgeType::Northeast, 4, 4, Some((3, 1))),
            (2, 2, EdgeType::Southwest, 4, 4, Some((1, 3))),
            (2, 2, EdgeType::Southeast, 4, 4, Some((3, 3))),
            // Test edge cases
            (1, 1, EdgeType::West, 4, 4, None),
            (1, 1, EdgeType::North, 4, 4, None),
            (1, 4, EdgeType::South, 4, 4, None),
            (4, 1, EdgeType::East, 4, 4, None),
            (1, 1, EdgeType::Northwest, 4, 4, None),
            (4, 4, EdgeType::Southeast, 4, 4, None),
        ];

        for (x, y, edge_type, width, height, expected) in tests {
            let result = get_neighbor_coords(x, y, edge_type, width, height);
            assert_eq!(result, expected, 
                "Failed for x:{}, y:{}, edge_type:{:?}", x, y, edge_type);
        }
    }

    #[test]
    fn test_line_east_west_alignment() {
        // M100,50 L100,0 matching with M0,0 L0,50
        let path1 = PathElement::Line {
            x1: 100.0,
            y1: 50.0,
            x2: 100.0,
            y2: 0.0,
        };
        let path2 = PathElement::Line {
            x1: 0.0,
            y1: 0.0,
            x2: 0.0,
            y2: 50.0,
        };

        let edge_type1 = detect_edge_type(&path1, &create_viewbox());
        let edge_type2 = detect_edge_type(&path2, &create_viewbox());
        
        assert_eq!(edge_type1, EdgeType::East);
        assert_eq!(edge_type2, EdgeType::West);

        let direction = get_neighbor_direction(1, 1, 2, 1);
        assert!(check_path_alignment(&path1, edge_type1, &path2, edge_type2, direction));
    }

    #[test]
    fn test_line_north_south_alignment() {
        // M50,100 L0,100 matching with M0,0 L50,0
        let path1 = PathElement::Line {
            x1: 50.0,
            y1: 100.0,
            x2: 0.0,
            y2: 100.0,
        };
        let path2 = PathElement::Line {
            x1: 0.0,
            y1: 0.0,
            x2: 50.0,
            y2: 0.0,
        };

        let edge_type1 = detect_edge_type(&path1, &create_viewbox());
        let edge_type2 = detect_edge_type(&path2, &create_viewbox());

        let direction = get_neighbor_direction(1, 1, 1, 2);
        assert!(check_path_alignment(&path1, edge_type1, &path2, edge_type2, direction));
    }

    #[test]
    fn test_circle_edge_alignment() {
        // Test circle with cx="50" cy="100" matching circle with cx="50" cy="0"
        let circle1 = PathElement::Circle {
            cx: 50.0,
            cy: 100.0,
            r: 5.0,
        };
        let circle2 = PathElement::Circle {
            cx: 50.0,
            cy: 0.0,
            r: 3.0,  // Different radius should still match
        };

        let edge_type1 = detect_edge_type(&circle1, &create_viewbox());
        let edge_type2 = detect_edge_type(&circle2, &create_viewbox());

        let direction = get_neighbor_direction(1, 1, 1, 2);
        assert!(check_path_alignment(&circle1, edge_type1, &circle2, edge_type2, direction));
    }

    #[test]
    fn test_circle_corner_alignment() {
        // Test circle with cx="100" cy="100" matching circle with cx="0" cy="100"
        let circle1 = PathElement::Circle {
            cx: 100.0,
            cy: 100.0,
            r: 5.0,
        };
        let circle2 = PathElement::Circle {
            cx: 0.0,
            cy: 100.0,
            r: 4.0,  // Different radius should still match
        };

        let edge_type1 = detect_edge_type(&circle1, &create_viewbox());
        let edge_type2 = detect_edge_type(&circle2, &create_viewbox());

        let direction = get_neighbor_direction(1, 1, 2, 1);
        assert!(check_path_alignment(&circle1, edge_type1, &circle2, edge_type2, direction));
    }

    #[test]
    fn test_misaligned_paths() {
        // Test circles with different centers
        let circle1 = PathElement::Circle {
            cx: 50.0,
            cy: 100.0,
            r: 5.0,
        };
        let circle2 = PathElement::Circle {
            cx: 40.0,  // Different x-coordinate
            cy: 0.0,
            r: 5.0,
        };

        let edge_type1 = detect_edge_type(&circle1, &create_viewbox());
        let edge_type2 = detect_edge_type(&circle2, &create_viewbox());
        let direction = get_neighbor_direction(1, 1, 1, 2);

        assert!(!check_path_alignment(&circle1, edge_type1, &circle2, edge_type2, direction));

        // Test misaligned lines
        let line1 = PathElement::Line {
            x1: 100.0,
            y1: 50.0,
            x2: 100.0,
            y2: 0.0,
        };
        let line2 = PathElement::Line {
            x1: 0.0,
            y1: 50.0,  
            x2: 0.0,
            y2: 10.0, // different y-coordinate
        };

        let edge_type1 = detect_edge_type(&line1, &create_viewbox());
        let edge_type2 = detect_edge_type(&line2, &create_viewbox());
        let direction = get_neighbor_direction(1, 1, 2, 1);

        assert!(!check_path_alignment(&line1, edge_type1, &line2, edge_type2, direction));
    }

    #[test]
    fn test_parse_line() {
        let d = "M 0,0 L 100,100";
        let element = parse_line(d).unwrap();
        match element {
            PathElement::Line { x1, y1, x2, y2 } => {
                assert_eq!(x1, 0.0);
                assert_eq!(y1, 0.0);
                assert_eq!(x2, 100.0);
                assert_eq!(y2, 100.0);
            },
            _ => panic!("Expected Line"),
        }
    }

    #[test]
    fn test_parse_circle() {
        let element = r#"<circle cx="50" cy="50" r="5" />"#;
        let circle = parse_circle(element).unwrap();
        match circle {
            PathElement::Circle { cx, cy, r } => {
                assert_eq!(cx, 50.0);
                assert_eq!(cy, 50.0);
                assert_eq!(r, 5.0);
            },
            _ => panic!("Expected Circle"),
        }
    }

    #[test]
    fn test_parse_arc() {
        let d = "M50,0A50,50 0 0,0 100,50";
        let element = parse_arc(d).unwrap();
        match element {
            PathElement::Arc { start_x, start_y, rx, ry, .. } => {
                assert_eq!(start_x, 50.0);
                assert_eq!(start_y, 0.0);
                assert_eq!(rx, 50.0);
                assert_eq!(ry, 50.0);
            },
            _ => panic!("Expected Arc"),
        }
    }

    #[test]
    fn test_edge_detection() {
        let viewbox = ViewBox {
            min_x: 0.0,
            min_y: 0.0,
            width: 100.0,
            height: 100.0,
        };

        // Test north edge
        let north_path=
            PathElement::Line { x1: 0.0, y1: 0.0, x2: 50.0, y2: 0.0};
        assert_eq!(detect_edge_type(&north_path, &viewbox), EdgeType::North);

        // Test corner
        let corner_path =
            PathElement::Circle { cx: 0.0, cy: 0.0, r: 5.0 };
        assert_eq!(detect_edge_type(&corner_path, &viewbox), EdgeType::Northwest);

        let center_path =
        PathElement::Circle { cx: 50.0, cy: 50.0, r: 5.0 };
    assert_eq!(detect_edge_type(&center_path, &viewbox), EdgeType::None);
    }
}

#[cfg(test)]
mod drawing_tests {
    use super::*;

    fn create_test_elements() -> HashMap<String, GridElement> {
        let mut elements = HashMap::new();

        // Test elements for a 3x3 grid
        // Create a vertical line on the east edge of (1,1) and west edge of (2,1)
        elements.insert("1,1 : ver-3-1".to_string(), GridElement {
            id: "ver-3-1".to_string(),
            position: (1, 1),
            path: PathElement::Line {
                x1: 100.0,
                y1: 50.0,
                x2: 100.0,
                y2: 0.0,
            },
            edge_type: EdgeType::East,
        });

        elements.insert("2,1 : ver-1-1".to_string(), GridElement {
            id: "ver-1-1".to_string(),
            position: (2, 1),
            path: PathElement::Line {
                x1: 0.0,
                y1: 0.0,
                x2: 0.0,
                y2: 50.0,
            },
            edge_type: EdgeType::West,
        });

        // Create a horizontal line on the south edge of (1,1) and north edge of (1,2)
        elements.insert("1,1 : hor-3-1".to_string(), GridElement {
            id: "hor-3-1".to_string(),
            position: (1, 1),
            path: PathElement::Line {
                x1: 0.0,
                y1: 100.0,
                x2: 50.0,
                y2: 100.0,
            },
            edge_type: EdgeType::South,
        });

        elements.insert("1,2 : hor-1-1".to_string(), GridElement {
            id: "hor-1-1".to_string(),
            position: (1, 2),
            path: PathElement::Line {
                x1: 0.0,
                y1: 0.0,
                x2: 50.0,
                y2: 0.0,
            },
            edge_type: EdgeType::North,
        });

        elements
    }

    #[test]
    fn test_should_draw_horizontal_edge() {
        let elements = create_test_elements();
        let grid_width = 4;
        let grid_height = 4;

        // Get our test elements
        let east_element = elements.get("1,1 : ver-3-1").unwrap();
        let west_element = elements.get("2,1 : ver-1-1").unwrap();

        // The element at (1,1) should take priority over (2,1) for the shared edge
        assert!(!should_draw_element(west_element, grid_width, grid_height, &elements),
                "Should not draw east edge of (2,1) because (1,1) has priority");
        assert!(should_draw_element(east_element, grid_width, grid_height, &elements),
                "Should draw east edge of (1,1) because it has priority");
    }

    #[test]
    fn test_should_draw_vertical_edge() {
        let elements = create_test_elements();
        let grid_width = 4;
        let grid_height = 4;

        // Get our test elements
        let south_element = elements.get("1,1 : hor-3-1").unwrap();
        let north_element = elements.get("1,2 : hor-1-1").unwrap();

        // The element at (1,2) should take priority over (1,1) for the shared edge
        assert!(should_draw_element(south_element, grid_width, grid_height, &elements),
                "Should draw south edge of (1,1) because (1,2) has priority");
        assert!(!should_draw_element(north_element, grid_width, grid_height, &elements),
                "Should not draw north edge of (1,2) because (1,1) has priority");
    }

    #[test]
    fn test_should_draw_non_edge() {
        let mut elements = create_test_elements();
        let grid_width = 4;
        let grid_height = 4;

        // Add a non-edge element
        elements.insert("1,1 : center".to_string(), GridElement {
            id: "center".to_string(),
            position: (1, 1),
            path: PathElement::Line {
                x1: 25.0,
                y1: 25.0,
                x2: 75.0,
                y2: 75.0,
            },
            edge_type: EdgeType::None,
        });

        let center_element = elements.get("1,1 : center").unwrap();
        assert!(should_draw_element(center_element, grid_width, grid_height, &elements),
                "Should always draw non-edge elements");
    }
}
