//! Lighting calculations for 3D rendering.

use crate::math::Vec3;

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
