// src/grid_service.rs

use std::str::FromStr;

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

#[derive(Debug)]
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
        radius_x: f32,
        radius_y: f32,
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

#[derive(Debug)]
pub struct ViewBox {
    pub min_x: f32,
    pub min_y: f32,
    pub width: f32,
    pub height: f32,
}

impl ViewBox {
    pub fn max_x(&self) -> f32 { self.min_x + self.width }
    pub fn max_y(&self) -> f32 { self.min_y + self.height }
}

pub fn parse_svg_element(element: &str) -> Option<PathElement> {
    if element.starts_with("<circle") {
        return parse_circle(element);
    }

    // Look for d="..." pattern
    let d = element
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
        radius_x: f32::from_str(&caps[3]).ok()?,
        radius_y: f32::from_str(&caps[4]).ok()?,
        x_axis_rotation: f32::from_str(&caps[5]).ok()?,
        large_arc: &caps[6] == "1",
        sweep: &caps[7] == "1",
        end_x: f32::from_str(&caps[8]).ok()?,
        end_y: f32::from_str(&caps[9]).ok()?,
    })
}

fn parse_circle(element: &str) -> Option<PathElement> {
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
            // Arcs themselves aren't typically on edges in
            EdgeType::None
        }
    }
}

pub fn adjust_viewbox_for_edges(viewbox: &mut ViewBox, edge_stroke_width: f32) {
    let half_stroke = edge_stroke_width / 2.0;
    viewbox.min_x -= half_stroke;
    viewbox.min_y -= half_stroke;
    viewbox.width += edge_stroke_width;
    viewbox.height += edge_stroke_width;
}


#[cfg(test)]
mod tests {
use super::*;

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
            PathElement::Arc { start_x, start_y, radius_x, radius_y, .. } => {
                assert_eq!(start_x, 50.0);
                assert_eq!(start_y, 0.0);
                assert_eq!(radius_x, 50.0);
                assert_eq!(radius_y, 50.0);
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
    }
}
