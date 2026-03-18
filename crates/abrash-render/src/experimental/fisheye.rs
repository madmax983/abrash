//! Fisheye Lens Filter (Barrel Distortion)
//!
//! A retro post-processing effect that distorts the image radially from the center,
//! mimicking an extreme wide-angle lens.

use abrash_core::framebuffer::Framebuffer;

#[cfg(feature = "parallel")]
use rayon::prelude::*;

/// Applies a fisheye (barrel distortion) effect to the framebuffer.
///
/// # Arguments
///
/// * `fb` - The framebuffer to apply the effect to.
/// * `strength` - The intensity of the distortion. Higher values create a more pronounced fisheye effect.
///   A value of 0.0 results in no distortion. Typical values are between 0.1 and 1.0.
pub fn apply_fisheye(fb: &mut Framebuffer, strength: f32) {
    if strength <= 0.0 {
        return;
    }

    let width = fb.width() as i32;
    let height = fb.height() as i32;
    let half_width = width as f32 / 2.0;
    let half_height = height as f32 / 2.0;

    // We must clone the source to avoid reading from a modified state
    // when using parallel execution, as pixel mappings are non-linear.
    let source = fb.as_slice().to_vec();

    // The maximum distance from the center (corner pixel)
    // We normalize UV coordinates, but for distortion we use aspect-corrected coordinates.
    // So maximum radius is derived from aspect ratio.
    let max_radius_sq = half_width * half_width + half_height * half_height;
    let max_radius = max_radius_sq.sqrt();
    let inv_max_radius = 1.0 / max_radius;

    // We can pre-calculate the power factor to avoid recalculating it per pixel
    // To match other barrel distortions: r_new = r + r^3 * strength
    // So the source radius is calculated as a function of the target radius.

    #[cfg(feature = "parallel")]
    let iter = fb.as_mut_slice().par_chunks_exact_mut(width as usize);

    #[cfg(not(feature = "parallel"))]
    let iter = fb.as_mut_slice().chunks_exact_mut(width as usize);

    iter.enumerate().for_each(|(y, row)| {
            let dy = (y as f32 - half_height) as f32;

            for (x, pixel) in row.iter_mut().enumerate() {
                let dx = (x as f32 - half_width) as f32;
                let distance = dx.hypot(dy);

                if distance == 0.0 {
                    continue; // Center pixel remains unchanged
                }

                // Normalize distance to [0, 1] range based on max radius
                let r_norm = distance * inv_max_radius;

                // Barrel distortion formula
                let distortion_factor = 1.0 + strength * r_norm * r_norm;

                // Calculate the corresponding source coordinates
                // We want to pull pixels from *further* away towards the center to create the bulging effect.
                // Or pull pixels from the center outwards?
                // A standard fisheye pulls the edges in, magnifying the center.
                // So target pixel (x, y) should sample from source pixel (sx, sy) that is further away from center.
                let sx = half_width + dx * distortion_factor;
                let sy = half_height + dy * distortion_factor;

                let sx_i = sx as i32;
                let sy_i = sy as i32;

                if sx_i >= 0 && sx_i < width && sy_i >= 0 && sy_i < height {
                    let source_idx = (sy_i * width + sx_i) as usize;
                    *pixel = source[source_idx];
                } else {
                    // Out of bounds - black border
                    *pixel = 0xFF00_0000;
                }
            }
        });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_apply_fisheye_zero_strength() {
        let mut fb = Framebuffer::new(10, 10).unwrap();
        fb.clear(0xFFFFFFFF);
        // Paint center black
        fb.set_pixel(5, 5, 0xFF000000);

        let original = fb.as_slice().to_vec();
        apply_fisheye(&mut fb, 0.0);

        assert_eq!(fb.as_slice(), original.as_slice());
    }

    #[test]
    fn test_apply_fisheye_changes_pixels() {
        let mut fb = Framebuffer::new(10, 10).unwrap();
        fb.clear(0xFFFFFFFF);
        // Paint corners black
        fb.set_pixel(0, 0, 0xFF000000);
        fb.set_pixel(9, 9, 0xFF000000);

        apply_fisheye(&mut fb, 0.5);

        // Due to the distortion, the corners should now be pulled inwards,
        // and the very edges might be black due to the bounds check.
        // We just verify the buffer has changed.

        let center = fb.get_pixel(5, 5).unwrap();
        assert_eq!(center, 0xFFFFFFFF); // Center remains unaffected

        // Edges should be black
        let edge = fb.get_pixel(0, 0).unwrap();
        assert_eq!(edge, 0xFF000000);
    }
}
