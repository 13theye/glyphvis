// src/utilities/segment_analysis.rs
//
// a collection of functions to understand segments and their connections.

use crate::{
    services::SegmentGraph,
    views::{CachedGrid, CachedSegment, DrawCommand, SegmentType},
};
use nannou::prelude::*;
use std::collections::{HashMap, HashSet};

// Constants
pub const CONNECTION_THRESHOLD: f32 = 0.1; // Threshold for considering points connected

/// Finds segments that intersect a vertical boundary
pub fn find_x_boundary_intersections(
    grid: &CachedGrid,
    graph: &SegmentGraph,
    boundary_x: u32,
) -> Vec<(Point2, Vec<String>)> {
    let mut intersections = Vec::new();

    // Calculate the x-coordinate of this boundary in viewbox space
    let boundary_x_coord = boundary_x as f32 * (grid.viewbox.width / grid.dimensions.0 as f32);

    // Find segments that cross this boundary
    for (id, segment) in &grid.segments {
        // Skip stretch segments
        if id.starts_with("stretch-") {
            continue;
        }

        // Check segments that might cross the boundary
        if segment.segment_type == SegmentType::Horizontal || is_arc_type(&segment.segment_type) {
            // Find intersection points with this boundary
            for command in &segment.draw_commands {
                if let Some(intersection) = find_command_x_intersection(command, boundary_x_coord) {
                    // Find all segments connected at this intersection
                    let connected_segments =
                        find_connected_segments_at_point(id, intersection, graph, grid);

                    intersections.push((intersection, connected_segments));
                }
            }
        }
    }

    intersections
}

/// Finds segments that intersect a horizontal boundary
pub fn find_y_boundary_intersections(
    grid: &CachedGrid,
    graph: &SegmentGraph,
    boundary_y: u32,
) -> Vec<(Point2, Vec<String>)> {
    let mut intersections = Vec::new();

    // Calculate the y-coordinate of this boundary in viewbox space
    let boundary_y_coord = boundary_y as f32 * (grid.viewbox.height / grid.dimensions.1 as f32);

    // Find segments that cross this boundary
    for (id, segment) in &grid.segments {
        // Skip stretch segments
        if id.starts_with("stretch-") {
            continue;
        }

        // Check segments that might cross the boundary
        if segment.segment_type == SegmentType::Vertical || is_arc_type(&segment.segment_type) {
            // Find intersection points with this boundary
            for command in &segment.draw_commands {
                if let Some(intersection) = find_command_y_intersection(command, boundary_y_coord) {
                    // Find all segments connected at this intersection
                    let connected_segments =
                        find_connected_segments_at_point(id, intersection, graph, grid);

                    intersections.push((intersection, connected_segments));
                }
            }
        }
    }

    intersections
}

/// Finds where a command intersects a vertical line
pub fn find_command_x_intersection(command: &DrawCommand, boundary_x: f32) -> Option<Point2> {
    match command {
        DrawCommand::Line { start, end } => {
            // If line crosses the boundary
            if (start.x <= boundary_x && end.x >= boundary_x)
                || (start.x >= boundary_x && end.x <= boundary_x)
            {
                // Calculate y at intersection
                let t = (boundary_x - start.x) / (end.x - start.x);
                let y = start.y + t * (end.y - start.y);
                Some(pt2(boundary_x, y))
            } else {
                None
            }
        }
        DrawCommand::Arc { points } => {
            // Check each line segment in the arc
            for window in points.windows(2) {
                if let [p1, p2] = window {
                    if (p1.x <= boundary_x && p2.x >= boundary_x)
                        || (p1.x >= boundary_x && p2.x <= boundary_x)
                    {
                        // Calculate y at intersection
                        let t = (boundary_x - p1.x) / (p2.x - p1.x);
                        let y = p1.y + t * (p2.y - p1.y);
                        return Some(pt2(boundary_x, y));
                    }
                }
            }
            None
        }
        DrawCommand::Circle { .. } => None, // Circles don't create boundary crossings
    }
}

/// Finds where a command intersects a horizontal line
pub fn find_command_y_intersection(command: &DrawCommand, boundary_y: f32) -> Option<Point2> {
    match command {
        DrawCommand::Line { start, end } => {
            // If line crosses the boundary
            if (start.y <= boundary_y && end.y >= boundary_y)
                || (start.y >= boundary_y && end.y <= boundary_y)
            {
                // Calculate x at intersection
                let t = (boundary_y - start.y) / (end.y - start.y);
                let x = start.x + t * (end.x - start.x);
                Some(pt2(x, boundary_y))
            } else {
                None
            }
        }
        DrawCommand::Arc { points } => {
            // Check each line segment in the arc
            for window in points.windows(2) {
                if let [p1, p2] = window {
                    if (p1.y <= boundary_y && p2.y >= boundary_y)
                        || (p1.y >= boundary_y && p2.y <= boundary_y)
                    {
                        // Calculate x at intersection
                        let t = (boundary_y - p1.y) / (p2.y - p1.y);
                        let x = p1.x + t * (p2.x - p1.x);
                        return Some(pt2(x, boundary_y));
                    }
                }
            }
            None
        }
        DrawCommand::Circle { .. } => None, // Circles don't create boundary crossings
    }
}

/// Find all segments connected at a point
pub fn find_connected_segments_at_point(
    segment_id: &str,
    point: Point2,
    graph: &SegmentGraph,
    grid: &CachedGrid,
) -> Vec<String> {
    let mut connected = vec![segment_id.to_string()];
    let mut visited = HashSet::new();
    visited.insert(segment_id.to_string());

    // Find direct neighbors in graph
    let mut check_neighbors = vec![segment_id.to_string()];

    while let Some(current_id) = check_neighbors.pop() {
        for neighbor in graph.neighbors_of(&current_id) {
            if visited.contains(&neighbor) {
                continue;
            }

            // Check if this neighbor is also at this intersection point
            if let Some(neighbor_segment) = grid.segment(&neighbor) {
                if segment_endpoints_contain_point(neighbor_segment, point, CONNECTION_THRESHOLD) {
                    connected.push(neighbor.clone());
                    visited.insert(neighbor.clone());
                    check_neighbors.push(neighbor);
                }
            }
        }
    }

    connected
}

/// Check if a segment's endpoints contain a specific point
pub fn segment_endpoints_contain_point(
    segment: &CachedSegment,
    point: Point2,
    threshold: f32,
) -> bool {
    for command in &segment.draw_commands {
        // Check if any endpoint matches our intersection point
        let endpoints = match command {
            DrawCommand::Line { start, end } => vec![*start, *end],
            DrawCommand::Arc { points } => {
                let mut eps = Vec::new();
                if let Some(first) = points.first() {
                    eps.push(*first);
                }
                if let Some(last) = points.last() {
                    eps.push(*last);
                }
                eps
            }
            DrawCommand::Circle { center, .. } => vec![*center],
        };

        for endpoint in endpoints {
            if point.distance(endpoint) < threshold {
                return true;
            }
        }
    }

    false
}

/// Check if a segment type is an arc
pub fn is_arc_type(segment_type: &SegmentType) -> bool {
    matches!(
        segment_type,
        SegmentType::ArcTopLeft
            | SegmentType::ArcTopRight
            | SegmentType::ArcBottomLeft
            | SegmentType::ArcBottomRight
    )
}

/// Find the most compatible connected segment
pub fn find_most_compatible_segment(
    segment_id: &str,
    graph: &SegmentGraph,
    grid: &CachedGrid,
    active_segments: &HashSet<String>,
) -> Option<String> {
    // Get all neighbors from the graph
    let neighbors = graph.neighbors_of(segment_id);

    // If any neighbor is active, prioritize it
    for neighbor in &neighbors {
        if active_segments.contains(neighbor) {
            return Some(neighbor.clone());
        }
    }

    // If no active neighbors, check for compatible segment types
    if let Some(segment) = grid.segment(segment_id) {
        for neighbor in &neighbors {
            if let Some(neighbor_segment) = grid.segment(neighbor) {
                if are_compatible_segments(segment, neighbor_segment) {
                    return Some(neighbor.clone());
                }
            }
        }
    }

    // Return the first neighbor if nothing else matches
    neighbors.first().cloned()
}

/// Check if two segments are compatible for strokes (similar types)
pub fn are_compatible_segments(seg1: &CachedSegment, seg2: &CachedSegment) -> bool {
    if seg1.segment_type == seg2.segment_type {
        return true;
    }

    // Special handling for arcs that form a circle
    if is_arc_type(&seg1.segment_type) && is_arc_type(&seg2.segment_type) {
        // Check if these arcs are adjacent in a circular pattern
        match (&seg1.segment_type, &seg2.segment_type) {
            (SegmentType::ArcTopLeft, SegmentType::ArcTopRight) => return true,
            (SegmentType::ArcTopRight, SegmentType::ArcBottomRight) => return true,
            (SegmentType::ArcBottomRight, SegmentType::ArcBottomLeft) => return true,
            (SegmentType::ArcBottomLeft, SegmentType::ArcTopLeft) => return true,
            // Also allow the reverse connections
            (SegmentType::ArcTopRight, SegmentType::ArcTopLeft) => return true,
            (SegmentType::ArcBottomRight, SegmentType::ArcTopRight) => return true,
            (SegmentType::ArcBottomLeft, SegmentType::ArcBottomRight) => return true,
            (SegmentType::ArcTopLeft, SegmentType::ArcBottomLeft) => return true,
            _ => {}
        }
    }

    // Default
    false
}

/// Get the most common segment type in a set of segments
pub fn get_primary_segment_type(segments: &[String], grid: &CachedGrid) -> SegmentType {
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

/// Get segment position based on segment type
pub fn get_segment_start_point(segment_id: &str, grid: &CachedGrid) -> Point2 {
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

/// Find leftmost point among draw commands
pub fn find_leftmost_point(commands: &[DrawCommand]) -> Point2 {
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

/// Find topmost point among draw commands
pub fn find_topmost_point(commands: &[DrawCommand]) -> Point2 {
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

/// Find rightmost point among draw commands
pub fn find_rightmost_point(commands: &[DrawCommand]) -> Point2 {
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

/// Find bottommost point among draw commands
pub fn find_bottommost_point(commands: &[DrawCommand]) -> Point2 {
    let mut bottommost = Point2::new(0.0, f32::MIN);

    for cmd in commands {
        match cmd {
            DrawCommand::Line { start, end } => {
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

/// Calculate average point of all command endpoints
pub fn find_average_point(commands: &[DrawCommand]) -> Point2 {
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

/// Determine which quadrant a point is in
pub fn get_quadrant(x: f32, y: f32, mid_x: f32, mid_y: f32) -> u8 {
    if y <= mid_y {
        if x < mid_x {
            1 // Top-left
        } else {
            2 // Top-right
        }
    } else if x < mid_x {
        3 // Bottom-left
    } else {
        4 // Bottom-right
    }
}

/// Identify the most suitable starting segment based on segment type
pub fn determine_stroke_start(
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
                    let pos_a = get_segment_start_point(a, grid).x;
                    let pos_b = get_segment_start_point(b, grid).x;
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
                    let pos_a = get_segment_start_point(a, grid).y;
                    let pos_b = get_segment_start_point(b, grid).y;
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
            // For arcs, find appropriate starting point based on type
            determine_arc_start(segments, grid, primary_type)
        }
        SegmentType::Unknown => {
            // Default to topmost, leftmost
            segments
                .iter()
                .max_by(|a, b| {
                    let pos_a = get_segment_start_point(a, grid);
                    let pos_b = get_segment_start_point(b, grid);
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

/// Determine the starting segment for an arc
pub fn determine_arc_start(
    segments: &[String],
    grid: &CachedGrid,
    arc_type: &SegmentType,
) -> String {
    // For different arc types, starting points differ
    match arc_type {
        SegmentType::ArcTopLeft => {
            // Start at top for top-left arc
            segments
                .iter()
                .min_by(|a, b| {
                    let pos_a = get_segment_start_point(a, grid);
                    let pos_b = get_segment_start_point(b, grid);
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
                .max_by(|a, b| {
                    let pos_a = get_segment_start_point(a, grid);
                    let pos_b = get_segment_start_point(b, grid);
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
                .min_by(|a, b| {
                    let pos_a = get_segment_start_point(a, grid);
                    let pos_b = get_segment_start_point(b, grid);
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
                    let pos_a = get_segment_start_point(a, grid);
                    let pos_b = get_segment_start_point(b, grid);
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

/// Group segments by connectivity and compatibility
pub fn group_segments_by_connectivity(
    segments: &HashSet<String>,
    grid: &CachedGrid,
    graph: &SegmentGraph,
) -> Vec<Vec<String>> {
    let mut groups = Vec::new();
    let mut visited: HashSet<String> = HashSet::new();

    for segment_id in segments {
        if visited.contains(segment_id) {
            continue;
        }

        let mut group = Vec::new();
        let mut queue = std::collections::VecDeque::new();
        queue.push_back(segment_id.clone());

        while let Some(current) = queue.pop_front() {
            if visited.contains(&current) {
                continue;
            }

            visited.insert(current.clone());
            group.push(current.clone());

            // Get current segment
            let current_segment = grid.segments.get(&current).unwrap();

            // Explore connected segments
            for neighbor in graph.neighbors_of(&current) {
                if !segments.contains(&neighbor) || visited.contains(&neighbor) {
                    continue;
                }

                // Get neighbor segment
                let neighbor_segment = grid.segments.get(&neighbor).unwrap();

                // Add to group if types are compatible
                if are_compatible_segments(current_segment, neighbor_segment) {
                    queue.push_back(neighbor);
                }
            }
        }

        if !group.is_empty() {
            groups.push(group);
        }
    }

    groups
}

/// Score the natural flow between two segments
pub fn score_segment_flow(
    current_id: &str,
    next_id: &str,
    grid: &CachedGrid,
    primary_type: &SegmentType,
) -> f32 {
    let current_pos = get_segment_start_point(current_id, grid);
    let next_pos = get_segment_start_point(next_id, grid);

    // Special handling for arc segments that form a circle
    if is_arc_type(primary_type) {
        let current_segment = grid.segment(current_id).unwrap();
        let next_segment = grid.segment(next_id).unwrap();

        // Determine if these are adjacent arcs in a circle
        match (&current_segment.segment_type, &next_segment.segment_type) {
            // Prioritize circular patterns (clockwise or counter-clockwise)
            (SegmentType::ArcTopLeft, SegmentType::ArcTopRight) => return 0.0,
            (SegmentType::ArcTopRight, SegmentType::ArcBottomRight) => return 0.0,
            (SegmentType::ArcBottomRight, SegmentType::ArcBottomLeft) => return 0.0,
            (SegmentType::ArcBottomLeft, SegmentType::ArcTopLeft) => return 0.0,
            // Second priority: reverse circular patterns
            (SegmentType::ArcTopRight, SegmentType::ArcTopLeft) => return 3.0,
            (SegmentType::ArcBottomRight, SegmentType::ArcTopRight) => return 3.0,
            (SegmentType::ArcBottomLeft, SegmentType::ArcBottomRight) => return 3.0,
            (SegmentType::ArcTopLeft, SegmentType::ArcBottomLeft) => return 3.0,
            _ => return 100.0, // Non-adjacent arcs get a higher score
        }
    }

    match primary_type {
        SegmentType::Horizontal => {
            // For horizontal, prefer moving right
            (next_pos.x - current_pos.x).abs() * 10.0
                + if next_pos.x < current_pos.x {
                    1000.0 // Penalize moving left
                } else {
                    0.0
                }
        }
        SegmentType::Vertical => {
            // For vertical, prefer moving down
            (next_pos.y - current_pos.y).abs() * 10.0
                + if next_pos.y > current_pos.y {
                    1000.0 // Penalize moving up
                } else {
                    0.0
                }
        }
        _ => {
            // Default scoring based on distance
            (next_pos - current_pos).length()
        }
    }
}

/// Order a list of segments based on natural writing flow
pub fn order_segments_by_flow(
    segments: &[String],
    start_segment: &str,
    grid: &CachedGrid,
    graph: &SegmentGraph,
    primary_type: &SegmentType,
) -> Vec<String> {
    let mut ordered = Vec::new();
    let mut visited = HashSet::new();

    // Start with the designated start segment
    let mut current = start_segment.to_string();
    ordered.push(current.clone());
    visited.insert(current.clone());

    // Traverse the segments using segment flow scores
    while ordered.len() < segments.len() {
        let mut best_next = None;
        let mut best_score = f32::MAX;

        // Find unvisited neighbors
        for neighbor in graph.neighbors_of(&current) {
            if segments.contains(&neighbor) && !visited.contains(&neighbor) {
                // Score based on position relative to current segment's flow
                let score = score_segment_flow(&current, &neighbor, grid, primary_type);
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
            for segment in segments {
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

/// Assign priority to segment types for ordering
pub fn get_segment_type_priority(segment_type: &SegmentType) -> u8 {
    match segment_type {
        SegmentType::ArcTopLeft => 1, // Highest priority
        SegmentType::ArcTopRight => 2,
        SegmentType::ArcBottomLeft => 3,
        SegmentType::ArcBottomRight => 4,
        SegmentType::Horizontal => 5,
        SegmentType::Vertical => 6,
        SegmentType::Unknown => 7, // Lowest priority
    }
}

/// Find all connections between segments at intersection points
pub fn identify_segment_connections(
    segment_groups: &[Vec<String>],
    graph: &SegmentGraph,
) -> HashMap<String, Vec<String>> {
    let mut connections: HashMap<String, Vec<String>> = HashMap::new();

    // For each group
    for group in segment_groups {
        for segment_id in group {
            // For each segment in the group, find its connections to other groups
            let neighbors = graph.neighbors_of(segment_id);

            for neighbor in neighbors {
                // Skip connections within the same group
                if group.contains(&neighbor) {
                    continue;
                }

                // Add connection if the neighbor is in another group
                for other_group in segment_groups {
                    if other_group != group && other_group.contains(&neighbor) {
                        connections
                            .entry(segment_id.clone())
                            .or_default()
                            .push(neighbor.clone());
                    }
                }
            }
        }
    }

    // Remove duplicates in the connection lists
    for connected_list in connections.values_mut() {
        connected_list.sort();
        connected_list.dedup();
    }

    connections
}
