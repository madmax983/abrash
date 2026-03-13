//! Lens flare rendering.
//!
//! Provides screen-space lens flare artifacts based on bright light sources.

use crate::framebuffer::Framebuffer;
use crate::math::Vec2;

#[cfg(feature = "parallel")]
use rayon::prelude::*;

/// Represents a single lens flare ghost artifact.
#[derive(Clone, Debug, PartialEq)]
pub struct FlareGhost {
    pub offset_scale: f32,
    pub radius: f32,
    pub color: u32,
}

/// Configuration for the lens flare generator.
#[derive(Clone, Debug, PartialEq)]
pub struct LensFlareConfig {
    pub ghosts: Vec<FlareGhost>,
    pub halo_radius: f32,
    pub halo_thickness: f32,
    pub halo_color: u32,
}

impl Default for LensFlareConfig {
    fn default() -> Self {
        Self {
            ghosts: vec![
                FlareGhost {
                    offset_scale: 0.2,
                    radius: 40.0,
                    color: 0x44FF_AA33,
                },
                FlareGhost {
                    offset_scale: -0.3,
                    radius: 25.0,
                    color: 0x4433_FF55,
                },
                FlareGhost {
                    offset_scale: -0.5,
                    radius: 70.0,
                    color: 0x2233_AAFF,
                },
                FlareGhost {
                    offset_scale: -0.9,
                    radius: 15.0,
                    color: 0x66FF_3333,
                },
                FlareGhost {
                    offset_scale: 1.0,
                    radius: 120.0,
                    color: 0x11FF_FFFF,
                },
            ],
            halo_radius: 150.0,
            halo_thickness: 10.0,
            halo_color: 0x22FF_FFFF,
        }
    }
}

/// Helper function to perform additive blending of two colors.
/// Uses pure integer arithmetic. Intensity is expected to be 0-256 (where 256 is 1.0).

#[inline(always)]
fn add_blend_int(dest: u32, src: u32, intensity: u32) -> u32 {
    // Bolt SWAR Optimization
    let rb_mask = 0x00FF00FF;
    let ag_mask = 0xFF00FF00;

    let dest_rb = dest & rb_mask;
    let dest_ag = (dest & ag_mask) >> 8;

    let src_rb = src & rb_mask;
    let src_ag = (src & ag_mask) >> 8;

    // Multiply by intensity (0-256)
    let src_rb = ((src_rb * intensity) >> 8) & rb_mask;
    let src_ag = ((src_ag * intensity) >> 8) & rb_mask; // Using rb_mask works for both because it's 0x00FF00FF and they are shifted to align

    // Add and mask to prevent overflow into neighbor channels
    let mut out_rb = dest_rb + src_rb;
    let mut out_ag = dest_ag + src_ag;

    // Clamp values using standard min logic to prevent overflow artifacts
    let a_out = (out_ag >> 16).min(255);
    let r_out = (out_rb >> 16).min(255);
    let g_out = (out_ag & 0xFFFF).min(255);
    let b_out = (out_rb & 0xFFFF).min(255);

    (a_out << 24) | (r_out << 16) | (g_out << 8) | b_out
}


struct RenderableGhost {
    cx: f32,
    cy: f32,
    r: f32,
    r_sq: f32,
    color: u32,
    min_y: i32,
    max_y: i32,
    min_x: i32,
    max_x: i32,
}

/// Bolt Performance Optimization:
///
/// Replaced `.chunks_mut(width)` with `.chunks_exact_mut(width)` to eliminate
/// remainder chunk handling and bounds checking, enabling better vectorization
/// and measurable performance improvements.

/// Bolt Performance Optimization:
///
/// Replaced `.chunks_mut(width)` with `.chunks_exact_mut(width)` to eliminate
/// remainder chunk handling and bounds checking, enabling better vectorization
/// and measurable performance improvements.
pub fn apply_lens_flare(fb: &mut Framebuffer, light_pos: Vec2, config: &LensFlareConfig) {
    let width = fb.width() as i32;
    let height = fb.height() as i32;
    let cx = width as f32 * 0.5;
    let cy = height as f32 * 0.5;

    let flare_vec = light_pos - Vec2::new(cx, cy);

    // Pre-calculate ghost properties and bounding boxes
    let mut renderables = Vec::with_capacity(config.ghosts.len());
    for ghost in &config.ghosts {
        let ghost_cx = cx + flare_vec.x * ghost.offset_scale;
        let ghost_cy = cy + flare_vec.y * ghost.offset_scale;
        let r_i32 = ghost.radius.ceil() as i32;

        renderables.push(RenderableGhost {
            cx: ghost_cx,
            cy: ghost_cy,
            r: ghost.radius,
            r_sq: ghost.radius * ghost.radius,
            color: ghost.color,
            min_y: (ghost_cy as i32 - r_i32).max(0),
            max_y: (ghost_cy as i32 + r_i32).min(height - 1),
            min_x: (ghost_cx as i32 - r_i32).max(0),
            max_x: (ghost_cx as i32 + r_i32).min(width - 1),
        });
    }

    // Pre-calculate halo properties
    let mut has_halo = false;
    let mut halo_cx = 0.0;
    let mut halo_cy = 0.0;
    let mut halo_min_y = 0;
    let mut halo_max_y = 0;
    let mut halo_min_x = 0;
    let mut halo_max_x = 0;
    let mut halo_max_r_sq = 0.0;
    let mut halo_min_r_sq = 0.0;

    if config.halo_radius > 0.0 && config.halo_thickness > 0.0 {
        has_halo = true;
        halo_cx = cx - flare_vec.x * 0.5;
        halo_cy = cy - flare_vec.y * 0.5;

        let max_r = config.halo_radius + config.halo_thickness;
        let min_r = config.halo_radius - config.halo_thickness;
        halo_max_r_sq = max_r * max_r;
        halo_min_r_sq = min_r * min_r;

        let r_i32 = max_r.ceil() as i32;
        halo_min_y = (halo_cy as i32 - r_i32).max(0);
        halo_max_y = (halo_cy as i32 + r_i32).min(height - 1);
        halo_min_x = (halo_cx as i32 - r_i32).max(0);
        halo_max_x = (halo_cx as i32 + r_i32).min(width - 1);
    }

    let pixels = fb.as_mut_slice();

    let process_row = |y: i32, row_slice: &mut [u32]| {
        let y_f32 = y as f32;

        for g in &renderables {
            if y >= g.min_y && y <= g.max_y {
                let dy = y_f32 - g.cy;
                let dy_sq = dy * dy;

                for x in g.min_x..=g.max_x {
                    let dx = x as f32 - g.cx;
                    let dist_sq = dx * dx + dy_sq;
                    if dist_sq <= g.r_sq {
                        let dist = dist_sq.sqrt();
                        let intensity = (256.0 * (1.0 - (dist / g.r))) as u32;
                        if intensity > 0 {
                            let idx = x as usize;
                            row_slice[idx] = add_blend_int(row_slice[idx], g.color, intensity);
                        }
                    }
                }
            }
        }

        if has_halo && y >= halo_min_y && y <= halo_max_y {
            let dy = y_f32 - halo_cy;
            let dy_sq = dy * dy;

            for x in halo_min_x..=halo_max_x {
                let dx = x as f32 - halo_cx;
                let dist_sq = dx * dx + dy_sq;
                if dist_sq <= halo_max_r_sq && dist_sq >= halo_min_r_sq {
                    let dist = dist_sq.sqrt();
                    let center_dist = (dist - config.halo_radius).abs();
                    let intensity = (256.0 * (1.0 - (center_dist / config.halo_thickness))) as i32;
                    if intensity > 0 {
                        let idx = x as usize;
                        row_slice[idx] =
                            add_blend_int(row_slice[idx], config.halo_color, intensity as u32);
                    }
                }
            }
        }
    };

    #[cfg(feature = "parallel")]
    {
        pixels
            .par_chunks_exact_mut(width as usize)
            .enumerate()
            .for_each(|(y, row_slice)| {
                process_row(y as i32, row_slice);
            });
    }

    #[cfg(not(feature = "parallel"))]
    {
        pixels
            .chunks_exact_mut(width as usize)
            .enumerate()
            .for_each(|(y, row_slice)| {
                process_row(y as i32, row_slice);
            });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lens_flare_default_config() {
        let config = LensFlareConfig::default();
        assert_eq!(config.ghosts.len(), 5);
        assert!(config.halo_radius > 0.0);
    }

    #[test]
    fn test_apply_lens_flare_does_not_panic() {
        let mut fb = Framebuffer::new(100, 100).unwrap();
        let config = LensFlareConfig::default();
        apply_lens_flare(&mut fb, Vec2::new(50.0, 50.0), &config);
    }

    #[test]
    fn test_add_blend_int_clamping() {
        // Red channel overflow check
        let c1 = 0xFF_FF0000;
        let c2 = 0xFF_FF0000;
        let result = add_blend_int(c1, c2, 256);
        assert_eq!(result, 0xFF_FF0000); // Should clamp to 255 (FF)

        // Green channel
        let c1 = 0xFF_00AA00;
        let c2 = 0xFF_00AA00;
        let result = add_blend_int(c1, c2, 256);
        assert_eq!(result, 0xFF_00FF00); // 0xAA * 2 > 0xFF -> clamped to 0xFF
    }

    #[test]
    fn test_lens_flare_offscreen_light() {
        // Light far outside the screen should still render correctly
        // without panicking, and some ghosts might appear on screen.
        let mut fb = Framebuffer::new(100, 100).unwrap();
        let config = LensFlareConfig::default();
        apply_lens_flare(&mut fb, Vec2::new(5000.0, -5000.0), &config);
    }
}
