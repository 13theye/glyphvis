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
const FLASH_DURATION: f32 = 0.132;
const FADE_DURATION: f32 = 0.132;
const FLASH_FADE_DURATION: f32 = 0.132;

// The color and thickness of the segment
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

// Which screen layer does the segment need to be drawn to?
#[derive(Debug, Clone, PartialEq)]
pub enum Layer {
    Background,
    Middle,
    Foreground,
}

// These messages tell the segment what to do on the next frame
#[derive(Debug, Clone, PartialEq)]
pub enum SegmentAction {
    On,                 // turn this segment on using PowerOn
    Off,                // turn this segment off using PowerOff
    BackboneUpdate,     // this segment is not active but needs to be updated via backbone effect
    InstantStyleChange, // just change the segment to the target style without any animation
}

// All segments are collected in the Grid's update_batch field,
// which is a Vec of segment_ids and StyleUpdateMsg.
#[derive(Debug, Clone)]
pub struct StyleUpdateMsg {
    pub action: Option<SegmentAction>, // when None, the segment just redraws as the previous frame state
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

// All the possible states of a segment.
#[derive(Debug, Clone)]
pub enum SegmentStateType {
    Idle,
    PoweringOn,
    PoweringOff,
    Active,
}

// This is too custom for the Ulsan project's grid type, and may need to be changed in
// the future. Currently it's used mostly for handwriting stroke-order simulation.
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

// A CachedSegment is the basic element of a Grid.
// Acts like a virtual light fixture, and is reponsible for its own drawing.
// Receives messages from the Grid that dictate its behavior for the next frame.
pub struct CachedSegment {
    // metadata
    pub id: String,
    pub tile_coordinate: (u32, u32), // which tile in the grid
    pub segment_type: SegmentType,

    // state
    pub current_style: DrawStyle, // current display style, here for quick access
    state: Box<dyn SegmentState>, // manages update behavior

    // draw instructions cache
    pub draw_commands: Vec<DrawCommand>, // Nannou draw command
    pub original_path: PathElement,      // SVG path
    pub edge_type: EdgeType,             // type of edge in the base tile
}

impl Clone for CachedSegment {
    fn clone(&self) -> Self {
        Self {
            id: self.id.clone(),
            tile_coordinate: self.tile_coordinate,
            segment_type: self.segment_type,
            current_style: self.current_style.clone(),
            state: self.state.clone_box(),
            draw_commands: self.draw_commands.clone(),
            original_path: self.original_path.clone(),
            edge_type: self.edge_type,
        }
    }
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

            // this isn't currently used so it's just tossed into the "Unknown" pile
            PathElement::Circle { .. } => SegmentType::Unknown,
        };

        Self {
            id: element_id,
            tile_coordinate,
            segment_type,

            // segment starts out in the Idle state
            state: Box::new(IdleState {
                style: DrawStyle::default(),
            }),
            current_style: DrawStyle::default(),

            draw_commands,
            original_path: path.clone(),
            edge_type,
        }
    }

    /**************************  State management *************************************** */

    // set up the segment state according to the StyleUpdateMessage in this frame's update batch
    fn update_segment_state(&mut self, msg: &StyleUpdateMsg) {
        match (&msg.action, &msg.target_style) {
            (Some(action), Some(target_style)) => {
                match action {
                    SegmentAction::On => {
                        // Update the style for active segments
                        let new_state = Box::new(PoweringOnState {
                            start_time: Instant::now(),
                            target_style: target_style.clone(),
                            flash_duration: FLASH_DURATION,
                            fade_duration: FLASH_FADE_DURATION,
                        });
                        self.transition_to(new_state);
                    }
                    SegmentAction::Off => {
                        let new_state = Box::new(PoweringOffState {
                            start_time: Instant::now(),
                            from_style: self.current_style.clone(),
                            target_style: target_style.clone(),
                            duration: FADE_DURATION,
                        });
                        self.transition_to(new_state);
                    }
                    SegmentAction::BackboneUpdate => {
                        let new_state = Box::new(IdleState {
                            style: target_style.clone(),
                        });
                        self.transition_to(new_state);
                    }
                    SegmentAction::InstantStyleChange => {
                        // instantly change to target style without animations or effects
                        let new_state = Box::new(ActiveState {
                            style: target_style.clone(),
                        });
                        self.transition_to(new_state);
                    }
                }
            }
            (None, Some(target_style)) => {
                // Direct style update without action
                let new_state = Box::new(ActiveState {
                    style: target_style.clone(),
                });
                self.state = new_state;
            }
            _ => {}
        }
    }

    pub fn update_segment_style(&mut self) {
        // let the state perform its update for this frame
        if let Some(new_state) = self.state.update() {
            self.transition_to(new_state);
        }

        // update the current style
        self.current_style = self.state.calculate_style();
    }

    fn transition_to(&mut self, new_state: Box<dyn SegmentState>) {
        self.state.exit();
        self.state = new_state;
        self.state.enter();
    }

    /**************************  Transform functions *************************************** */

    pub fn apply_transform(&mut self, transform: &Transform2D) {
        for command in &mut self.draw_commands {
            command.apply_transform(transform);
        }
    }

    pub fn scale_stroke_weight(&mut self, scale_factor: f32) {
        self.current_style.stroke_weight *= scale_factor;
        self.state.scale_stroke_weight(scale_factor);
    }

    /************************ Utility Methods ****************************/

    pub fn is_background(&self) -> bool {
        matches!(self.state.layer(), Layer::Background)
    }

    pub fn is_idle(&self) -> bool {
        matches!(self.state.state_type(), SegmentStateType::Idle)
    }
}

// CachedGrid stores the pre-processed drawing commands for an entire grid
#[derive(Clone)]
pub struct CachedGrid {
    pub dimensions: (u32, u32), // number of tiles in x and y
    pub segments: HashMap<String, CachedSegment>,
    pub viewbox: ViewBox,

    // temporary segments for the stretch effect
    pub stretch_segments: HashMap<String, CachedSegment>,
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

                    segments.insert(segment.id.clone(), segment);
                }
            }
        }

        // Remove overlapping segments
        // this doesn't work, and slide effects look better without it
        // so shelving for now
        //segments = purge_overlapping_segments(segments, project.grid_x, project.grid_y);

        Self {
            dimensions: (project.grid_x, project.grid_y),
            segments,
            viewbox,
            stretch_segments: HashMap::new(),
        }
    }

    /************************ Rendering ****************************/

    // Draws the grid's current frame state
    pub fn draw(
        &mut self,
        draw: &Draw,
        update_batch: &HashMap<String, StyleUpdateMsg>,
        visible: bool,
    ) {
        let mut foreground_segments = Vec::new();
        let mut middle_segments = Vec::new();

        for segment in self.segments.values_mut() {
            // process update message
            if let Some(msg) = update_batch.get(&segment.id) {
                segment.update_segment_state(msg);
            }

            // update segment style
            segment.update_segment_style();

            // draw background layer first, or prepare other layers
            if visible {
                match segment.state.layer() {
                    Layer::Background => {
                        for command in &segment.draw_commands {
                            command.draw(draw, &segment.current_style);
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
                    command.draw(draw, &segment.current_style);
                }
            }

            for segment in foreground_segments {
                for command in &segment.draw_commands {
                    command.draw(draw, &segment.current_style);
                }
            }
        }
    }

    /************************ Transform Methods **************************/

    pub fn apply_transform(&mut self, transform: &Transform2D) {
        for segment in self.segments.values_mut() {
            segment.apply_transform(transform);
        }
    }

    pub fn scale_stroke_weights(&mut self, scale_factor: f32) {
        for segment in self.segments.values_mut() {
            segment.scale_stroke_weight(scale_factor);
        }
    }

    /************************ Utility Methods ****************************/

    // returns an iterator for the segments of a given tile.
    pub fn get_tile_segments_iter(&self, x: u32, y: u32) -> impl Iterator<Item = &CachedSegment> {
        self.segments
            .values()
            .filter(move |segment| segment.tile_coordinate == (x, y))
    }

    // returns a segment reference by ID
    pub fn segment(&self, id: &str) -> Option<&CachedSegment> {
        self.segments.get(id)
    }

    // returns the segments of a given row
    pub fn row_mut(&mut self, number: i32) -> Vec<&mut CachedSegment> {
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

    // returns the segments of a given column
    pub fn col_mut(&mut self, number: i32) -> Vec<&mut CachedSegment> {
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

    /************************ Stretch ****************************/
    pub fn add_stretch_segment(&mut self, segment: CachedSegment) {
        self.stretch_segments.insert(segment.id.clone(), segment);
    }

    pub fn remove_stretch_segment(&mut self, id: &str) {
        self.stretch_segments.remove(id);
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

// SegmentState manages the current and future styles of a segment based on what it's
// supposed to be doing at any given time
pub trait SegmentState {
    fn state_type(&self) -> SegmentStateType;
    fn enter(&self);
    fn update(&self) -> Option<Box<dyn SegmentState>>;
    fn exit(&self);
    fn layer(&self) -> Layer;
    fn calculate_style(&self) -> DrawStyle;
    fn scale_stroke_weight(&mut self, scale_factor: f32);
    fn clone_box(&self) -> Box<dyn SegmentState>;
}

#[derive(Debug, Clone)]
pub struct IdleState {
    style: DrawStyle,
}

impl SegmentState for IdleState {
    fn state_type(&self) -> SegmentStateType {
        SegmentStateType::Idle
    }

    fn enter(&self) {
        // No special entry behavior
    }

    fn update(&self) -> Option<Box<dyn SegmentState>> {
        // An idle segment doesn't need to be updated
        None
    }

    fn exit(&self) {
        // No special exit behavior
    }

    fn layer(&self) -> Layer {
        Layer::Background
    }

    fn calculate_style(&self) -> DrawStyle {
        // An idle segment doesn't need to update its style
        self.style.clone()
    }

    fn scale_stroke_weight(&mut self, scale_factor: f32) {
        self.style.stroke_weight *= scale_factor;
    }

    fn clone_box(&self) -> Box<dyn SegmentState> {
        Box::new(self.clone())
    }
}

#[derive(Debug, Clone)]
pub struct ActiveState {
    style: DrawStyle,
}

impl SegmentState for ActiveState {
    fn state_type(&self) -> SegmentStateType {
        SegmentStateType::Active
    }

    fn enter(&self) {
        // No special entry behavior
    }

    fn update(&self) -> Option<Box<dyn SegmentState>> {
        // An idle segment doesn't need to be updated
        None
    }

    fn exit(&self) {
        // No special exit behavior
    }

    fn layer(&self) -> Layer {
        Layer::Foreground
    }

    fn calculate_style(&self) -> DrawStyle {
        self.style.clone()
    }

    fn scale_stroke_weight(&mut self, scale_factor: f32) {
        self.style.stroke_weight *= scale_factor;
    }

    fn clone_box(&self) -> Box<dyn SegmentState> {
        Box::new(self.clone())
    }
}

#[derive(Debug, Clone)]
pub struct PoweringOnState {
    target_style: DrawStyle,
    start_time: Instant,
    flash_duration: f32,
    fade_duration: f32,
}

impl SegmentState for PoweringOnState {
    fn state_type(&self) -> SegmentStateType {
        SegmentStateType::PoweringOn
    }

    fn enter(&self) {
        // No particular enter behavior
    }

    fn update(&self) -> Option<Box<dyn SegmentState>> {
        let elapsed = self.start_time.elapsed().as_secs_f32();
        if elapsed >= self.flash_duration + self.fade_duration {
            // Change to active state
            Some(Box::new(ActiveState {
                style: self.target_style.clone(),
            }))
        } else {
            None
        }
    }

    fn exit(&self) {
        // No special behavior
    }

    fn layer(&self) -> Layer {
        Layer::Foreground
    }

    fn calculate_style(&self) -> DrawStyle {
        let elapsed = self.start_time.elapsed().as_secs_f32();
        if elapsed <= self.flash_duration {
            // Flash phase
            DrawStyle {
                color: rgba(1.0, 0.0, 0.0, 1.0),
                stroke_weight: self.target_style.stroke_weight,
            }
        } else {
            // Fade phase
            let fade_progress = (elapsed - self.flash_duration) / self.fade_duration;
            let flash_color = rgba(1.0, 0.0, 0.0, 1.0);

            DrawStyle {
                color: easing::color_exp_ease(
                    flash_color,
                    self.target_style.color,
                    fade_progress,
                    6.0,
                ),
                stroke_weight: self.target_style.stroke_weight,
            }
        }
    }

    fn scale_stroke_weight(&mut self, scale_factor: f32) {
        self.target_style.stroke_weight *= scale_factor;
    }

    fn clone_box(&self) -> Box<dyn SegmentState> {
        Box::new(self.clone())
    }
}

#[derive(Debug, Clone)]
pub struct PoweringOffState {
    target_style: DrawStyle,
    from_style: DrawStyle,
    start_time: Instant,
    duration: f32,
}

impl SegmentState for PoweringOffState {
    fn state_type(&self) -> SegmentStateType {
        SegmentStateType::PoweringOff
    }

    fn enter(&self) {
        // No particular enter behavior
    }

    fn update(&self) -> Option<Box<dyn SegmentState>> {
        let elapsed = self.start_time.elapsed().as_secs_f32();
        if elapsed >= self.duration {
            // Change to idle state
            Some(Box::new(IdleState {
                style: self.target_style.clone(),
            }))
        } else {
            None
        }
    }

    fn exit(&self) {
        // No special behavior
    }

    fn layer(&self) -> Layer {
        Layer::Middle
    }

    fn calculate_style(&self) -> DrawStyle {
        let elapsed = self.start_time.elapsed().as_secs_f32();
        if elapsed <= self.duration {
            // Fade phase
            let fade_progress = elapsed / self.duration;

            DrawStyle {
                color: easing::color_exp_ease(
                    self.from_style.color,
                    self.target_style.color,
                    fade_progress,
                    6.0,
                ),
                stroke_weight: self.target_style.stroke_weight,
            }
        } else {
            self.target_style.clone()
        }
    }

    fn scale_stroke_weight(&mut self, scale_factor: f32) {
        self.from_style.stroke_weight *= scale_factor;
        self.target_style.stroke_weight *= scale_factor;
    }

    fn clone_box(&self) -> Box<dyn SegmentState> {
        Box::new(self.clone())
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
