// src/views/grid/grid_generic.rs

// The SVG grid data structures are converted to draw commands and
// cached in the structures in this module.
//
// The structures are like the "hardware" of the visualisation.
//
// The CachedGrid holds the entire grid of CachedSegments, provides
// general methods for instantiating a grid from the Project file, and
// general methods for drawing and transforming the grid.
//
// CachedSegments hold the pre-processed draw commands for a single
// segment. Also representing a segment's "hardware", it is responsible
// for updating its style and drawing itself.
//
// Main Types in this module:
// DrawCommand, DrawStyle, CachedSegment, and CachedGrid
//
// Suppporting Types:
// Layer, SegmentAction, StyleUpdateMsg

use nannou::prelude::*;
use std::collections::HashMap;
use std::time::Instant;

use crate::{
    models::{EdgeType, PathElement, Project, ViewBox},
    utilities::{
        easing, grid_utility, segment_utility,
        svg::{edge_detection, parser},
    },
    views::Transform2D,
};

// TODO: USE ANIMATION DURATION CONFIG INSTEAD OF THESE CONSTANTS
pub const ARC_RESOLUTION: usize = 25;
const FLASH_DURATION: f32 = 0.07;
const FADE_DURATION: f32 = 0.15;
const FLASH_FADE_DURATION: f32 = 0.12;

// DrawCommand is a single drawing operation that has been pre-processed from
// SVG path data
#[derive(Debug, Clone)]
pub enum DrawCommand {
    Line { start: Point2, end: Point2 },
    Arc { points: Vec<Point2> },
    Circle { center: Point2, radius: f32 },
}

impl DrawCommand {
    fn apply_transform(&mut self, transform: &Transform2D) {
        match self {
            DrawCommand::Line { start, end, .. } => {
                *start = transform.apply_to_point(*start);
                *end = transform.apply_to_point(*end);
            }
            DrawCommand::Arc { points, .. } => {
                for point in points {
                    *point = transform.apply_to_point(*point);
                }
            }
            DrawCommand::Circle { center, radius, .. } => {
                *center = transform.apply_to_point(*center);
                *radius *= transform.scale;
            }
        }
    }

    fn draw(&self, draw: &Draw, style: &DrawStyle) {
        match self {
            DrawCommand::Line { start, end, .. } => {
                draw.line()
                    .start(*start)
                    .end(*end)
                    .stroke_weight(style.stroke_weight)
                    .color(style.color)
                    .caps_round();
            }
            DrawCommand::Arc { points, .. } => {
                for window in points.windows(2) {
                    if let [p1, p2] = window {
                        draw.line()
                            .start(*p1)
                            .end(*p2)
                            .stroke_weight(style.stroke_weight)
                            .color(style.color)
                            .caps_round();
                    }
                }
            }
            DrawCommand::Circle { center, radius, .. } => {
                draw.ellipse()
                    .x_y(center.x, center.y)
                    .radius(*radius)
                    .stroke(style.color)
                    .stroke_weight(style.stroke_weight)
                    .color(style.color)
                    .caps_round();
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct DrawStyle {
    pub color: Rgba<f32>,
    pub stroke_weight: f32,
}

impl Default for DrawStyle {
    fn default() -> Self {
        Self {
            color: rgba(0.82, 0.0, 0.14, 1.0),
            stroke_weight: 5.1,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Layer {
    Background,
    Middle,
    Foreground,
}

#[derive(Debug, Clone, PartialEq)]
pub enum SegmentAction {
    On,
    Off,
    BackboneUpdate,
    InstantStyleChange,
}

#[derive(Debug, Clone)]
pub struct StyleUpdateMsg {
    pub action: Option<SegmentAction>,
    pub target_style: Option<DrawStyle>,
}

impl StyleUpdateMsg {
    pub fn new(action: SegmentAction, target_style: DrawStyle) -> Self {
        Self {
            action: Some(action),
            target_style: Some(target_style),
        }
    }
}

#[derive(Debug, Clone)]
pub enum SegmentState {
    Idle {
        style: DrawStyle,
    },
    PoweringOn {
        start_time: Instant,
        target_style: DrawStyle,
    },
    PoweringOff {
        start_time: Instant,
        from_style: DrawStyle,
        target_style: DrawStyle,
    },
    Active {
        style: DrawStyle,
    },
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum SegmentType {
    Horizontal,
    Vertical,
    ArcTopLeft,     // arc-1
    ArcTopRight,    // arc-2
    ArcBottomLeft,  // arc-3
    ArcBottomRight, // arc-4
    Unknown,
}

// A CachedSegment contains pre-processed draw commands for a segment
// Acts like a virtual light fixture, responds to style update messages
#[derive(Debug, Clone)]
pub struct CachedSegment {
    // metadata
    pub id: String,
    pub tile_coordinate: (u32, u32),
    pub segment_type: SegmentType,
    pub layer: Layer,

    // style state
    state: SegmentState,

    // draw commands cache
    pub draw_commands: Vec<DrawCommand>,
    pub original_path: PathElement,
    pub edge_type: EdgeType,
}

impl CachedSegment {
    pub fn new(
        element_id: String,
        tile_coordinate: (u32, u32),
        path: &PathElement,
        edge_type: EdgeType,
        viewbox: &ViewBox,
        grid_dims: (u32, u32),
    ) -> Self {
        // create the transformation to this tile's position
        let tile_transform =
            segment_utility::calculate_tile_transform(viewbox, tile_coordinate, grid_dims);

        // Generate commands with tile transform
        let draw_commands = segment_utility::generate_draw_commands(path, viewbox, &tile_transform);

        // Determine SegmentType from PathElement
        let segment_type = match &path {
            PathElement::Line { x1, y1, x2, y2 } => {
                let dx = (x2 - x1).abs();
                let dy = (y2 - y1).abs();
                if dx > dy {
                    SegmentType::Horizontal
                } else {
                    SegmentType::Vertical
                }
            }
            PathElement::Arc {
                start_x,
                start_y,
                end_x,
                end_y,
                ..
            } => grid_utility::classify_arc(start_x, start_y, end_x, end_y),
            PathElement::Circle { .. } => SegmentType::Unknown,
        };

        Self {
            id: element_id,
            tile_coordinate,
            segment_type,
            layer: Layer::Background,

            // segment starts out in the Idle state
            state: SegmentState::Idle {
                style: DrawStyle::default(),
            },

            draw_commands,
            original_path: path.clone(),
            edge_type,
        }
    }

    /**************************  Style functions *************************************** */

    fn update_animation(&mut self) {
        match &self.state {
            SegmentState::PoweringOn {
                start_time,
                target_style,
            } => {
                let elapsed_time = start_time.elapsed().as_secs_f32();
                if elapsed_time <= FLASH_DURATION + FLASH_FADE_DURATION {
                    self.layer = Layer::Foreground;
                } else {
                    // Animation complete
                    self.state = SegmentState::Active {
                        style: target_style.clone(),
                    }
                }
            }

            SegmentState::PoweringOff {
                start_time,
                from_style: _,
                target_style,
            } => {
                let elapsed_time = start_time.elapsed().as_secs_f32();
                if elapsed_time <= FLASH_DURATION + FLASH_FADE_DURATION {
                    self.layer = Layer::Middle;
                } else {
                    // Animation complete
                    self.layer = Layer::Background;
                    self.state = SegmentState::Idle {
                        style: target_style.clone(),
                    }
                }
            }
            _ => {}
        };
    }

    pub fn current_style(&self) -> DrawStyle {
        match &self.state {
            SegmentState::Idle { style } | SegmentState::Active { style } => style.clone(),
            SegmentState::PoweringOn { .. } => self.calculate_transition_style(),
            SegmentState::PoweringOff { .. } => self.calculate_transition_style(),
        }
    }

    fn calculate_transition_style(&self) -> DrawStyle {
        match &self.state {
            SegmentState::PoweringOn {
                start_time,
                target_style,
            } => {
                let elapsed_time = start_time.elapsed().as_secs_f32();
                let flash_color = rgba(1.0, 1.0, 0.8, 1.0);
                let color = if elapsed_time <= FLASH_DURATION {
                    flash_color
                } else if elapsed_time <= FLASH_DURATION + FLASH_FADE_DURATION {
                    // Fade to target color
                    let fade_progress = (elapsed_time - FLASH_DURATION) / FLASH_FADE_DURATION;
                    easing::color_exp_ease(flash_color, target_style.color, fade_progress, 6.0)
                } else {
                    // Animation complete
                    target_style.color
                };

                DrawStyle {
                    color,
                    stroke_weight: target_style.stroke_weight,
                }
            }

            SegmentState::PoweringOff {
                start_time,
                from_style,
                target_style,
            } => {
                let elapsed_time = start_time.elapsed().as_secs_f32();

                // Calculate color based on animation phase
                let color = if elapsed_time <= FADE_DURATION {
                    // Fade to target color
                    let fade_progress = elapsed_time / FADE_DURATION;
                    easing::color_exp_ease(from_style.color, target_style.color, fade_progress, 6.0)
                } else {
                    // Animation complete
                    target_style.color
                };

                DrawStyle {
                    color,
                    stroke_weight: target_style.stroke_weight,
                }
            }

            SegmentState::Idle { style } | SegmentState::Active { style } => style.clone(),
        }
    }

    pub fn process_style_update_msg(&mut self, msg: &StyleUpdateMsg) {
        match (&msg.action, &msg.target_style) {
            (Some(action), Some(target_style)) => {
                match action {
                    SegmentAction::On => {
                        // Update the style for active segments
                        self.state = SegmentState::PoweringOn {
                            start_time: Instant::now(),
                            target_style: target_style.clone(),
                        }
                    }
                    SegmentAction::Off => {
                        self.state = SegmentState::PoweringOff {
                            start_time: Instant::now(),
                            from_style: self.current_style(),
                            target_style: target_style.clone(),
                        }
                    }
                    SegmentAction::BackboneUpdate => {
                        self.state = SegmentState::Idle {
                            style: target_style.clone(),
                        }
                    }
                    SegmentAction::InstantStyleChange => {
                        // instantly change to target style without animations or effects
                        self.state = SegmentState::Active {
                            style: target_style.clone(),
                        };
                        self.layer = Layer::Foreground;
                    }
                }
            }
            (None, Some(target_style)) => {
                // Direct style update without action
                self.state = SegmentState::Active {
                    style: target_style.clone(),
                }
            }
            _ => {}
        }
    }

    /**************************  Transform functions *************************************** */

    pub fn apply_transform(&mut self, transform: &Transform2D) {
        for command in &mut self.draw_commands {
            command.apply_transform(transform);
        }
    }

    pub fn scale_stroke_weight(&mut self, scale_factor: f32) {
        match &mut self.state {
            SegmentState::Idle { style } | SegmentState::Active { style } => {
                style.stroke_weight *= scale_factor;
            }
            SegmentState::PoweringOn { target_style, .. } => {
                target_style.stroke_weight *= scale_factor;
            }
            SegmentState::PoweringOff {
                from_style,
                target_style,
                ..
            } => {
                from_style.stroke_weight *= scale_factor;
                target_style.stroke_weight *= scale_factor;
            }
        }
    }

    /************************ Utility Methods ****************************/

    pub fn is_background(&self) -> bool {
        matches!(self.layer, Layer::Background)
    }

    pub fn is_idle(&self) -> bool {
        matches!(self.state, SegmentState::Idle { .. })
    }
}

// CachedGrid stores the pre-processed drawing commands for an entire grid
#[derive(Debug, Clone)]
pub struct CachedGrid {
    pub dimensions: (u32, u32), // number of tiles in x and y
    pub segments: HashMap<String, CachedSegment>,
    pub viewbox: ViewBox,
}

impl CachedGrid {
    pub fn new(project: &Project) -> Self {
        // Parse viewbox from SVG
        let viewbox = grid_utility::parse_viewbox(&project.svg_base_tile)
            .expect("Failed to parse viewbox from SVG");

        // Parse the SVG & create basic grid elements
        let elements = parser::parse_svg(&project.svg_base_tile);
        let grid_dims = (project.grid_x, project.grid_y);
        let mut segments = HashMap::new();

        // Create grid elements and detect edges
        for y in 1..=project.grid_y {
            for x in 1..=project.grid_x {
                for element in &elements {
                    let edge_type = edge_detection::detect_edge_type(&element.path, &viewbox);
                    let element_id = format!("{},{} : {}", x, y, element.id);
                    let segment = CachedSegment::new(
                        element_id.clone(),
                        (x, y),
                        &element.path,
                        edge_type,
                        &viewbox,
                        grid_dims,
                    );
                    /*
                    // Only print edge elements for brevity
                    if edge_type != EdgeType::None {
                        println!("Created {} at ({},{}) - {:?}", element_id, x, y, edge_type);
                    }
                    */

                    segments.insert(segment.id.clone(), segment);
                }
            }
        }

        // Remove overlapping segments
        //segments = purge_overlapping_segments(segments, project.grid_x, project.grid_y);

        Self {
            dimensions: (project.grid_x, project.grid_y),
            segments,
            viewbox,
        }
    }

    /************************ Rendering ****************************/
    pub fn draw(
        &mut self,
        draw: &Draw,
        update_batch: &HashMap<String, StyleUpdateMsg>,
        visible: bool,
    ) {
        let mut foreground_segments = Vec::new();
        let mut middle_segments = Vec::new();

        for segment in self.segments.values_mut() {
            if let Some(msg) = update_batch.get(&segment.id) {
                segment.process_style_update_msg(msg);
            }

            segment.update_animation();

            // draw background layer first, or prepare other layers
            if visible {
                match segment.layer {
                    Layer::Background => {
                        for command in &segment.draw_commands {
                            command.draw(draw, &segment.current_style());
                        }
                    }
                    Layer::Middle => {
                        middle_segments.push(segment.clone());
                    }
                    Layer::Foreground => {
                        foreground_segments.push(segment.clone());
                    }
                }
            }
        }

        if visible {
            for segment in middle_segments {
                for command in &segment.draw_commands {
                    command.draw(draw, &segment.current_style());
                }
            }

            for segment in foreground_segments {
                for command in &segment.draw_commands {
                    command.draw(draw, &segment.current_style());
                }
            }
        }
    }

    /************************ Transform Methods **************************/

    pub fn apply_transform(&mut self, transform: &Transform2D) {
        //self.transform = transform.clone();
        for segment in self.segments.values_mut() {
            segment.apply_transform(transform);
        }
    }

    pub fn scale_stroke_weights(&mut self, scale_factor: f32) {
        for segment in self.segments.values_mut() {
            segment.scale_stroke_weight(scale_factor);
        }
    }

    pub fn slide(&mut self, axis: &str, number: i32, distance: f32) {
        if axis != "x" && axis != "y" {
            println!("Slide axis value must be x or y. Current value is {}", axis);
        }
        let translation = match axis {
            "x" => vec2(distance, 0.0),
            "y" => vec2(0.0, distance),
            _ => vec2(0.0, 0.0),
        };

        let transform = Transform2D {
            translation,
            scale: 1.0,
            rotation: 0.0,
        };

        let segments = match axis {
            "x" => self.row_mut(number),
            "y" => self.col_mut(number),
            _ => Vec::new(),
        };

        for segment in segments {
            segment.apply_transform(&transform);
        }
    }

    fn row_mut(&mut self, number: i32) -> Vec<&mut CachedSegment> {
        // check that number is a valid index
        if number < 0 {
            return Vec::new();
        }
        let index = number as u32;

        self.segments
            .values_mut()
            .filter(|segment| segment.tile_coordinate.1 == index)
            .collect()
    }

    fn col_mut(&mut self, number: i32) -> Vec<&mut CachedSegment> {
        // check that number is a valid index
        if number < 0 {
            return Vec::new();
        }
        let index = number as u32;

        self.segments
            .values_mut()
            .filter(|segment| segment.tile_coordinate.0 == index)
            .collect()
    }

    /************************ Utility Methods ****************************/

    pub fn get_tile_segments_iter(&self, x: u32, y: u32) -> impl Iterator<Item = &CachedSegment> {
        self.segments
            .values()
            .filter(move |segment| segment.tile_coordinate == (x, y))
    }

    pub fn segment(&self, id: &str) -> Option<&CachedSegment> {
        self.segments.get(id)
    }

    /************************ Validation ****************************/

    pub fn validate_segment_points(&self) -> bool {
        for segment in self.segments.values() {
            for command in &segment.draw_commands {
                match command {
                    DrawCommand::Line { start, end, .. } => {
                        if !start.x.is_finite()
                            || !start.y.is_finite()
                            || !end.x.is_finite()
                            || !end.y.is_finite()
                        {
                            println!("Line error at segment {}", segment.id);
                            println!("Invalid line points: start={:?}, end={:?}", start, end);
                            return false;
                        }
                    }
                    DrawCommand::Arc { points, .. } => {
                        for point in points {
                            if !point.x.is_finite() || !point.y.is_finite() {
                                println!("Invalid arc point: {:?}", point);
                                return false;
                            }
                        }
                    }
                    DrawCommand::Circle { center, radius, .. } => {
                        if !center.x.is_finite() || !center.y.is_finite() || !radius.is_finite() {
                            println!("Invalid circle: center={:?}, radius={}", center, radius);
                            return false;
                        }
                    }
                }
            }
        }
        true
    }
}

/************************ CachedGrid Initialization Helper ****************************/

// Unlike Glyphmaker, where we draw all elements and then handle selection logic,
// in Glyphvis we decide on whether to draw an element at the beginning.
//
// This function doesn't work! Run grid.slide() to see the problem.
// But we decided not to use it because grid.slide looks better without purging.
fn _purge_overlapping_segments(
    segments: HashMap<String, CachedSegment>,
    grid_width: u32,
    grid_height: u32,
) -> HashMap<String, CachedSegment> {
    let mut final_segments = HashMap::new();

    // Group segments by position for easier overlap checking
    let mut segments_by_pos: HashMap<(u32, u32), Vec<&CachedSegment>> = HashMap::new();
    for segment in segments.values() {
        segments_by_pos
            .entry(segment.tile_coordinate)
            .or_default()
            .push(segment);
    }

    // Check each segment against its potential neighbors
    for segment in segments.values() {
        // Skip if it's not an edge
        if segment.edge_type == EdgeType::None {
            final_segments.insert(
                segment.id.clone(),
                segments.get(&segment.id).unwrap().clone(),
            );
            continue;
        }

        // Get potential neighbors based on edge type
        if let Some((neighbor_x, neighbor_y)) = grid_utility::get_neighbor_coords(
            segment.tile_coordinate.0,
            segment.tile_coordinate.1,
            segment.edge_type,
            grid_width,
            grid_height,
        ) {
            // check if neighbor has priority
            let neighbor_has_priority = neighbor_x < segment.tile_coordinate.0
                || (neighbor_x == segment.tile_coordinate.1
                    && neighbor_y < segment.tile_coordinate.1);

            if neighbor_has_priority {
                // Look for matching segments at neighbor position
                if let Some(neighbor_segments) = segments_by_pos.get(&(neighbor_x, neighbor_y)) {
                    let mut should_keep = true;

                    for neighbor in neighbor_segments {
                        let direction = grid_utility::get_neighbor_direction(
                            segment.tile_coordinate.0,
                            segment.tile_coordinate.1,
                            neighbor_x,
                            neighbor_y,
                        );

                        if grid_utility::check_segment_alignment(segment, neighbor, direction) {
                            should_keep = false;
                            break;
                        }
                    }

                    if should_keep {
                        final_segments.insert(
                            segment.id.clone(),
                            segments.get(&segment.id).unwrap().clone(),
                        );
                    }
                }
            } else {
                // This segment has priority, keep it
                final_segments.insert(
                    segment.id.clone(),
                    segments.get(&segment.id).unwrap().clone(),
                );
            }
        } else {
            // No valid neighbor position, keep the segment
            final_segments.insert(
                segment.id.clone(),
                segments.get(&segment.id).unwrap().clone(),
            );
        }
    }
    // return:
    final_segments
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

    const TEST_GRID_DIMS: (u32, u32) = (1, 1);

    mod draw_command_tests {
        use super::*;

        #[test]
        fn test_draw_command_transform() {
            let transform = Transform2D {
                translation: Vec2::new(10.0, 10.0),
                scale: 2.0,
                rotation: 0.0,
            };

            // Test Line transformation
            let mut line = DrawCommand::Line {
                start: pt2(0.0, 0.0),
                end: pt2(5.0, 5.0),
            };
            line.apply_transform(&transform);
            match line {
                DrawCommand::Line { start, end, .. } => {
                    assert_eq!(start, pt2(10.0, 10.0));
                    assert_eq!(end, pt2(20.0, 20.0));
                }
                _ => panic!("Wrong variant"),
            }

            // Test Circle transformation
            let mut circle = DrawCommand::Circle {
                center: pt2(0.0, 0.0),
                radius: 5.0,
            };
            circle.apply_transform(&transform);
            match circle {
                DrawCommand::Circle { center, radius, .. } => {
                    assert_eq!(center, pt2(10.0, 10.0));
                    assert_eq!(radius, 10.0);
                }
                _ => panic!("Wrong variant"),
            }
        }
    }

    mod cached_segment_tests {
        use super::*;

        #[test]
        fn test_cached_segment_creation() {
            let viewbox = create_test_viewbox();
            let path = PathElement::Line {
                x1: 0.0,
                y1: 0.0,
                x2: 10.0,
                y2: 10.0,
            };

            let segment = CachedSegment::new(
                "test".to_string(),
                (1, 1),
                &path,
                EdgeType::None,
                &viewbox,
                TEST_GRID_DIMS,
            );

            assert_eq!(segment.id, "test");
            assert_eq!(segment.tile_coordinate, (1, 1));
            assert_eq!(segment.edge_type, EdgeType::None);
            assert!(!segment.draw_commands.is_empty());
        }

        #[test]
        fn test_coordinate_transformation() {
            let viewbox = create_test_viewbox();

            // Test center point transformation
            let path = PathElement::Circle {
                cx: 50.0, // Center of viewbox
                cy: 50.0,
                r: 5.0,
            };

            let segment = CachedSegment::new(
                "center_test".to_string(),
                (1, 1),
                &path,
                EdgeType::None,
                &viewbox,
                TEST_GRID_DIMS,
            );

            // Center point should be transformed to (0,0) in Nannou coordinates
            match &segment.draw_commands[0] {
                DrawCommand::Circle { center, .. } => {
                    println!("Center: {:?}", center);
                    assert_eq!(center.x, 0.0);
                    assert_eq!(center.y, 0.0);
                }
                _ => panic!("Expected Circle"),
            }
        }
    }

    mod cached_grid_tests {
        use super::*;

        fn create_test_project() -> Project {
            // Create minimal project for testing
            Project {
                svg_base_tile: r#"<svg id="test" viewBox="0 0 100 100">
                    <path id="line1" d="M0,0 L100,0"/>
                    <circle id="circle1" cx="50" cy="50" r="5"/>
                </svg>"#
                    .to_string(),
                grid_x: 2,
                grid_y: 2,
                glyphs: HashMap::new(),
                shows: HashMap::new(),
            }
        }

        #[test]
        fn test_grid_creation() {
            let project = create_test_project();
            let grid = CachedGrid::new(&project);

            assert_eq!(grid.dimensions, (2, 2));
            assert!(!grid.segments.is_empty());
        }

        #[test]
        fn test_overlap_elimination() {
            let project = create_test_project();
            let grid = CachedGrid::new(&project);

            // Test that overlapping edges are properly eliminated
            // For example, if we have a horizontal line at y=0, it should only appear
            // in either the top or bottom tile, not both
            let top_segments = grid.get_tile_segments_iter(1, 1);
            let bottom_segments = grid.get_tile_segments_iter(1, 2);

            // Ensure we don't have the same edge in both tiles
            let top_edges: Vec<EdgeType> = top_segments.map(|s| s.edge_type).collect();
            let bottom_edges: Vec<EdgeType> = bottom_segments.map(|s| s.edge_type).collect();

            assert!(
                !(top_edges.contains(&EdgeType::South) && bottom_edges.contains(&EdgeType::North))
            );
        }
    }
}
