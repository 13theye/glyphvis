
// src/services/svg/parser.rs
use std::str::FromStr;
use crate::models::PathElement;

pub struct SVGElement {
    pub id: String,
    pub path: PathElement,
}

pub fn parse_svg(svg_content: &str) -> Vec<SVGElement> {
    svg_content
        .lines()
        .filter(|line| line.contains("<path") || line.contains("<circle"))
        .filter_map(|line| {
            if let Some(id) = parse_id(line) {
                if let Some(path) = parse_element(line) {
                    return Some(SVGElement { id, path });
                }
            }
            None
        })
        .collect()
}

fn parse_id(element: &str) -> Option<String> {
    if let Some(id_start) = element.find("id=\"") {
        if let Some(id_end) = element[id_start + 4..].find('\"') {
            return Some(element[id_start + 4..id_start + 4 + id_end].to_string());
        }
    }
    None
}


// supported SVG elements: path & circle
fn parse_element(element: &str) -> Option<PathElement> {
    println!("Parsing SVG element: '{}'", element);
    if element.contains("<circle") {
        return parse_circle(element);
    }

    if let Some((_, second_part)) = element.split_once("id=") {
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
    } else {
        None
    }
}

// Move the existing parsing functions from path_service.rs
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_line() {
        let svg_data = r#"<path id="line1" d="M 0,0 L 100,100"/>"#;
        let elements = parse_svg(svg_data);
        
        assert_eq!(elements.len(), 1);
        assert_eq!(elements[0].id, "line1");
        
        match &elements[0].path {
            PathElement::Line { x1, y1, x2, y2 } => {
                assert_eq!(*x1, 0.0);
                assert_eq!(*y1, 0.0);
                assert_eq!(*x2, 100.0);
                assert_eq!(*y2, 100.0);
            },
            _ => panic!("Expected Line"),
        }
    }

    #[test]
    fn test_parse_circle() {
        let svg_data = r#"<circle id="circle1" cx="50" cy="50" r="25"/>"#;
        let elements = parse_svg(svg_data);
        
        assert_eq!(elements.len(), 1);
        assert_eq!(elements[0].id, "circle1");
        
        match &elements[0].path {
            PathElement::Circle { cx, cy, r } => {
                assert_eq!(*cx, 50.0);
                assert_eq!(*cy, 50.0);
                assert_eq!(*r, 25.0);
            },
            _ => panic!("Expected Circle"),
        }
    }

    #[test]
    fn test_parse_arc() {
        let svg_data = r#"<path id="arc1" d="M50,0A50,50 0 0,0 100,50"/>"#;
        let elements = parse_svg(svg_data);
        
        assert_eq!(elements.len(), 1);
        assert_eq!(elements[0].id, "arc1");
        
        match &elements[0].path {
            PathElement::Arc { 
                start_x, start_y, rx, ry, 
                x_axis_rotation, large_arc, sweep,
                end_x, end_y 
            } => {
                assert_eq!(*start_x, 50.0);
                assert_eq!(*start_y, 0.0);
                assert_eq!(*rx, 50.0);
                assert_eq!(*ry, 50.0);
                assert_eq!(*x_axis_rotation, 0.0);
                assert!(!*large_arc);
                assert!(!*sweep);
                assert_eq!(*end_x, 100.0);
                assert_eq!(*end_y, 50.0);
            },
            _ => panic!("Expected Arc"),
        }
    }

    #[test]
    fn test_parse_multiple_elements() {
        let svg_data = r#"
            <path id="line1" d="M 0,0 L 100,100"/>
            <circle id="circle1" cx="50" cy="50" r="25"/>
        "#;
        let elements = parse_svg(svg_data);
        assert_eq!(elements.len(), 2);
    }

    #[test]
    fn test_parse_invalid_element() {
        let svg_data = r#"<path id="invalid" d="not a path"/>"#;
        let elements = parse_svg(svg_data);
        assert_eq!(elements.len(), 0);
    }
}