//! Procedural Lightning Post-Processing Filter
//!
//! Simulates electric discharges (lightning/tesla coils) using a randomized branching path algorithm.

#![cfg(feature = "nova")]

use crate::framebuffer::Framebuffer;
use abrash_core::utils::XorShift32;

/// Configuration for the Lightning effect.
#[derive(Debug, Clone, Copy)]
pub struct LightningConfig {
    /// Normalized start X coordinate (0.0 to 1.0).
    pub start_x: f32,
    /// Normalized start Y coordinate (0.0 to 1.0).
    pub start_y: f32,
    /// Normalized end X coordinate (0.0 to 1.0).
    pub end_x: f32,
    /// Normalized end Y coordinate (0.0 to 1.0).
    pub end_y: f32,
    /// The base color of the lightning.
    pub color: u32,
    /// The maximum number of expected branches per main segment.
    pub branches: f32,
    /// The recursive depth for line splitting.
    pub max_depth: u32,
    /// How much the bolt zigzags (jaggedness).
    pub jag_factor: f32,
    /// Random seed for bolt generation.
    pub seed: u32,
}

impl Default for LightningConfig {
    fn default() -> Self {
        Self {
            start_x: 0.5,
            start_y: 0.0,
            end_x: 0.5,
            end_y: 1.0,
            color: 0xFF_88_AA_FF, // Light blueish
            branches: 1.0,
            max_depth: 6,
            jag_factor: 0.3,
            seed: 42,
        }
    }
}

// Additive blend function for u32 colors
fn blend_add(src: u32, dst: u32) -> u32 {
    let sr = (src >> 16) & 0xFF;
    let sg = (src >> 8) & 0xFF;
    let sb = src & 0xFF;

    let dr = (dst >> 16) & 0xFF;
    let dg = (dst >> 8) & 0xFF;
    let db = dst & 0xFF;

    let r = (sr + dr).min(255);
    let g = (sg + dg).min(255);
    let b = (sb + db).min(255);

    0xFF_00_00_00 | (r << 16) | (g << 8) | b
}

fn draw_line_add(fb: &mut Framebuffer, x0: i32, y0: i32, x1: i32, y1: i32, color: u32) {
    let dx = (x1 - x0).abs();
    let sx = if x0 < x1 { 1 } else { -1 };
    let dy = -(y1 - y0).abs();
    let sy = if y0 < y1 { 1 } else { -1 };
    let mut err = dx + dy;

    let mut cx = x0;
    let mut cy = y0;
    let width = fb.width() as i32;
    let height = fb.height() as i32;

    loop {
        if cx >= 0 && cx < width && cy >= 0 && cy < height {
            let idx = (cy * width + cx) as usize;
            let current = fb.as_slice()[idx];
            fb.as_mut_slice()[idx] = blend_add(color, current);
        }

        if cx == x1 && cy == y1 {
            break;
        }

        let e2 = 2 * err;
        if e2 >= dy {
            err += dy;
            cx += sx;
        }
        if e2 <= dx {
            err += dx;
            cy += sy;
        }
    }
}

fn generate_bolt(
    fb: &mut Framebuffer,
    rng: &mut XorShift32,
    x0: f32,
    y0: f32,
    x1: f32,
    y1: f32,
    color: u32,
    depth: u32,
    config: &LightningConfig,
) {
    if depth == 0 {
        draw_line_add(fb, x0 as i32, y0 as i32, x1 as i32, y1 as i32, color);
        return;
    }

    // Find midpoint
    let mut mx = x0 + (x1 - x0) * 0.5;
    let mut my = y0 + (y1 - y0) * 0.5;

    // Displace midpoint normal to the line segment
    let nx = -(y1 - y0);
    let ny = x1 - x0;
    let len = nx.hypot(ny);

    if len > 0.0 {
        let displacement = (rng.next_f32() - 0.5) * config.jag_factor * len;
        mx += (nx / len) * displacement;
        my += (ny / len) * displacement;
    }

    // Draw main segments recursively
    generate_bolt(fb, rng, x0, y0, mx, my, color, depth - 1, config);
    generate_bolt(fb, rng, mx, my, x1, y1, color, depth - 1, config);

    // Random chance to branch
    let chance = config.branches / ((config.max_depth - depth + 1) as f32);
    if rng.next_f32() < chance {
        let branch_dx = (rng.next_f32() - 0.5) * len * 0.8;
        let branch_dy = (rng.next_f32() - 0.5) * len * 0.8;

        let bx = mx + branch_dx;
        let by = my + branch_dy;

        // Darker branch color
        let sr = (color >> 16) & 0xFF;
        let sg = (color >> 8) & 0xFF;
        let sb = color & 0xFF;
        let dim_color = 0xFF_00_00_00 | ((sr / 2) << 16) | ((sg / 2) << 8) | (sb / 2);

        generate_bolt(fb, rng, mx, my, bx, by, dim_color, depth - 1, config);
    }
}

/// Applies a procedural lightning effect to the framebuffer.
///
/// Simulates electric discharges by rendering a recursive fractal branching line structure.
pub fn apply_lightning(fb: &mut Framebuffer, config: &LightningConfig) {
    let width = fb.width() as f32;
    let height = fb.height() as f32;

    let sx = config.start_x * width;
    let sy = config.start_y * height;
    let ex = config.end_x * width;
    let ey = config.end_y * height;

    let mut rng = XorShift32::new(config.seed);

    generate_bolt(
        fb,
        &mut rng,
        sx,
        sy,
        ex,
        ey,
        config.color,
        config.max_depth,
        config,
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_apply_lightning_modifies_buffer() {
        let width = 100;
        let height = 100;
        let mut fb = Framebuffer::new(width, height).unwrap();
        fb.clear(0xFF_00_00_00);

        let config = LightningConfig::default();
        apply_lightning(&mut fb, &config);

        let mut changed = false;
        for &pixel in fb.as_slice() {
            if pixel != 0xFF_00_00_00 {
                changed = true;
                break;
            }
        }
        assert!(changed, "Lightning effect should draw onto the framebuffer");
    }
}
