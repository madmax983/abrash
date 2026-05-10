//! Metaballs Rendering Filter
//!
//! Experimental 2D metaballs implementation.

use crate::framebuffer::Framebuffer;
use abrash_core::math::Vec2;

/// A simple 2D metaball.
pub struct Metaball {
    pub position: Vec2,
    pub radius: f32,
    pub color: u32,
}

/// Renders metaballs into the framebuffer.
///
/// This function calculates the influence of all metaballs on each pixel
/// and colors the pixel if the combined influence is above a threshold.
pub fn render_metaballs(fb: &mut Framebuffer, metaballs: &[Metaball], threshold: f32) {
    let width = fb.width() as usize;
    let height = fb.height() as usize;

    let pixels = fb.as_mut_slice();

    for y in 0..height {
        let y_f32 = y as f32;
        for x in 0..width {
            let x_f32 = x as f32;
            let mut total_influence = 0.0;
            let mut blended_r = 0.0;
            let mut blended_g = 0.0;
            let mut blended_b = 0.0;

            for metaball in metaballs {
                let dx = x_f32 - metaball.position.x;
                let dy = y_f32 - metaball.position.y;
                let distance_sq = dx * dx + dy * dy;

                // Influence function: R^2 / d^2
                // Avoid division by zero and sqrt!
                let influence = if distance_sq > 0.0001 {
                    (metaball.radius * metaball.radius) / distance_sq
                } else {
                    1000.0 // Arbitrary high influence at center
                };

                total_influence += influence;

                // Simple additive blending based on influence contribution
                let r = ((metaball.color >> 16) & 0xFF) as f32;
                let g = ((metaball.color >> 8) & 0xFF) as f32;
                let b = (metaball.color & 0xFF) as f32;

                blended_r += r * influence;
                blended_g += g * influence;
                blended_b += b * influence;
            }

            if total_influence >= threshold {
                // Normalize color based on total influence
                let final_r = (blended_r / total_influence).clamp(0.0, 255.0) as u32;
                let final_g = (blended_g / total_influence).clamp(0.0, 255.0) as u32;
                let final_b = (blended_b / total_influence).clamp(0.0, 255.0) as u32;

                let idx = y * width + x;
                pixels[idx] = 0xFF00_0000 | (final_r << 16) | (final_g << 8) | final_b;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::framebuffer::Framebuffer;
    use abrash_core::math::Vec2;

    #[test]
    fn test_render_metaballs_draws_pixel() {
        let width = 10;
        let height = 10;
        let mut fb = Framebuffer::new(width, height).unwrap();
        fb.clear(0xFF00_0000); // Black

        let metaballs = vec![Metaball {
            position: Vec2::new(5.0, 5.0),
            radius: 4.0,
            color: 0xFFFF_0000, // Red
        }];

        render_metaballs(&mut fb, &metaballs, 1.0);

        // Center pixel should definitely be red because it is exactly at the position
        let center_pixel = fb.get_pixel(5, 5).unwrap();
        assert_eq!(
            center_pixel, 0xFFFF_0000,
            "Center pixel should be drawn red"
        );

        // Edge pixel should be black
        let edge_pixel = fb.get_pixel(0, 0).unwrap();
        assert_eq!(edge_pixel, 0xFF00_0000, "Edge pixel should remain black");
    }
}
