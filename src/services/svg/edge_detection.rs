// /src/services/svg/edge_detection.rs

use crate::models::{EdgeType, PathElement, ViewBox};

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