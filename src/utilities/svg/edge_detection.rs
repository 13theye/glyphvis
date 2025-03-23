// /src/utilities/svg/edge_detection.rs
// A collection of functions for finding the edges of a SVG

use crate::models::{EdgeType, PathElement, ViewBox};

pub fn detect_edge_type(element: &PathElement, viewbox: &ViewBox) -> EdgeType {
    match element {
        PathElement::Line { x1, y1, x2, y2, .. } => {
            // Check for lines that run along edges
            if y1 == y2 {
                if *y1 == viewbox.min_y {
                    return EdgeType::North;
                }
                if *y1 == viewbox.max_y() {
                    return EdgeType::South;
                }
            }
            if x1 == x2 {
                if *x1 == viewbox.min_x {
                    return EdgeType::West;
                }
                if *x1 == viewbox.max_x() {
                    return EdgeType::East;
                }
            }
            EdgeType::None
        }
        PathElement::Circle { cx, cy, .. } => {
            // Check corners first
            if *cy == viewbox.min_y && *cx == viewbox.min_x {
                return EdgeType::Northwest;
            }
            if *cy == viewbox.min_y && *cx == viewbox.max_x() {
                return EdgeType::Northeast;
            }
            if *cy == viewbox.max_y() && *cx == viewbox.min_x {
                return EdgeType::Southwest;
            }
            if *cy == viewbox.max_y() && *cx == viewbox.max_x() {
                return EdgeType::Southeast;
            }

            // Then edges
            if *cy == viewbox.min_y {
                return EdgeType::North;
            }
            if *cy == viewbox.max_y() {
                return EdgeType::South;
            }
            if *cx == viewbox.min_x {
                return EdgeType::West;
            }
            if *cx == viewbox.max_x() {
                return EdgeType::East;
            }

            EdgeType::None
        }
        PathElement::Arc { .. } => {
            // Arcs themselves can't be on edges. { .. } means ignore the rest of the fields.
            EdgeType::None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_viewbox() -> ViewBox {
        ViewBox {
            min_x: 0.0,
            min_y: 0.0,
            width: 100.0,
            height: 100.0,
        }
    }

    #[test]
    fn test_line_edge_detection() {
        let viewbox = create_test_viewbox();

        // Test horizontal lines
        let north_line = PathElement::Line {
            x1: 0.0,
            y1: 0.0,
            x2: 50.0,
            y2: 0.0,
        };
        assert_eq!(detect_edge_type(&north_line, &viewbox), EdgeType::North);

        let south_line = PathElement::Line {
            x1: 0.0,
            y1: 100.0,
            x2: 50.0,
            y2: 100.0,
        };
        assert_eq!(detect_edge_type(&south_line, &viewbox), EdgeType::South);

        // Test vertical lines
        let west_line = PathElement::Line {
            x1: 0.0,
            y1: 0.0,
            x2: 0.0,
            y2: 50.0,
        };
        assert_eq!(detect_edge_type(&west_line, &viewbox), EdgeType::West);

        let east_line = PathElement::Line {
            x1: 100.0,
            y1: 0.0,
            x2: 100.0,
            y2: 50.0,
        };
        assert_eq!(detect_edge_type(&east_line, &viewbox), EdgeType::East);

        // Test non-edge line
        let center_line = PathElement::Line {
            x1: 25.0,
            y1: 25.0,
            x2: 75.0,
            y2: 75.0,
        };
        assert_eq!(detect_edge_type(&center_line, &viewbox), EdgeType::None);
    }

    #[test]
    fn test_circle_edge_detection() {
        let viewbox = create_test_viewbox();

        // Test corners
        let nw_circle = PathElement::Circle {
            cx: 0.0,
            cy: 0.0,
            r: 5.0,
        };
        assert_eq!(detect_edge_type(&nw_circle, &viewbox), EdgeType::Northwest);

        let ne_circle = PathElement::Circle {
            cx: 100.0,
            cy: 0.0,
            r: 5.0,
        };
        assert_eq!(detect_edge_type(&ne_circle, &viewbox), EdgeType::Northeast);

        // Test edges
        let north_circle = PathElement::Circle {
            cx: 50.0,
            cy: 0.0,
            r: 5.0,
        };
        assert_eq!(detect_edge_type(&north_circle, &viewbox), EdgeType::North);

        // Test non-edge circle
        let center_circle = PathElement::Circle {
            cx: 50.0,
            cy: 50.0,
            r: 5.0,
        };
        assert_eq!(detect_edge_type(&center_circle, &viewbox), EdgeType::None);
    }

    #[test]
    fn test_arc_edge_detection() {
        let viewbox = create_test_viewbox();
        let arc = PathElement::Arc {
            start_x: 0.0,
            start_y: 0.0,
            rx: 50.0,
            ry: 50.0,
            x_axis_rotation: 0.0,
            large_arc: false,
            sweep: false,
            end_x: 100.0,
            end_y: 100.0,
        };
        assert_eq!(detect_edge_type(&arc, &viewbox), EdgeType::None);
    }
}
