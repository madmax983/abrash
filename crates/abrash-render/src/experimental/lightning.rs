//! Procedural Lightning Generator.
//!
//! Provides a screen-space electrical arc generator using the recursive
//! midpoint displacement algorithm. Can be used for spell effects or sci-fi visuals.

use abrash_core::framebuffer::Framebuffer;
use abrash_core::math::Vec2;
use rand::{Rng, RngExt, SeedableRng};

/// A single line segment of a lightning bolt.
#[derive(Clone, Debug)]
pub struct LightningSegment {
    pub start: Vec2,
    pub end: Vec2,
    pub thickness: f32,
    pub intensity: f32,
}

/// A procedurally generated lightning bolt.
#[derive(Clone, Debug)]
pub struct LightningBolt {
    pub start: Vec2,
    pub end: Vec2,
    pub color: u32,
    pub segments: Vec<LightningSegment>,
}

impl LightningBolt {
    /// Generates a new lightning bolt using midpoint displacement.
    #[must_use]
    pub fn generate(
        start: Vec2,
        end: Vec2,
        color: u32,
        jaggedness: f32,
        generations: u32,
        thickness: f32,
        seed: u64,
    ) -> Self {
        let mut rng = rand::rngs::StdRng::seed_from_u64(seed);
        let mut segments = vec![LightningSegment {
            start,
            end,
            thickness,
            intensity: 1.0,
        }];

        let offset_amount = jaggedness;

        for _ in 0..generations {
            let mut new_segments = Vec::with_capacity(segments.len() * 2);
            for seg in segments {
                let mid = (seg.start + seg.end) * 0.5;
                let dir = (seg.end - seg.start).normalize_or_zero();
                let normal = Vec2::new(-dir.y, dir.x);
                let offset =
                    (rng.random::<f32>() - 0.5) * offset_amount * (seg.start - seg.end).length();
                let displaced_mid = mid + normal * offset;

                new_segments.push(LightningSegment {
                    start: seg.start,
                    end: displaced_mid,
                    thickness: seg.thickness,
                    intensity: seg.intensity,
                });
                new_segments.push(LightningSegment {
                    start: displaced_mid,
                    end: seg.end,
                    thickness: seg.thickness,
                    intensity: seg.intensity,
                });
            }
            segments = new_segments;
        }

        Self {
            start,
            end,
            color,
            segments,
        }
    }
}

/// Helper to additively blend an intensity onto an existing ARGB pixel.
#[inline(always)]
fn add_blend_int(dest: u32, color: u32, intensity: f32) -> u32 {
    let int_u32 = (intensity.clamp(0.0, 1.0) * 256.0) as u32;
    if int_u32 == 0 {
        return dest;
    }

    let a1 = (dest >> 24) & 0xFF;
    let r1 = (dest >> 16) & 0xFF;
    let g1 = (dest >> 8) & 0xFF;
    let b1 = dest & 0xFF;

    let a2 = (color >> 24) & 0xFF;
    let r2 = (color >> 16) & 0xFF;
    let g2 = (color >> 8) & 0xFF;
    let b2 = color & 0xFF;

    let a_src = (a2 * int_u32) >> 8;
    let r_src = (r2 * int_u32) >> 8;
    let g_src = (g2 * int_u32) >> 8;
    let b_src = (b2 * int_u32) >> 8;

    let a_out = (a1 + a_src).min(255);
    let r_out = (r1 + r_src).min(255);
    let g_out = (g1 + g_src).min(255);
    let b_out = (b1 + b_src).min(255);

    (a_out << 24) | (r_out << 16) | (g_out << 8) | b_out
}

/// Renders the lightning bolt onto the framebuffer with an additive glow effect.
pub fn apply_lightning(fb: &mut Framebuffer, bolt: &LightningBolt) {
    let width = fb.width() as i32;
    let height = fb.height() as i32;
    let pixels = fb.as_mut_slice();

    for seg in &bolt.segments {
        let max_dist = seg.thickness;
        let max_dist_sq = max_dist * max_dist;

        let min_x = (seg.start.x.min(seg.end.x) - max_dist).max(0.0) as i32;
        let max_x = (seg.start.x.max(seg.end.x) + max_dist).min((width - 1) as f32) as i32;
        let min_y = (seg.start.y.min(seg.end.y) - max_dist).max(0.0) as i32;
        let max_y = (seg.start.y.max(seg.end.y) + max_dist).min((height - 1) as f32) as i32;

        let dir = seg.end - seg.start;
        let len_sq = dir.x * dir.x + dir.y * dir.y;
        if len_sq < 1e-5 {
            continue;
        }

        for y in min_y..=max_y {
            let row_offset = (y * width) as usize;
            for x in min_x..=max_x {
                let px = x as f32;
                let py = y as f32;

                // Distance from point to line segment
                let t = ((px - seg.start.x) * dir.x + (py - seg.start.y) * dir.y) / len_sq;
                let t_clamped = t.clamp(0.0, 1.0);
                let proj_x = seg.start.x + t_clamped * dir.x;
                let proj_y = seg.start.y + t_clamped * dir.y;

                let dist_sq = (px - proj_x) * (px - proj_x) + (py - proj_y) * (py - proj_y);

                if dist_sq <= max_dist_sq {
                    let dist = dist_sq.sqrt();
                    // Basic falloff for soft edges
                    let intensity = (1.0 - (dist / max_dist)) * seg.intensity;
                    if intensity > 0.0 {
                        let idx = row_offset + x as usize;
                        pixels[idx] = add_blend_int(pixels[idx], bolt.color, intensity);
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lightning_generation() {
        let start = Vec2::new(10.0, 10.0);
        let end = Vec2::new(100.0, 100.0);
        let generations = 3;

        let bolt = LightningBolt::generate(start, end, 0xFFFFFFFF, 0.5, generations, 2.0, 42);

        assert_eq!(bolt.segments.len(), 2_usize.pow(generations));
        assert_eq!(bolt.segments[0].start, start);
        assert_eq!(bolt.segments.last().unwrap().end, end);
    }
}
