// src/animation/stroke_order.rs
//
// extension to TransitionEngine that creates Transitions that follow
// a natural writing style

use crate::{
    animation::transition::SegmentChange,
    services::SegmentGraph,
    views::{CachedGrid, CachedSegment, DrawCommand, GridInstance, SegmentType},
};

use nannou::prelude::*;
use std::collections::{HashMap, HashSet, VecDeque};

#[derive(Clone)]
struct Stroke {
    segments: Vec<String>,
    start_segment: String,
    end_segment: String,
    primary_type: SegmentType,
    start_position: Point2,
}

pub fn generate_stroke_order(
    grid_instance: &GridInstance,
    target_segments: &HashSet<String>,
) -> Vec<String> {
    let grid = &grid_instance.grid;
    let graph = &grid_instance.graph;
    let start_segments = &grid_instance.current_active_segments;

    // Find segments to turn on
    let segments_to_turn_on: HashSet<_> = target_segments
        .difference(start_segments)
        .cloned()
        .collect();

    if segments_to_turn_on.is_empty() {
        return Vec::new();
    }

    // Step 1: Group segments into strokes
    let strokes = group_segments_into_strokes(&segments_to_turn_on, grid, graph);

    // Step 2: For each stroke, order the segments within it to follow writing direction
    let mut ordered_strokes = Vec::new();
    for stroke in strokes {
        let (ordered_segments, end_segment) = order_segments_in_stroke(&stroke, grid, graph);
        ordered_strokes.push(Stroke {
            segments: ordered_segments,
            start_segment: stroke.start_segment.clone(),
            end_segment,
            primary_type: stroke.primary_type,
            start_position: stroke.start_position,
        });
    }

    // Step 3: Identify connections between strokes
    let stroke_connections = identify_connections(&ordered_strokes, graph);

    // Step 4: Order the strokes considering connections and quadrants
    ordered_strokes = order_strokes_by_position(ordered_strokes, &stroke_connections, grid);

    // Step 5: Process strokes in order, with special handling for connected strokes
    order_strokes_with_connections(ordered_strokes, &stroke_connections)
}

pub fn convert_to_transition_changes(
    ordered_segments: Vec<String>,
    grid_instance: &GridInstance,
) -> Vec<Vec<SegmentChange>> {
    let start_segments = &grid_instance.current_active_segments;
    let target_segments = grid_instance.target_segments.as_ref().unwrap();

    // First, handle segments that need to be turned on
    let mut changes = Vec::new();

    // Create a change for each segment to be turned on (one at a time)
    for segment_id in ordered_segments {
        changes.push(vec![SegmentChange {
            segment_id,
            turn_on: true,
        }]);
    }

    // Now handle segments that need to be turned off
    let segments_to_turn_off: Vec<_> = start_segments
        .difference(target_segments)
        .cloned()
        .collect();

    if !segments_to_turn_off.is_empty() {
        let turn_off_changes = segments_to_turn_off
            .into_iter()
            .map(|segment_id| SegmentChange {
                segment_id,
                turn_on: false,
            })
            .collect();

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
                end_segment: "".to_string(),
                primary_type,
                start_position,
            });
        }
    }
    strokes
}

// Check if two segments should be part of the same stroke
fn are_compatible_segments(seg1: &CachedSegment, seg2: &CachedSegment) -> bool {
    if seg1.segment_type == seg2.segment_type {
        return true;
    }

    // Special handling for arcs that form a circle
    if is_arc_type(&seg1.segment_type) && is_arc_type(&seg2.segment_type) {
        // Check if these arcs are adjacent in a circular pattern
        // For example: ArcTopLeft followed by ArcTopRight forms the top half of a circle
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
                .max_by(|a, b| {
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

fn order_strokes_with_connections(
    strokes: Vec<Stroke>,
    connections: &HashMap<String, Vec<String>>,
) -> Vec<String> {
    // Now we'll reorder based on connected strokes
    let mut final_order = Vec::new();
    let mut processed_strokes: HashSet<String> = HashSet::new();
    let mut remaining_strokes: HashSet<String> =
        strokes.iter().map(|s| s.start_segment.clone()).collect();

    while !remaining_strokes.is_empty() {
        // Get the next stroke based on our ordering
        let next_stroke_id = find_next_stroke(&strokes, &remaining_strokes);
        let next_stroke = strokes
            .iter()
            .find(|s| s.start_segment == next_stroke_id)
            .unwrap();

        // Add all segments from this stroke to the final order
        for segment_id in &next_stroke.segments {
            final_order.push(segment_id.clone());
        }

        // Mark this stroke as processed
        processed_strokes.insert(next_stroke_id.clone());
        remaining_strokes.remove(&next_stroke_id);

        // Check for connected strokes
        if let Some(connected_stroke_ids) = connections.get(&next_stroke_id) {
            let mut sorted_connected_ids = connected_stroke_ids.clone();
            sorted_connected_ids.sort();
            // Find unprocessed connected strokes
            let available_connected: Vec<&String> = sorted_connected_ids
                .iter()
                .filter(|id| !processed_strokes.contains(*id))
                .collect();

            if !available_connected.is_empty() {
                // Get the corresponding stroke objects
                let connected_strokes: Vec<&Stroke> = strokes
                    .iter()
                    .filter(|s| available_connected.contains(&&s.start_segment))
                    .collect();

                // Sort by our specified priority
                let sorted_connected = sort_connected_strokes(connected_strokes);

                if !sorted_connected.is_empty() {
                    // Process the highest priority connected stroke next
                    let next_connected = sorted_connected[0];

                    // Add all segments from this connected stroke
                    for segment_id in &next_connected.segments {
                        final_order.push(segment_id.clone());
                    }

                    // Mark as processed
                    processed_strokes.insert(next_connected.start_segment.clone());
                    remaining_strokes.remove(&next_connected.start_segment);
                }
            }
        }
    }

    final_order
}

// Helper to find the next stroke from remaining strokes
fn find_next_stroke(ordered_strokes: &[Stroke], remaining: &HashSet<String>) -> String {
    for stroke in ordered_strokes {
        if remaining.contains(&stroke.start_segment) {
            return stroke.start_segment.clone();
        }
    }
    // Fallback if something goes wrong
    remaining.iter().next().unwrap().clone()
}

// Sort connected strokes using basic rules
fn sort_connected_strokes(strokes: Vec<&Stroke>) -> Vec<&Stroke> {
    let mut sorted = strokes.clone();
    sorted.sort_by(|a, b| {
        // First prioritize by segment type according to specified order
        let type_a_priority = get_type_priority(&a.primary_type);
        let type_b_priority = get_type_priority(&b.primary_type);

        if type_a_priority != type_b_priority {
            return type_a_priority.cmp(&type_b_priority);
        }

        // Then by horizontal position (left to right)
        if (a.start_position.x - b.start_position.x).abs() > 1.0 {
            return a
                .start_position
                .x
                .partial_cmp(&b.start_position.x)
                .unwrap_or(std::cmp::Ordering::Equal);
        }
        // If same type priority, sort by vertical position (top to bottom)
        if (a.start_position.y - b.start_position.y).abs() > 1.0 {
            return b
                .start_position
                .y
                .partial_cmp(&a.start_position.y)
                .unwrap_or(std::cmp::Ordering::Equal);
        }

        std::cmp::Ordering::Equal
    });

    sorted
}

// Helper function to assign priority to segment types
fn get_type_priority(segment_type: &SegmentType) -> u8 {
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

fn order_strokes_by_position(
    mut strokes: Vec<Stroke>,
    connections: &HashMap<String, Vec<String>>,
    grid: &CachedGrid,
) -> Vec<Stroke> {
    let mut result = Vec::new();
    let mut remaining: HashSet<String> = strokes.iter().map(|s| s.start_segment.clone()).collect();

    // Sort strokes by quadrant and position for initial ordering
    strokes.sort_by(|a, b| {
        // Define quadrant boundaries
        let mid_x = 2.4; // Horizontal middle of the grid
        let mid_y = 2.4; // Vertical middle of the grid

        // Get start segment tile
        let a_start_tile = grid.get_segment(&a.start_segment).unwrap().tile_coordinate;
        let b_start_tile = grid.get_segment(&b.start_segment).unwrap().tile_coordinate;

        // Determine which quadrant each stroke starts in
        let a_quadrant = get_quadrant(a_start_tile.0 as f32, a_start_tile.1 as f32, mid_x, mid_y);
        let b_quadrant = get_quadrant(b_start_tile.0 as f32, b_start_tile.1 as f32, mid_x, mid_y);

        // Rule 1: Quadrant 1 (top-left) before all others
        if a_quadrant == 1 && b_quadrant != 1 {
            return std::cmp::Ordering::Less;
        }
        if a_quadrant != 1 && b_quadrant == 1 {
            return std::cmp::Ordering::Greater;
        }

        // Rule 2: Quadrant 2 (top-right) before bottom half
        if a_quadrant == 2 && (b_quadrant == 3 || b_quadrant == 4) {
            return std::cmp::Ordering::Less;
        }
        if (a_quadrant == 3 || a_quadrant == 4) && b_quadrant == 2 {
            return std::cmp::Ordering::Greater;
        }

        // Rule 3: Quadrant 3 (bottom-left) before quadrant 4
        if a_quadrant == 3 && b_quadrant == 4 {
            return std::cmp::Ordering::Less;
        }
        if a_quadrant == 4 && b_quadrant == 3 {
            return std::cmp::Ordering::Greater;
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
            // Vertical before Horizontal
            (SegmentType::Vertical, SegmentType::Horizontal) => std::cmp::Ordering::Less,
            (SegmentType::Horizontal, SegmentType::Vertical) => std::cmp::Ordering::Greater,

            // Right-to-left arcs before left-to-right arcs
            (SegmentType::ArcTopLeft, SegmentType::ArcTopRight) => std::cmp::Ordering::Less,
            (SegmentType::ArcTopRight, SegmentType::ArcTopLeft) => std::cmp::Ordering::Greater,
            (SegmentType::ArcBottomLeft, SegmentType::ArcBottomRight) => std::cmp::Ordering::Less,
            (SegmentType::ArcBottomRight, SegmentType::ArcBottomLeft) => {
                std::cmp::Ordering::Greater
            }

            // !Horizontal before any arc
            (SegmentType::Horizontal, _) if is_arc_type(&b.primary_type) => {
                std::cmp::Ordering::Greater
            }
            (_, SegmentType::Horizontal) if is_arc_type(&a.primary_type) => {
                std::cmp::Ordering::Less
            }
            // !Vertical before any arc
            (SegmentType::Vertical, _) if is_arc_type(&b.primary_type) => {
                std::cmp::Ordering::Greater
            }
            (_, SegmentType::Vertical) if is_arc_type(&a.primary_type) => std::cmp::Ordering::Less,

            // Default equal if no other rule applies
            _ => std::cmp::Ordering::Equal,
        }
    });

    // Build the result by walking through the strokes in position order
    // but prioritizing connected strokes
    while !remaining.is_empty() {
        // Get the next stroke from those remaining, by position order
        let next_stroke_opt = strokes
            .iter()
            .find(|s| remaining.contains(&s.start_segment));

        if let Some(next_stroke) = next_stroke_opt {
            let stroke_id = next_stroke.start_segment.clone();
            result.push(next_stroke.clone());
            remaining.remove(&stroke_id);

            // Process all connected strokes immediately
            let mut connected_chain = vec![stroke_id.clone()];
            let mut i = 0;

            // Process the chain of connections
            while i < connected_chain.len() {
                let current_id = &connected_chain[i];

                // Find strokes connected to the current one
                if let Some(connected_ids) = connections.get(current_id) {
                    for connected_id in connected_ids {
                        if remaining.contains(connected_id)
                            && !connected_chain.contains(connected_id)
                        {
                            // Add to the chain and to the result
                            connected_chain.push(connected_id.clone());

                            if let Some(connected_stroke) =
                                strokes.iter().find(|s| &s.start_segment == connected_id)
                            {
                                result.push(connected_stroke.clone());
                                remaining.remove(connected_id);
                            }
                        }
                    }
                }

                i += 1;
            }
        } else {
            // Fallback - shouldn't happen with correct data
            break;
        }
    }

    result
}

// Function to identify connections between different segment types
fn identify_connections(strokes: &[Stroke], graph: &SegmentGraph) -> HashMap<String, Vec<String>> {
    let mut connections: HashMap<String, Vec<String>> = HashMap::new();

    // Sort strokes by ID for deterministic processing
    let sorted_strokes = {
        let mut s = strokes.to_vec();
        s.sort_by(|a, b| a.start_segment.cmp(&b.start_segment));
        s
    };

    for stroke in &sorted_strokes {
        // For regular strokes, check connections at the end segment
        if !stroke.end_segment.is_empty() {
            add_connections_from_segment(
                &stroke.end_segment,
                &stroke.start_segment,
                strokes,
                graph,
                &mut connections,
            );
        }

        // For arcs, also check connections at ALL segments in the stroke
        if is_arc_type(&stroke.primary_type) {
            for segment_id in &stroke.segments {
                // Skip the end segment which we already processed
                if segment_id == &stroke.end_segment {
                    continue;
                }

                add_connections_from_segment(
                    segment_id,
                    &stroke.start_segment,
                    strokes,
                    graph,
                    &mut connections,
                );
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

// Helper function to add connections from a specific segment
fn add_connections_from_segment(
    segment_id: &str,
    source_stroke_id: &str,
    strokes: &[Stroke],
    graph: &SegmentGraph,
    connections: &mut HashMap<String, Vec<String>>,
) {
    // Find all segments connected to this segment
    let mut connected_segments = graph.get_neighbors(segment_id);
    connected_segments.sort();

    // Find which strokes these segments belong to
    let connected_stroke_ids: Vec<String> = connected_segments
        .into_iter()
        .filter_map(|connected_segment| {
            // Find which stroke this segment belongs to
            strokes
                .iter()
                .find(|s| s.segments.contains(&connected_segment))
                .map(|s| s.start_segment.clone())
        })
        .filter(|id| id != source_stroke_id) // Don't include self-connections
        .collect();

    if !connected_stroke_ids.is_empty() {
        connections
            .entry(source_stroke_id.to_string())
            .or_default()
            .extend(connected_stroke_ids);
    }
}

// Helper function to determine quadrant
fn get_quadrant(x: f32, y: f32, mid_x: f32, mid_y: f32) -> u8 {
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

// Order segments within a stroke in natural writing order
fn order_segments_in_stroke(
    stroke: &Stroke,
    grid: &CachedGrid,
    graph: &SegmentGraph,
) -> (Vec<String>, String) {
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
    let end_segment = ordered.last().unwrap().clone();

    (ordered, end_segment)
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

    // Special handling for arc segments that form a circle
    if is_arc_type(primary_type) {
        let current_segment = grid.get_segment(current).unwrap();
        let next_segment = grid.get_segment(next).unwrap();

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
        SegmentType::ArcTopLeft
        | SegmentType::ArcTopRight
        | SegmentType::ArcBottomLeft
        | SegmentType::ArcBottomRight => {
            // For top-left arc, prefer counter-clockwise motion
            let dx = next_pos.x - current_pos.x;
            let dy = next_pos.y - current_pos.y;
            if current_pos.y > next_pos.y {
                dx.abs() // Moving down, prefer smaller horizontal change
            } else {
                dy.abs() // Moving horizontally, prefer smaller vertical change
            }
        }
        _ => {
            // Default scoring
            (next_pos - current_pos).length()
        }
    }
}
