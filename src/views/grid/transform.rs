use nannou::prelude::*;
use std::f32::consts::PI;

#[derive(Debug, Clone)]
pub struct Transform2D {
    pub translation: Vec2,
    pub scale: f32,
    pub rotation: f32,
}

impl Default for Transform2D {
    fn default() -> Self {
        Self {
            translation: Vec2::ZERO,
            scale: 1.0,
            rotation: 0.0,
        }
    }
}

impl Transform2D {
    // new function to combine two transforms
    pub fn combine(&self, other: &Transform2D) -> Transform2D {
        Transform2D {
            translation: self.translation + other.translation,
            scale: self.scale * other.scale,
            rotation: self.rotation + other.rotation,
        }
    }

    // new function to directly transform a point
    pub fn apply_to_point(&self, point: Point2) -> Point2 {
        // 1. Scale
        let scaled = point * self.scale;

        // 2. Rotate
        let rotation = self.rotation * PI / 180.0;
        let cos_rot = rotation.cos();
        let sin_rot = rotation.sin();
        let rotated = pt2(
            scaled.x * cos_rot - scaled.y * sin_rot,
            scaled.x * sin_rot + scaled.y * cos_rot,
        );

        // 3. Translate
        rotated + self.translation
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use std::f32::consts::PI;
    
    #[test]
    fn test_default_transform() {
        let transform = Transform2D::default();
        assert_eq!(transform.translation, Vec2::ZERO);
        assert_eq!(transform.scale, 1.0);
        assert_eq!(transform.rotation, 0.0);
    }

    #[test]
    fn test_combine_transforms() {
        let t1 = Transform2D {
            translation: Vec2::new(1.0, 2.0),
            scale: 2.0,
            rotation: PI / 4.0,
        };

        let t2 = Transform2D {
            translation: Vec2::new(3.0, 4.0),
            scale: 3.0,
            rotation: PI / 2.0,
        };

        let combined = t1.combine(&t2);
        assert_eq!(combined.translation, Vec2::new(4.0, 6.0));
        assert_eq!(combined.scale, 6.0);
        assert_eq!(combined.rotation, 3.0 * PI / 4.0);
    }

    #[test]
    fn test_point_transformation() {
        // Test translation only
        let transform = Transform2D {
            translation: Vec2::new(1.0, 1.0),
            scale: 1.0,
            rotation: 0.0,
        };
        let point = pt2(1.0, 1.0);
        let transformed = transform.apply_to_point(point);
        assert!((transformed.x - 2.0).abs() < 1e-6);
        assert!((transformed.y - 2.0).abs() < 1e-6);

        // Test scale only
        let transform = Transform2D {
            translation: Vec2::ZERO,
            scale: 2.0,
            rotation: 0.0,
        };
        let transformed = transform.apply_to_point(point);
        assert!((transformed.x - 2.0).abs() < 1e-6);
        assert!((transformed.y - 2.0).abs() < 1e-6);

        // Test rotation only (90 degrees)
        let transform = Transform2D {
            translation: Vec2::ZERO,
            scale: 1.0,
            rotation: PI / 2.0,
        };
        let transformed = transform.apply_to_point(point);
        assert!((transformed.x - -1.0).abs() < 1e-6);
        assert!((transformed.y - 1.0).abs() < 1e-6);

        // Test combined transformation
        let transform = Transform2D {
            translation: Vec2::new(1.0, 1.0),
            scale: 2.0,
            rotation: PI / 2.0,
        };
        let transformed = transform.apply_to_point(point);
        assert!((transformed.x - -1.0).abs() < 1e-6);
        assert!((transformed.y - 3.0).abs() < 1e-6);
    }
}