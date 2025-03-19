// src/animation/stroke_order.rs

use crate::{
    animation::transition::SegmentChange,
    services::SegmentGraph,
    views::{CachedGrid, CachedSegment, DrawCommand, GridInstance, SegmentType},
};

use nannou::prelude::*;
use std::collections::{HashMap, HashSet, VecDeque};

struct Stroke {
    segments: Vec<String>,
    start_segment: String,
    primary_type: SegmentType,
    start_position: Point2,
}

pub fn generate_stroke_order_changes(
    grid_instance: &GridInstance,
    target_segments: &HashSet<String>,
) -> Vec<Vec<SegmentChange>> {
    let grid = &grid_instance.grid;
    let graph = &grid_instance.graph;
    let start_segments = &grid_instance.current_active_segments;

    // Find segments that need to be activated or deactivated
    let segments_to_turn_on: HashSet<_> = target_segments
        .difference(start_segments)
        .cloned()
        .collect();
    let segments_to_turn_off: HashSet<_> = start_segments
        .difference(target_segments)
        .cloned()
        .collect();

    // if nothing to turn on, just turn off remaining segments
    if segments_to_turn_on.is_empty() {
        return generate_turn_off_changes(&segments_to_turn_off);
    }

    // Group segments into strokes
    let strokes = group_segments_into_strokes(&segments_to_turn_on, grid, graph);

    // Order strokes according to Hangeul rules
    let ordered_strokes = order_strokes_by_position(strokes);

    // Creates changes for each segment in each stroke, one at a time
    let mut changes = Vec::new();

    // Process each stroke
    for stroke in ordered_strokes {
        let segments_in_order = order_segments_in_stroke(&stroke, grid, graph);

        // Add each segment in the stroke as an individual step
        for segment_id in segments_in_order {
            let change = SegmentChange {
                segment_id,
                turn_on: true,
            };
            changes.push(vec![change]);
        }
    }

    // Add turn-off changes at the end (all at once)
    if !segments_to_turn_off.is_empty() {
        let mut turn_off_changes = Vec::new();
        for segment_id in segments_to_turn_off {
            turn_off_changes.push(SegmentChange {
                segment_id,
                turn_on: false,
            });
        }
        changes.push(turn_off_changes);
    }
    changes
}

fn group_segments_into_strokes(
    segments: &HashSet<String>,
    grid: &CachedGrid,
    graph: &SegmentGraph,
) -> Vec<Stroke> {
    let mut strokes = Vec::new();
    let mut visited: HashSet<String> = HashSet::new();

    for segment_id in segments {
        if visited.contains(segment_id) {
            continue;
        }

        let mut stroke_segments = Vec::new();
        let mut queue = VecDeque::new();
        queue.push_back(segment_id.clone());

        while let Some(current) = queue.pop_front() {
            if visited.contains(&current) {
                continue;
            }
            visited.insert(current.clone());
            stroke_segments.push(current.clone());

            // Get current segment type
            let current_segment = grid.segments.get(&current).unwrap();

            // Explore connected segments
            for neighbor in graph.get_neighbors(&current) {
                if !segments.contains(&neighbor) || visited.contains(&neighbor) {
                    continue;
                }

                // get neighbor type
                let neighbor_segment = grid.segments.get(&neighbor).unwrap();

                // add to stroke if types are compatible
                if are_compatible_segments(current_segment, neighbor_segment) {
                    queue.push_back(neighbor);
                }
            }
        }

        if !stroke_segments.is_empty() {
            // Determine primary type and start position
            let primary_type = get_primary_segment_type(&stroke_segments, grid);
            let start_segment = determine_stroke_start(&stroke_segments, grid, &primary_type);
            let start_position = get_segment_position(&start_segment, grid);

            strokes.push(Stroke {
                segments: stroke_segments,
                start_segment,
                primary_type,
                start_position,
            });
        }
    }
    strokes
}

// Check if two segments should be part of the same stroke
fn are_compatible_segments(seg1: &CachedSegment, seg2: &CachedSegment) -> bool {
    // need to refine rules
    seg1.segment_type == seg2.segment_type
}

// Get the most common segment type in a stroke
fn get_primary_segment_type(segments: &[String], grid: &CachedGrid) -> SegmentType {
    let mut type_counts: HashMap<SegmentType, usize> = HashMap::new();

    for id in segments {
        if let Some(segment) = grid.segments.get(id) {
            *type_counts.entry(segment.segment_type).or_insert(0) += 1;
        }
    }

    type_counts
        .into_iter()
        .max_by_key(|&(_, count)| count)
        .map(|(typ, _)| typ)
        .unwrap_or(SegmentType::Unknown)
}

// Determine the starting segment for a stroke
fn determine_stroke_start(
    segments: &[String],
    grid: &CachedGrid,
    primary_type: &SegmentType,
) -> String {
    match primary_type {
        SegmentType::Horizontal => {
            // For horizontal strokes, start at leftmost
            segments
                .iter()
                .min_by(|a, b| {
                    let pos_a = get_segment_position(a, grid).x;
                    let pos_b = get_segment_position(b, grid).x;
                    pos_a
                        .partial_cmp(&pos_b)
                        .unwrap_or(std::cmp::Ordering::Equal)
                })
                .unwrap()
                .clone()
        }
        SegmentType::Vertical => {
            // For vertical strokes, start at topmost
            segments
                .iter()
                .max_by(|a, b| {
                    let pos_a = get_segment_position(a, grid).y;
                    let pos_b = get_segment_position(b, grid).y;
                    pos_a
                        .partial_cmp(&pos_b)
                        .unwrap_or(std::cmp::Ordering::Equal)
                })
                .unwrap()
                .clone()
        }
        SegmentType::ArcTopLeft
        | SegmentType::ArcTopRight
        | SegmentType::ArcBottomLeft
        | SegmentType::ArcBottomRight => {
            // For arcs, find an appropriate starting point based on type
            determine_arc_start(segments, grid, primary_type)
        }
        SegmentType::Unknown => {
            // Default to topmost, leftmost
            segments
                .iter()
                .max_by(|a, b| {
                    let pos_a = get_segment_position(a, grid);
                    let pos_b = get_segment_position(b, grid);
                    pos_a
                        .y
                        .partial_cmp(&pos_b.y)
                        .unwrap_or(std::cmp::Ordering::Equal)
                        .then(
                            pos_a
                                .x
                                .partial_cmp(&pos_b.x)
                                .unwrap_or(std::cmp::Ordering::Equal),
                        )
                })
                .unwrap()
                .clone()
        }
    }
}

// Get position for a segment (using the starting point)
fn get_segment_position(segment_id: &str, grid: &CachedGrid) -> Point2 {
    if let Some(segment) = grid.segments.get(segment_id) {
        // Use the appropriate point based on segment type
        match segment.segment_type {
            SegmentType::Horizontal => find_leftmost_point(&segment.draw_commands),
            SegmentType::Vertical => find_topmost_point(&segment.draw_commands),
            SegmentType::ArcTopLeft => find_topmost_point(&segment.draw_commands),
            SegmentType::ArcTopRight => find_topmost_point(&segment.draw_commands),
            SegmentType::ArcBottomLeft => find_leftmost_point(&segment.draw_commands),
            SegmentType::ArcBottomRight => find_rightmost_point(&segment.draw_commands),
            SegmentType::Unknown => find_average_point(&segment.draw_commands),
        }
    } else {
        Point2::new(0.0, 0.0)
    }
}

// Helper functions to find specific points in draw commands
fn find_leftmost_point(commands: &[DrawCommand]) -> Point2 {
    let mut leftmost = Point2::new(f32::MAX, 0.0);

    for cmd in commands {
        match cmd {
            DrawCommand::Line { start, end } => {
                if start.x < leftmost.x {
                    leftmost = *start;
                }
                if end.x < leftmost.x {
                    leftmost = *end;
                }
            }
            DrawCommand::Arc { points } => {
                for point in points {
                    if point.x < leftmost.x {
                        leftmost = *point;
                    }
                }
            }
            DrawCommand::Circle { center, .. } => {
                if center.x < leftmost.x {
                    leftmost = *center;
                }
            }
        }
    }

    leftmost
}

// Similarly implement other point-finding functions
fn find_topmost_point(commands: &[DrawCommand]) -> Point2 {
    let mut topmost = Point2::new(0.0, f32::MAX);

    for cmd in commands {
        match cmd {
            DrawCommand::Line { start, end } => {
                // Note: Lower y value is higher in screen coordinates
                if start.y < topmost.y {
                    topmost = *start;
                }
                if end.y < topmost.y {
                    topmost = *end;
                }
            }
            DrawCommand::Arc { points } => {
                for point in points {
                    if point.y < topmost.y {
                        topmost = *point;
                    }
                }
            }
            DrawCommand::Circle { center, .. } => {
                if center.y < topmost.y {
                    topmost = *center;
                }
            }
        }
    }

    topmost
}

fn find_rightmost_point(commands: &[DrawCommand]) -> Point2 {
    let mut rightmost = Point2::new(f32::MIN, 0.0);

    for cmd in commands {
        match cmd {
            DrawCommand::Line { start, end } => {
                if start.x > rightmost.x {
                    rightmost = *start;
                }
                if end.x > rightmost.x {
                    rightmost = *end;
                }
            }
            DrawCommand::Arc { points } => {
                for point in points {
                    if point.x > rightmost.x {
                        rightmost = *point;
                    }
                }
            }
            DrawCommand::Circle { center, .. } => {
                if center.x > rightmost.x {
                    rightmost = *center;
                }
            }
        }
    }

    rightmost
}

fn find_bottommost_point(commands: &[DrawCommand]) -> Point2 {
    let mut bottommost = Point2::new(0.0, f32::MIN);

    for cmd in commands {
        match cmd {
            DrawCommand::Line { start, end } => {
                // Note: Higher y value is lower in screen coordinates
                if start.y > bottommost.y {
                    bottommost = *start;
                }
                if end.y > bottommost.y {
                    bottommost = *end;
                }
            }
            DrawCommand::Arc { points } => {
                for point in points {
                    if point.y > bottommost.y {
                        bottommost = *point;
                    }
                }
            }
            DrawCommand::Circle { center, .. } => {
                if center.y > bottommost.y {
                    bottommost = *center;
                }
            }
        }
    }

    bottommost
}

fn find_average_point(commands: &[DrawCommand]) -> Point2 {
    let mut sum = Point2::new(0.0, 0.0);
    let mut count = 0;

    for cmd in commands {
        match cmd {
            DrawCommand::Line { start, end } => {
                sum += *start;
                sum += *end;
                count += 2;
            }
            DrawCommand::Arc { points } => {
                for point in points {
                    sum += *point;
                    count += 1;
                }
            }
            DrawCommand::Circle { center, .. } => {
                sum += *center;
                count += 1;
            }
        }
    }

    if count > 0 {
        sum / count as f32
    } else {
        Point2::new(0.0, 0.0)
    }
}

// Determine start point for arc segments
fn determine_arc_start(segments: &[String], grid: &CachedGrid, arc_type: &SegmentType) -> String {
    // For different arc types, starting points differ
    match arc_type {
        SegmentType::ArcTopLeft => {
            // Start at top for top-left arc
            segments
                .iter()
                .min_by(|a, b| {
                    let pos_a = get_segment_position(a, grid);
                    let pos_b = get_segment_position(b, grid);
                    pos_a
                        .y
                        .partial_cmp(&pos_b.y)
                        .unwrap_or(std::cmp::Ordering::Equal)
                })
                .unwrap()
                .clone()
        }
        SegmentType::ArcTopRight => {
            // Start at top for top-right arc
            segments
                .iter()
                .min_by(|a, b| {
                    let pos_a = get_segment_position(a, grid);
                    let pos_b = get_segment_position(b, grid);
                    pos_a
                        .y
                        .partial_cmp(&pos_b.y)
                        .unwrap_or(std::cmp::Ordering::Equal)
                })
                .unwrap()
                .clone()
        }
        SegmentType::ArcBottomLeft => {
            // Start at left for bottom-left arc
            segments
                .iter()
                .max_by(|a, b| {
                    let pos_a = get_segment_position(a, grid);
                    let pos_b = get_segment_position(b, grid);
                    pos_a
                        .x
                        .partial_cmp(&pos_b.x)
                        .unwrap_or(std::cmp::Ordering::Equal)
                })
                .unwrap()
                .clone()
        }
        SegmentType::ArcBottomRight => {
            // Start at right for bottom-right arc
            segments
                .iter()
                .min_by(|a, b| {
                    let pos_a = get_segment_position(a, grid);
                    let pos_b = get_segment_position(b, grid);
                    pos_a
                        .x
                        .partial_cmp(&pos_b.x)
                        .unwrap_or(std::cmp::Ordering::Equal)
                })
                .unwrap()
                .clone()
        }
        _ => segments[0].clone(),
    }
}

fn order_strokes_by_position(mut strokes: Vec<Stroke>) -> Vec<Stroke> {
    // First, identify connected horizontal and vertical strokes
    let connected_pairs = identify_horizontal_vertical_connections(&strokes);

    // Sort strokes by position using quadrant-based prioritization
    strokes.sort_by(|a, b| {
        // Check if strokes are connected in a horizontal-to-vertical sequence
        if let Some((horizontal_id, vertical_id)) = connected_pairs.iter().find(|(h, v)| {
            (*h == a.start_segment && *v == b.start_segment)
                || (*h == b.start_segment && *v == a.start_segment)
        }) {
            // If a is the horizontal and b is the vertical, a comes first
            if &a.start_segment == horizontal_id && &b.start_segment == vertical_id {
                return std::cmp::Ordering::Less;
            }
            // If b is the horizontal and a is the vertical, b comes first
            if &b.start_segment == horizontal_id && &a.start_segment == vertical_id {
                return std::cmp::Ordering::Greater;
            }
        }

        // Define quadrant boundaries
        let mid_x = 2.5; // Horizontal middle of the grid
        let mid_y = 2.0; // Vertical middle of the grid

        // Determine which quadrant each stroke starts in
        let a_quadrant = get_quadrant(a.start_position.x, a.start_position.y, mid_x, mid_y);
        let b_quadrant = get_quadrant(b.start_position.x, b.start_position.y, mid_x, mid_y);

        // Rule 1: Quadrant 1 (top-left) before all others
        if a_quadrant == 1 && b_quadrant != 1 {
            return std::cmp::Ordering::Greater;
        }
        if a_quadrant != 1 && b_quadrant == 1 {
            return std::cmp::Ordering::Less;
        }

        // Rule 2: Quadrant 2 (top-right) before bottom half
        if a_quadrant == 2 && (b_quadrant == 3 || b_quadrant == 4) {
            return std::cmp::Ordering::Greater;
        }
        if (a_quadrant == 3 || a_quadrant == 4) && b_quadrant == 2 {
            return std::cmp::Ordering::Less;
        }

        // For all areas, prioritize top to bottom
        if (a.start_position.y - b.start_position.y).abs() > 1.0 {
            return b
                .start_position
                .y
                .partial_cmp(&a.start_position.y)
                .unwrap_or(std::cmp::Ordering::Equal);
        }

        // For all areas, then prioritize left to right
        if (a.start_position.x - b.start_position.x).abs() > 1.0 {
            return a
                .start_position
                .x
                .partial_cmp(&b.start_position.x)
                .unwrap_or(std::cmp::Ordering::Equal);
        }

        // If positions are very close, use segment type priority
        match (&a.primary_type, &b.primary_type) {
            // Horizontal before vertical
            (SegmentType::Horizontal, SegmentType::Vertical) => std::cmp::Ordering::Less,
            (SegmentType::Vertical, SegmentType::Horizontal) => std::cmp::Ordering::Greater,

            // Right-to-left arcs before left-to-right arcs
            (SegmentType::ArcTopLeft, SegmentType::ArcTopRight) => std::cmp::Ordering::Less,
            (SegmentType::ArcTopRight, SegmentType::ArcTopLeft) => std::cmp::Ordering::Greater,
            (SegmentType::ArcBottomLeft, SegmentType::ArcBottomRight) => std::cmp::Ordering::Less,
            (SegmentType::ArcBottomRight, SegmentType::ArcBottomLeft) => {
                std::cmp::Ordering::Greater
            }

            // Horizontal before any arc
            (SegmentType::Horizontal, _) if is_arc_type(&b.primary_type) => {
                std::cmp::Ordering::Less
            }
            (_, SegmentType::Horizontal) if is_arc_type(&a.primary_type) => {
                std::cmp::Ordering::Greater
            }

            // Vertical before any arc
            (SegmentType::Vertical, _) if is_arc_type(&b.primary_type) => std::cmp::Ordering::Less,
            (_, SegmentType::Vertical) if is_arc_type(&a.primary_type) => {
                std::cmp::Ordering::Greater
            }

            // Default equal if no other rule applies
            _ => std::cmp::Ordering::Equal,
        }
    });

    strokes
}

// Function to identify connected horizontal-vertical segments
fn identify_horizontal_vertical_connections(strokes: &[Stroke]) -> Vec<(String, String)> {
    let mut connections = Vec::new();

    for (i, stroke_a) in strokes.iter().enumerate() {
        if stroke_a.primary_type != SegmentType::Horizontal {
            continue;
        }

        // Find the end point of this horizontal stroke
        let end_point = find_rightmost_point_for_stroke(stroke_a);

        // Look for vertical strokes that start at this end point
        for stroke_b in strokes.iter().skip(i + 1) {
            if stroke_b.primary_type != SegmentType::Vertical {
                continue;
            }

            // Find the start point of this vertical stroke
            let start_point = find_topmost_point_for_stroke(stroke_b);

            // Check if they're connected (within a small threshold)
            if (end_point.x - start_point.x).abs() < 1.0
                && (end_point.y - start_point.y).abs() < 1.0
            {
                connections.push((
                    stroke_a.start_segment.clone(),
                    stroke_b.start_segment.clone(),
                ));
            }
        }
    }

    connections
}

// Helper to find the rightmost point for a horizontal stroke
fn find_rightmost_point_for_stroke(stroke: &Stroke) -> Point2 {
    // Implementation would access all segments in the stroke
    // and find the rightmost point across all of them
    // This is a simplified implementation:
    Point2::new(stroke.start_position.x + 10.0, stroke.start_position.y)
}

// Helper to find the topmost point for a vertical stroke
fn find_topmost_point_for_stroke(stroke: &Stroke) -> Point2 {
    // Implementation would access all segments in the stroke
    // and find the topmost point across all of them
    // This is a simplified implementation:
    Point2::new(stroke.start_position.x, stroke.start_position.y)
}

// Helper function to determine quadrant
fn get_quadrant(x: f32, y: f32, mid_x: f32, mid_y: f32) -> u8 {
    if y <= mid_y {
        if x < mid_x {
            1 // Top-left
        } else {
            2 // Top-right
        }
    } else {
        if x < mid_x {
            3 // Bottom-left
        } else {
            4 // Bottom-right
        }
    }
}

// Helper function to check if a segment type is an arc
fn is_arc_type(segment_type: &SegmentType) -> bool {
    matches!(
        segment_type,
        SegmentType::ArcTopLeft
            | SegmentType::ArcTopRight
            | SegmentType::ArcBottomLeft
            | SegmentType::ArcBottomRight
    )
}
// Order strokes based on position for Hangeul writing flow
fn order_strokes_by_position_old(mut strokes: Vec<Stroke>) -> Vec<Stroke> {
    // Sort strokes by position using Hangeul-like principles
    strokes.sort_by(|a, b| {
        // Sort by vertical position first (top to bottom)
        if (a.start_position.y - b.start_position.y).abs() > 1.0 {
            return b
                .start_position
                .y
                .partial_cmp(&a.start_position.y)
                .unwrap_or(std::cmp::Ordering::Equal);
        }
        // Then by horizontal position (left to right)
        if (a.start_position.x - b.start_position.x).abs() > 1.0 {
            return a
                .start_position
                .x
                .partial_cmp(&b.start_position.x)
                .unwrap_or(std::cmp::Ordering::Equal);
        }

        // If positions are very close, use type priority
        match (a.primary_type, b.primary_type) {
            (SegmentType::Horizontal, SegmentType::Vertical) => std::cmp::Ordering::Less,
            (SegmentType::Vertical, SegmentType::Horizontal) => std::cmp::Ordering::Greater,
            _ => std::cmp::Ordering::Equal,
        }
    });

    strokes
}

// Order segments within a stroke in natural writing order
fn order_segments_in_stroke(
    stroke: &Stroke,
    grid: &CachedGrid,
    graph: &SegmentGraph,
) -> Vec<String> {
    let mut ordered = Vec::new();
    let mut visited = HashSet::new();

    // Start with the designated start segment
    let mut current = stroke.start_segment.clone();
    ordered.push(current.clone());
    visited.insert(current.clone());

    // Traverse the stroke using SegmentGraph
    while ordered.len() < stroke.segments.len() {
        let mut best_next = None;
        let mut best_score = f32::MAX;

        // Find unvisited neighbors
        for neighbor in graph.get_neighbors(&current) {
            if stroke.segments.contains(&neighbor) && !visited.contains(&neighbor) {
                // Score based on position relative to current segment's flow
                let score = score_next_segment(&current, &neighbor, grid, &stroke.primary_type);
                if score < best_score {
                    best_score = score;
                    best_next = Some(neighbor.clone());
                }
            }
        }

        if let Some(next) = best_next {
            ordered.push(next.clone());
            visited.insert(next.clone());
            current = next;
        } else {
            // If we can't continue the path, find any unvisited segment
            for segment in &stroke.segments {
                if !visited.contains(segment) {
                    ordered.push(segment.clone());
                    visited.insert(segment.clone());
                    current = segment.clone();
                    break;
                }
            }
        }
    }

    ordered
}

// Score next segment based on natural writing flow
fn score_next_segment(
    current: &str,
    next: &str,
    grid: &CachedGrid,
    primary_type: &SegmentType,
) -> f32 {
    let current_pos = get_segment_position(current, grid);
    let next_pos = get_segment_position(next, grid);

    match primary_type {
        SegmentType::Horizontal => {
            // For horizontal, prefer moving right
            (next_pos.x - current_pos.x).abs() * 10.0
                + if next_pos.x < current_pos.x {
                    1000.0
                } else {
                    0.0
                }
        }
        SegmentType::Vertical => {
            // For vertical, prefer moving down
            (next_pos.y - current_pos.y).abs() * 10.0
                + if next_pos.y > current_pos.y {
                    1000.0
                } else {
                    0.0
                }
        }
        SegmentType::ArcTopLeft => {
            // For top-left arc, prefer counter-clockwise motion
            let dx = next_pos.x - current_pos.x;
            let dy = next_pos.y - current_pos.y;
            if current_pos.y < next_pos.y {
                dx.abs() // Moving down, prefer smaller horizontal change
            } else {
                dy.abs() // Moving horizontally, prefer smaller vertical change
            }
        }
        SegmentType::ArcTopRight => {
            // Similar logic for other arc types
            let dx = next_pos.x - current_pos.x;
            let dy = next_pos.y - current_pos.y;
            if current_pos.y < next_pos.y {
                dx.abs()
            } else {
                dy.abs()
            }
        }
        _ => {
            // Default scoring
            (next_pos - current_pos).length()
        }
    }
}

// Generate changes to turn off segments
fn generate_turn_off_changes(segments: &HashSet<String>) -> Vec<Vec<SegmentChange>> {
    if segments.is_empty() {
        return Vec::new();
    }

    let changes = segments
        .iter()
        .map(|id| SegmentChange {
            segment_id: id.clone(),
            turn_on: false,
        })
        .collect();

    vec![changes]
}
