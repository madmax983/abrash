//! Pointillism Post-Processing Filter
//!
//! A post-processing effect that converts the framebuffer into a stylized
//! pointillist painting by splatting overlapping procedural dots.

use crate::framebuffer::Framebuffer;

/// Applies a pointillism effect to the framebuffer.
///
/// Overwrites the framebuffer with splatted dots sampled from the original image.
///
/// # Panics
///
/// Panics if the internal framebuffer allocation fails due to memory exhaustion.
///
/// # Arguments
///
/// * `fb` - The framebuffer to modify in-place.
/// * `cell_size` - The base size of the dots.
pub fn apply_pointillism(fb: &mut Framebuffer, cell_size: f32) {
    if cell_size <= 1.0 {
        return;
    }

    let width = fb.width() as usize;
    let height = fb.height() as usize;
    if width == 0 || height == 0 {
        return;
    }

    let mut src_fb = Framebuffer::new(width as u32, height as u32).unwrap();
    src_fb.as_mut_slice().copy_from_slice(fb.as_slice());
    let src_pixels = src_fb.as_slice();
    let pixels = fb.as_mut_slice();

    let inv_cell = 1.0 / cell_size;

    // Procedural hash function
    let hash = |x: i32, y: i32| -> f32 {
        let mut h = x
            .wrapping_mul(374_761_393)
            .wrapping_add(y.wrapping_mul(668_265_263));
        h = (h ^ (h >> 13)).wrapping_mul(127_412_617);
        ((h ^ (h >> 16)) as u32 as f32) / (u32::MAX as f32)
    };

    #[cfg(feature = "parallel")]
    {
        use rayon::prelude::*;
        pixels
            .par_chunks_mut(width)
            .enumerate()
            .for_each(|(y, row)| {
                for x in 0..width {
                    let px = x as f32;
                    let py = y as f32;

                    let cx = (px * inv_cell).floor() as i32;
                    let cy = (py * inv_cell).floor() as i32;

                    let mut best_z = -1.0;
                    let mut best_color = 0xFFFF_FFFF;

                    // Evaluate 3x3 neighborhood of cells to find overlapping dots
                    for dy in -1..=1 {
                        for dx in -1..=1 {
                            let nx = cx + dx;
                            let ny = cy + dy;

                            let h1 = hash(nx, ny);
                            let h2 = hash(nx + 100, ny);
                            let h3 = hash(nx, ny + 100);
                            let h4 = hash(nx + 200, ny + 200); // Depth/Order hash

                            let dot_x = (nx as f32 + h1) * cell_size;
                            let dot_y = (ny as f32 + h2) * cell_size;
                            let dot_radius = cell_size * (0.5 + h3 * 0.8);
                            let dot_z = h4; // Random depth to resolve overlaps

                            let dist_sq = (px - dot_x) * (px - dot_x) + (py - dot_y) * (py - dot_y);

                            if dist_sq <= dot_radius * dot_radius {
                                if dot_z > best_z {
                                    best_z = dot_z;

                                    let sample_x =
                                        (dot_x as i32).clamp(0, width as i32 - 1) as usize;
                                    let sample_y =
                                        (dot_y as i32).clamp(0, height as i32 - 1) as usize;
                                    best_color = src_pixels[sample_y * width + sample_x];
                                }
                            }
                        }
                    }

                    if best_z >= 0.0 {
                        row[x] = best_color;
                    } else {
                        row[x] = src_pixels[y * width + x];
                    }
                }
            });
    }

    #[cfg(not(feature = "parallel"))]
    {
        for y in 0..height {
            for x in 0..width {
                let px = x as f32;
                let py = y as f32;

                let cx = (px * inv_cell).floor() as i32;
                let cy = (py * inv_cell).floor() as i32;

                let mut best_z = -1.0;
                let mut best_color = 0xFFFF_FFFF;

                for dy in -1..=1 {
                    for dx in -1..=1 {
                        let nx = cx + dx;
                        let ny = cy + dy;

                        let h1 = hash(nx, ny);
                        let h2 = hash(nx + 100, ny);
                        let h3 = hash(nx, ny + 100);
                        let h4 = hash(nx + 200, ny + 200);

                        let dot_x = (nx as f32 + h1) * cell_size;
                        let dot_y = (ny as f32 + h2) * cell_size;
                        let dot_radius = cell_size * (0.5 + h3 * 0.8);
                        let dot_z = h4;

                        let dist_sq = (px - dot_x) * (px - dot_x) + (py - dot_y) * (py - dot_y);

                        if dist_sq <= dot_radius * dot_radius {
                            if dot_z > best_z {
                                best_z = dot_z;

                                let sample_x = (dot_x as i32).clamp(0, width as i32 - 1) as usize;
                                let sample_y = (dot_y as i32).clamp(0, height as i32 - 1) as usize;
                                best_color = src_pixels[sample_y * width + sample_x];
                            }
                        }
                    }
                }

                if best_z >= 0.0 {
                    pixels[y * width + x] = best_color;
                } else {
                    pixels[y * width + x] = src_pixels[y * width + x];
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::framebuffer::Framebuffer;

    #[test]
    fn test_apply_pointillism() {
        let mut fb = Framebuffer::new(100, 100).unwrap();
        // Clear to white
        fb.clear(0xFFFF_FFFF);

        // Draw a black square in the middle
        for y in 40..60 {
            for x in 40..60 {
                fb.as_mut_slice()[y * 100 + x] = 0xFF00_0000;
            }
        }

        apply_pointillism(&mut fb, 5.0);

        // Since Pointillism splats overlapping dots sampled from the original image,
        // the sharp black square should be somewhat distorted, meaning some white pixels
        // might enter the 40..60 region, or black pixels might escape it.
        // It's sufficient to check that the buffer changed from the original square.

        let mut original_fb = Framebuffer::new(100, 100).unwrap();
        original_fb.clear(0xFFFF_FFFF);
        for y in 40..60 {
            for x in 40..60 {
                original_fb.as_mut_slice()[y * 100 + x] = 0xFF00_0000;
            }
        }

        let has_changed = fb
            .as_slice()
            .iter()
            .zip(original_fb.as_slice().iter())
            .any(|(&a, &b)| a != b);
        assert!(has_changed, "Pointillism should alter the image");
    }
}
