/// src/draw/path_draw.rs
/// PathElement translation to Nannou Draw for drawing

use nannou::prelude::*;
use crate::services::path_service::PathElement;
use crate::models::grid_model::ViewBox;
use super::{Transform2D, DrawParams};

use nannou::lyon::tessellation::LineCap;

use std::f32::consts::PI;

pub fn draw_element(
    draw: &Draw,
    element: &PathElement,
    transform: &Transform2D,
    params: &DrawParams,
    viewbox: &ViewBox,
) {
    
    match element {
        PathElement::Line { x1, y1, x2, y2 } => {
            draw_line(draw, (*x1, *y1), (*x2, *y2), viewbox, transform, params);
        },
        PathElement::Circle { cx, cy, r } => {
            draw_circle(draw, (*cx, *cy), *r, viewbox, transform, params);
        },
        PathElement::Arc { start_x, start_y, rx, ry, x_axis_rotation, large_arc, sweep, end_x, end_y } => {
            draw_arc(
                draw,
                (*start_x, *start_y),
                (*end_x, *end_y),
                (*rx, *ry),
                *x_axis_rotation,
                *large_arc,
                *sweep,
                viewbox,
                transform,
                params,
            );
        }
    }
}

// Method to transform from origin at top left to origin at center and apply transform
fn transform_point(
    svg_x: f32,
    svg_y: f32,
    center_x: f32,
    center_y: f32,
    transform: &Transform2D,
) -> Point2 {

    // 1. Translate from SVG coordinates (top-left origin) to local coordinates
    let local_x = svg_x - center_x;
    let local_y = center_y - svg_y; // invert y to match nannou
    
    // 2. Apply scale
    let scaled_x = local_x * transform.scale;
    let scaled_y = local_y * transform.scale;
    
    // 3. Apply rotation
    let cos_rot = transform.rotation.cos();
    let sin_rot = transform.rotation.sin();
    let rotated_x = (scaled_x * cos_rot) - (scaled_y * sin_rot);
    let rotated_y = (scaled_x * sin_rot) + (scaled_y * cos_rot);
    
    // 4. Apply final translation to Nannou coordinates
    pt2(
        transform.translation.x + rotated_x,
        transform.translation.y + rotated_y
    )
}



fn draw_line(
    draw: &Draw,
    (x1, y1): (f32, f32),
    (x2, y2): (f32, f32),
    viewbox: &ViewBox,
    transform: &Transform2D,
    params: &DrawParams,
) {
    let center_x = viewbox.width / 2.0;
    let center_y = viewbox.height / 2.0;

    let start = transform_point(x1, y1, center_x, center_y, transform);
    let end = transform_point(x2, y2, center_x, center_y, transform);

    draw.line()
        .points(start, end)
        .color(params.color)
        .stroke_weight(params.stroke_weight)
        .caps(LineCap::Round);
}

fn draw_circle(
    draw: &Draw,
    (cx, cy): (f32, f32),
    r: f32,
    viewbox: &ViewBox,
    transform: &Transform2D,
    params: &DrawParams,
) {
    let center_x = viewbox.width / 2.0;
    let center_y = viewbox.height / 2.0;
    let center = transform_point(cx, cy, center_x, center_y, transform);

    draw.ellipse()
        .x_y(center.x, center.y)
        .radius(r * transform.scale)
        .stroke(params.color)
        .stroke_weight(params.stroke_weight)
        .color(params.color)
        .caps(LineCap::Round);
}

fn draw_arc(
    draw: &Draw,
    (start_x, start_y): (f32, f32),
    (end_x, end_y): (f32, f32),
    (rx, ry): (f32, f32),
    x_axis_rotation: f32,
    large_arc: bool,
    sweep: bool,
    viewbox: &ViewBox,
    transform: &Transform2D,
    params: &DrawParams,
) {
    let debug_flag = false;

    if debug_flag {
        println!("\n=== Arc Debug ===");
        println!("Input parameters:");
        println!("  start: ({}, {})", start_x, start_y);
        println!("  end: ({}, {})", end_x, end_y);
        println!("  rx, ry: {}, {}", rx, ry);
        println!("  x_axis_rotation: {}", x_axis_rotation);
        println!("  large_arc: {}", large_arc);
        println!("  sweep: {}", sweep);
    }

    let center_x = viewbox.width / 2.0;
    let center_y = viewbox.height / 2.0;

    // Convert coordinates to screen space
    let screen_start = transform_point(start_x, start_y, center_x, center_y, transform);
    let screen_end = transform_point(end_x, end_y, center_x, center_y, transform);

    if debug_flag {
        println!("\nTransformed points:");
        println!("  screen_start: ({:.2}, {:.2})", screen_start.x, screen_start.y);
        println!("  screen_end: ({:.2}, {:.2})", screen_end.x, screen_end.y);
    }
    
    // Scale radii
    let rx_scaled = rx * transform.scale;
    let ry_scaled = ry * transform.scale;
    
    // Calculate center and angles using the geometric method
    let (center, start_angle, sweep_angle) = calculate_arc_center(
        screen_start,
        screen_end,
        rx_scaled,
        ry_scaled,
        x_axis_rotation,
        large_arc,
        sweep
    );

    // Generate points along the arc
    let resolution = 128;
    let mut points = Vec::with_capacity(resolution + 1);
    
    for i in 0..=resolution {
        let t = i as f32 / resolution as f32;
        let angle = start_angle + t * sweep_angle;
        
        // Calculate point with proper radii and rotation
        let x = center.x + rx_scaled * (angle.cos() * x_axis_rotation.to_radians().cos() - 
                                        angle.sin() * x_axis_rotation.to_radians().sin());
        let y = center.y + ry_scaled * (angle.cos() * x_axis_rotation.to_radians().sin() + 
                                        angle.sin() * x_axis_rotation.to_radians().cos());
        
        points.push(pt2(x, y));
    }

    // Draw the arc segments as individual lines with proper stroke weight
    for window in points.windows(2) {
        if let [p1, p2] = window {
            draw.line()
                .start(*p1)
                .end(*p2)
                .stroke_weight(params.stroke_weight)
                .color(params.color);
        }
    }
}

fn calculate_arc_center(
    start: Point2,
    end: Point2,
    rx: f32,
    ry: f32,
    x_axis_rotation: f32,
    large_arc: bool,
    sweep: bool
) -> (Point2, f32, f32) {  // Returns (center, start_angle, sweep_angle)
    let debug_flag = false;
    if debug_flag {println!("\nCenter calculation:");}

    // Step 1: Transform to origin and unrotated coordinates
    let dx = (start.x - end.x) / 2.0;
    let dy = (start.y - end.y) / 2.0;

    if debug_flag {println!("  dx, dy: {:.2}, {:.2}", dx, dy);}
    
    let angle_rad = x_axis_rotation.to_radians();
    let cos_phi = angle_rad.cos();
    let sin_phi = angle_rad.sin();
    
    // Rotate to align with axes
    let x1p = cos_phi * dx + sin_phi * dy;
    let y1p = -sin_phi * dx + cos_phi * dy;

    if debug_flag {println!("  x1p, y1p: {:.2}, {:.2}", x1p, y1p);}

    
    // Step 2: Ensure radii are large enough
    let rx_sq = rx * rx;
    let ry_sq = ry * ry;
    let x1p_sq = x1p * x1p;
    let y1p_sq = y1p * y1p;
    
    let radii_check = x1p_sq / rx_sq + y1p_sq / ry_sq;
    let (rx_final, ry_final) = if radii_check > 1.0 {
        let sqrt_scale = radii_check.sqrt();
        (rx * sqrt_scale, ry * sqrt_scale)
    } else {
        (rx, ry)
    };
    
    // Step 3: Calculate center parameters
    let rx_sq = rx_final * rx_final;
    let ry_sq = ry_final * ry_final;
    
    let term = (rx_sq * ry_sq - rx_sq * y1p_sq - ry_sq * x1p_sq) / 
                (rx_sq * y1p_sq + ry_sq * x1p_sq);
                
    let s = if term <= 0.0 { 0.0 } else { term.sqrt() };

    if debug_flag {
        println!("  term: {:.2}", term);
        println!("  s: {:.2}", s);
    }
    
    // Choose center based on sweep and large-arc flags
    let cxp = s * rx_final * y1p / ry_final;
    let cyp = -s * ry_final * x1p / rx_final;

    if debug_flag{println!("  cxp, cyp before flip: {:.2}, {:.2}", cxp, cyp);}
    
    // Handle sweep flag to make it clockwise by flipping the center.
    let (cxp, cyp) = if sweep {
        (-cxp, -cyp)
    } else {
        (cxp, cyp)
    };

    if debug_flag{println!("  cxp, cyp after sweep: {:.2}, {:.2}", cxp, cyp);}

    // Step 4: Transform center back to original coordinate space
    let cx = cos_phi * cxp - sin_phi * cyp + (start.x + end.x) / 2.0;
    let cy = sin_phi * cxp + cos_phi * cyp + (start.y + end.y) / 2.0;

    if debug_flag {println!("  final center: ({:.2}, {:.2})", cx, cy);}
    
    // Step 5: Calculate angles
    let start_vec_x = (x1p - cxp) / rx_final;
    let start_vec_y = (y1p - cyp) / ry_final;
    let end_vec_x = (-x1p - cxp) / rx_final;
    let end_vec_y = (-y1p - cyp) / ry_final;
    
    let start_angle = (start_vec_y).atan2(start_vec_x);
    let mut sweep_angle = (end_vec_y).atan2(end_vec_x) - start_angle;

    if debug_flag {
        println!("  start_angle: {:.2}° ({:.2} rad)", start_angle.to_degrees(), start_angle);
        println!("  sweep_angle: {:.2}° ({:.2} rad)", sweep_angle.to_degrees(), sweep_angle);
    }
    
    // Ensure sweep angle matches flags
    if !sweep && sweep_angle > 0.0 {
        sweep_angle -= 2.0 * PI;
    } else if sweep && sweep_angle < 0.0 {
        sweep_angle += 2.0 * PI;
    }
    
    // Force the short path for !large_arc
    if !large_arc && sweep_angle.abs() > PI {
        sweep_angle = if sweep_angle > 0.0 {
            sweep_angle - 2.0 * PI
        } else {
            sweep_angle + 2.0 * PI
        };
    }
    (pt2(cx, cy), start_angle, sweep_angle)
}


#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_coordinate_transform() {
        /*/
        let viewbox = ViewBox {
            min_x: 0.0,
            min_y: 0.0,
            width: 100.0,
            height: 100.0,
        };*/
        
        // Test case 1: No translation or rotation, only centering
        let transform = Transform2D {
            translation: Vec2::new(0.0, 0.0),
            scale: 1.0,
            rotation: 0.0,
        };
        
        // Point at SVG (0,0) should move to (-50,50) in Nannou space
        let point = transform_point(0.0, 0.0, 50.0, 50.0, &transform);
        assert_eq!(point.x, -50.0);
        assert_eq!(point.y, 50.0);
        
        // Point at SVG (100,100) should move to (50,-50) in Nannou space
        let point = transform_point(100.0, 100.0, 50.0, 50.0, &transform);
        assert_eq!(point.x, 50.0);
        assert_eq!(point.y, -50.0);
        
        // Test case 2: With translation
        let transform = Transform2D {
            translation: Vec2::new(100.0, 100.0),
            scale: 1.0,
            rotation: 0.0,
        };
        
        let point = transform_point(0.0, 0.0, 50.0, 50.0, &transform);
        assert_eq!(point.x, 50.0); // -50 + 100
        assert_eq!(point.y, 150.0); // 50 + 100
        
        // Test case 3: With scaling
        let transform = Transform2D {
            translation: Vec2::new(0.0, 0.0),
            scale: 2.0,
            rotation: 0.0,
        };
        
        let point = transform_point(0.0, 0.0, 50.0, 50.0, &transform);
        assert_eq!(point.x, -100.0); // (-50) * 2
        assert_eq!(point.y, 100.0); // (50) * 2
        
        // Test case 4: With rotation
        let transform = Transform2D {
            translation: Vec2::new(0.0, 0.0),
            scale: 1.0,
            rotation: PI / 2.0, // 90 degrees
        };
        
        let point = transform_point(0.0, 0.0, 50.0, 50.0, &transform);
        let difx = (point.x - -50.0).abs();
        let dify = (point.y - -50.0).abs();
        assert!(difx < 0.001);
        assert!(dify < 0.001);
        
        // Test case 5: Combined transformation
        let transform = Transform2D {
            translation: Vec2::new(100.0, 100.0),
            scale: 2.0,
            rotation: PI / 4.0, // 45 degrees
        };
        
        let point = transform_point(0.0, 0.0, 50.0, 50.0, &transform);
        // For a 45-degree rotation of point (-50, 50) scaled by 2:
   
        let expected_x = (-100.0 * (PI/4.0).cos() - 100.0 * (PI/4.0).sin()) + 100.0;
        let expected_y = (-100.0 * (PI/4.0).sin() + 100.0 * (PI/4.0).cos()) + 100.0;
        println!("Expected: ({:.2}, {:.2})", expected_x, expected_y);
        println!("Actual: ({:.2}, {:.2})", point.x, point.y);
        assert!((point.x - expected_x).abs() < 0.001);
        assert!((point.y - expected_y).abs() < 0.001);
    }
}