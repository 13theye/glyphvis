/// src/render/path_render.rs
/// Static PathElement translation to Nannou Draw for rendering

use nannou::prelude::*;
use crate::services::grid_service::PathElement;
use crate::models::grid_model::ViewBox;
use super::{Transform2D, RenderParams};

pub struct PathRenderer {
    viewbox: ViewBox,
}

impl PathRenderer {
    pub fn new(viewbox: ViewBox) -> Self {
        Self { viewbox }
    }

    pub fn draw_element(
        &self,
        draw: &Draw,
        element: &PathElement,
        transform: &Transform2D,
        params: &RenderParams,
    ) {
        let center_x = self.viewbox.width / 2.0;
        let center_y = self.viewbox.height / 2.0;
        
        match element {
            PathElement::Line { x1, y1, x2, y2 } => {
                self.draw_line(draw, (*x1, *y1), (*x2, *y2), center_x, center_y, transform, params);
            },
            PathElement::Circle { cx, cy, r } => {
                self.draw_circle(draw, (*cx, *cy), *r, center_x, center_y, transform, params);
            },
            PathElement::Arc { start_x, start_y, rx, ry, x_axis_rotation, large_arc, sweep, end_x, end_y } => {
                self.draw_arc(
                    draw,
                    (*start_x, *start_y),
                    (*end_x, *end_y),
                    (*rx, *ry),
                    *x_axis_rotation,
                    *large_arc,
                    *sweep,
                    center_x,
                    center_y,
                    transform,
                    params,
                );
            }
        }
    }

    fn draw_line(
        &self,
        draw: &Draw,
        (x1, y1): (f32, f32),
        (x2, y2): (f32, f32),
        center_x: f32,
        center_y: f32,
        transform: &Transform2D,
        params: &RenderParams,
    ) {
        let start = pt2(
            transform.translation.x + (x1 - center_x) * transform.scale,
            transform.translation.y + (y1 - center_y) * transform.scale
        );
        let end = pt2(
            transform.translation.x + (x2 - center_x) * transform.scale,
            transform.translation.y + (y2 - center_y) * transform.scale
        );

        draw.line()
            .start(start)
            .end(end)
            .color(params.color)
            .stroke_weight(params.stroke_weight);
    }

    fn draw_circle(
        &self,
        draw: &Draw,
        (cx, cy): (f32, f32),
        r: f32,
        center_x: f32,
        center_y: f32,
        transform: &Transform2D,
        params: &RenderParams,
    ) {
        let center = pt2(
            transform.translation.x + (cx - center_x) * transform.scale,
            transform.translation.y + (cy - center_y) * transform.scale
        );

        draw.ellipse()
            .x_y(center.x, center.y)
            .radius(r * transform.scale)
            .stroke(params.color)
            .stroke_weight(params.stroke_weight)
            .color(params.color);
    }

    fn draw_arc(
        &self,
        draw: &Draw,
        (start_x, start_y): (f32, f32),
        (end_x, end_y): (f32, f32),
        (rx, ry): (f32, f32),
        x_axis_rotation: f32,
        large_arc: bool,
        sweep: bool,
        center_x: f32,
        center_y: f32,
        transform: &Transform2D,
        params: &RenderParams,
    ) {
        /*
        // Debug inputs
        println!("\nArc path at position: {:?}", transform.translation);
        println!("Input params:");
        println!("  Start: ({}, {}), End: ({}, {})", start_x, start_y, end_x, end_y);
        println!("  rx: {}, ry: {}, rotation: {}", rx, ry, x_axis_rotation);
        println!("  large_arc: {}, sweep: {}", large_arc, sweep);
        */
    
        // Convert coordinates to screen space
        let screen_start = pt2(
            transform.translation.x + (start_x - center_x) * transform.scale,
            transform.translation.y + (start_y - center_y) * transform.scale
        );
        let screen_end = pt2(
            transform.translation.x + (end_x - center_x) * transform.scale,
            transform.translation.y + (end_y - center_y) * transform.scale
        );
        
        /*
        println!("Screen coordinates:");
        println!("  Start: ({:.2}, {:.2})", screen_start.x, screen_start.y);
        println!("  End: ({:.2}, {:.2})", screen_end.x, screen_end.y);
        */
        // SVG to center parameterization conversion
        // Step 1: Transform to origin
        let x1p = (screen_start.x - screen_end.x) / 2.0;
        let y1p = (screen_start.y - screen_end.y) / 2.0;
    
        //println!("Step 1 - Transform to origin:");
        //println!("  x1p: {:.2}, y1p: {:.2}", x1p, y1p);
    
        // Rotate to align with coordinate axes
        let angle_rad = x_axis_rotation.to_radians();
        let cos_angle = angle_rad.cos();
        let sin_angle = angle_rad.sin();
    
        let xp = cos_angle * x1p + sin_angle * y1p;
        let yp = -sin_angle * x1p + cos_angle * y1p;
    
        //println!("After rotation:");
        //println!("  xp: {:.2}, yp: {:.2}", xp, yp);
    
        // Step 2: Compute center
        let rx_scaled = rx * transform.scale;
        let ry_scaled = ry * transform.scale;
        let rx_sq = rx_scaled * rx_scaled;
        let ry_sq = ry_scaled * ry_scaled;
        let xp_sq = xp * xp;
        let yp_sq = yp * yp;
    
        // Ensure radii are large enough
        let radii_scale = xp_sq / rx_sq + yp_sq / ry_sq;
        let (rx_final, ry_final) = if radii_scale > 1.0 {
            let sqrt_scale = radii_scale.sqrt();
            //println!("Scaling up radii by factor: {:.2}", sqrt_scale);
            (rx_scaled * sqrt_scale, ry_scaled * sqrt_scale)
        } else {
            (rx_scaled, ry_scaled)
        };
    
        //println!("Final radii:");
        //println!("  rx: {:.2}, ry: {:.2}", rx_final, ry_final);
    
        let rx_sq = rx_final * rx_final;
        let ry_sq = ry_final * ry_final;
    
        let term = (rx_sq * ry_sq - rx_sq * yp_sq - ry_sq * xp_sq) / 
                   (rx_sq * yp_sq + ry_sq * xp_sq);
        /*
        println!("Center calculation:");
        println!("  Term under sqrt: {:.2}", term);
     */
        // Guard against numerical errors that might make term slightly negative
        let s = if term <= 0.0 { 
            println!("  Warning: Non-positive term, using s = 0");
            0.0 
        } else { 
            term.sqrt() 
        };
        let cp = if large_arc == sweep {-s} else {s};
    
        let cxp = cp * rx_final * yp / ry_final;
        let cyp = -cp * ry_final * xp / rx_final;
    
        // Step 3: Transform back
        let center_x = cos_angle * cxp - sin_angle * cyp + (screen_start.x + screen_end.x) / 2.0;
        let center_y = sin_angle * cxp + cos_angle * cyp + (screen_start.y + screen_end.y) / 2.0;
        /*
        println!("Calculated center: ({:.2}, {:.2})", center_x, center_y);
        */
        // Calculate angles
        let start_angle = ((yp - cyp) / ry_final).atan2((xp - cxp) / rx_final);
        let end_angle = ((-yp - cyp) / ry_final).atan2((-xp - cxp) / rx_final);
        
        /* 
        println!("Angles:");
        println!("  Start: {:.2}, End: {:.2}", start_angle, end_angle);
        */
        // Generate points
        let resolution = 64;
        let mut points = Vec::with_capacity(resolution + 1);
        
        // Calculate total angle sweep
        let mut delta_angle = end_angle - start_angle;
        
        // Ensure we're sweeping in the correct direction
        if sweep {
            if delta_angle < 0.0 {
                delta_angle += 2.0 * std::f32::consts::PI;
            }
        } else {
            if delta_angle > 0.0 {
                delta_angle -= 2.0 * std::f32::consts::PI;
            }
        }
    
        for i in 0..=resolution {
            let t = i as f32 / resolution as f32;
            let angle = start_angle + t * delta_angle;
    
            let x = center_x + rx_final * (cos_angle * angle.cos() - sin_angle * angle.sin());
            let y = center_y + ry_final * (sin_angle * angle.cos() + cos_angle * angle.sin());
    
            points.push(pt2(x, y));
        }
     /* 
        println!("Generated {} points", points.len());
        if let Some(first) = points.first() {
            println!("First point: ({:.2}, {:.2})", first.x, first.y);
        }
        if let Some(last) = points.last() {
            println!("Last point: ({:.2}, {:.2})", last.x, last.y);
        }
        */
    
        // Build and draw the path
        if let Some(first) = points.first() {
            let mut builder = nannou::geom::Path::builder()
                .move_to(nannou::geom::Point2::new(first.x, first.y));
            
            for point in points.iter().skip(1) {
                builder = builder.line_to(nannou::geom::Point2::new(point.x, point.y));
            }
    
            let path = builder.build();
            draw.path()
                .stroke()
                .weight(params.stroke_weight)
                .color(params.color)
                .events(path.iter());
        }
    }
}