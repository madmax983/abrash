//! Lighting calculations for 3D rendering.

use crate::math::Vec3;

/// Directional light (like sunlight)
#[derive(Debug, Clone, Copy)]
pub struct DirectionalLight {
    /// Direction the light travels (normalized)
    pub direction: Vec3,
    /// Light color (RGB, 0.0-1.0)
    pub color: Vec3,
}

impl DirectionalLight {
    pub fn new(direction: Vec3, color: Vec3) -> Self {
        Self {
            direction: direction.normalize(),
            color,
        }
    }

    /// Calculate light intensity on a surface with given normal
    /// Returns 0.0-1.0 based on Lambert's cosine law
    pub fn intensity(&self, normal: Vec3) -> f32 {
        // N dot L (light direction is inverted because it points AT the surface)
        let n_dot_l = normal.dot(self.direction * -1.0);
        n_dot_l.max(0.0)
    }

    /// Calculate lit color for a surface
    pub fn shade(&self, normal: Vec3, base_color: Vec3) -> Vec3 {
        let i = self.intensity(normal);
        Vec3::new(
            base_color.x * self.color.x * i,
            base_color.y * self.color.y * i,
            base_color.z * self.color.z * i,
        )
    }
}

/// Convert Vec3 color (0.0-1.0 per channel) to u32 ARGB
pub fn color_to_u32(color: Vec3) -> u32 {
    let r = (color.x.clamp(0.0, 1.0) * 255.0) as u32;
    let g = (color.y.clamp(0.0, 1.0) * 255.0) as u32;
    let b = (color.z.clamp(0.0, 1.0) * 255.0) as u32;
    0xFF000000 | (r << 16) | (g << 8) | b
}

/// Convert u32 ARGB to Vec3 color (0.0-1.0 per channel)
pub fn u32_to_color(argb: u32) -> Vec3 {
    let r = ((argb >> 16) & 0xFF) as f32 / 255.0;
    let g = ((argb >> 8) & 0xFF) as f32 / 255.0;
    let b = (argb & 0xFF) as f32 / 255.0;
    Vec3::new(r, g, b)
}
