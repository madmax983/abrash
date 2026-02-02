use crate::math::Vec3;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ScreenPoint {
    pub x: i32,
    pub y: i32,
    pub z: f32,
}

/// Project a 3D point to screen coordinates
pub fn project_to_screen(v: Vec3, w: f32, width: u32, height: u32) -> ScreenPoint {
    // Perspective divide
    let inv_w = if w.abs() > 0.0001 { 1.0 / w } else { 1.0 };
    let ndc_x = v.x * inv_w;
    let ndc_y = v.y * inv_w;
    let depth = v.z * inv_w;

    // NDC to screen coordinates
    let screen_x = ((ndc_x + 1.0) * 0.5 * width as f32) as i32;
    let screen_y = ((1.0 - ndc_y) * 0.5 * height as f32) as i32; // Flip Y

    ScreenPoint {
        x: screen_x,
        y: screen_y,
        z: depth,
    }
}
