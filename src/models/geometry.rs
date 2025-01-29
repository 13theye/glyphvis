// src/models/geometry.rs
// Some types for working with segment geometry

#[derive(Debug, Clone)]
pub struct ViewBox {
    pub min_x: f32,
    pub min_y: f32,
    pub width: f32,
    pub height: f32,
}

impl ViewBox {
    pub fn max_x(&self) -> f32 {
        self.min_x + self.width
    }
    pub fn max_y(&self) -> f32 {
        self.min_y + self.height
    }
}

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
    None,
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
    },
}

#[derive(Debug, Clone)]
pub struct GridElement {
    pub id: String,
    pub position: (u32, u32),
    pub path: PathElement,
    pub edge_type: EdgeType,
}

#[cfg(test)]
mod tests {
    use super::*;

    mod viewbox_tests {
        use super::*;

        #[test]
        fn test_viewbox_calculations() {
            let viewbox = ViewBox {
                min_x: 10.0,
                min_y: 20.0,
                width: 100.0,
                height: 200.0,
            };

            assert_eq!(viewbox.max_x(), 110.0);
            assert_eq!(viewbox.max_y(), 220.0);
        }
    }

    mod edge_type_tests {
        use super::*;

        #[test]
        fn test_edge_type_equality() {
            assert_eq!(EdgeType::North, EdgeType::North);
            assert_ne!(EdgeType::North, EdgeType::South);
            assert_eq!(EdgeType::None, EdgeType::None);
        }
    }

    mod path_element_tests {
        use super::*;

        #[test]
        fn test_path_element_creation() {
            let line = PathElement::Line {
                x1: 0.0,
                y1: 0.0,
                x2: 10.0,
                y2: 10.0,
            };

            let circle = PathElement::Circle {
                cx: 5.0,
                cy: 5.0,
                r: 2.0,
            };

            // Test we can create and match on different types
            match line {
                PathElement::Line { x1, y1, x2, y2 } => {
                    assert_eq!(x1, 0.0);
                    assert_eq!(y1, 0.0);
                    assert_eq!(x2, 10.0);
                    assert_eq!(y2, 10.0);
                }
                _ => panic!("Wrong variant"),
            }

            match circle {
                PathElement::Circle { cx, cy, r } => {
                    assert_eq!(cx, 5.0);
                    assert_eq!(cy, 5.0);
                    assert_eq!(r, 2.0);
                }
                _ => panic!("Wrong variant"),
            }
        }
    }
}
