use abrash_core::framebuffer::Framebuffer;

/// Applies a frosted glass effect to the given framebuffer.
///
/// The `radius` parameter determines the maximum pixel displacement.
/// A larger radius results in a more blurred, heavily frosted appearance.
pub fn apply_frosted_glass(fb: &mut Framebuffer, radius: u32) {
    let width = fb.width() as usize;
    let height = fb.height() as usize;

    if width == 0 || height == 0 || radius == 0 {
        return;
    }

    let mut temp_buffer = vec![0; width * height];
    let src = fb.as_slice();

    let r_i32 = radius as i32;
    let w_i32 = width as i32;
    let h_i32 = height as i32;

    #[cfg(feature = "parallel")]
    use rayon::prelude::*;

    #[cfg(feature = "parallel")]
    let chunk_iter = temp_buffer.par_chunks_exact_mut(width).enumerate();
    #[cfg(not(feature = "parallel"))]
    let chunk_iter = temp_buffer.chunks_exact_mut(width).enumerate();

    chunk_iter.for_each(|(y, row)| {
        let mut state = (12345 + y * 98765) as u32;
        let mut xorshift = || {
            state ^= state << 13;
            state ^= state >> 17;
            state ^= state << 5;
            state
        };

        let y_i32 = y as i32;
        let diameter = radius * 2 + 1;

        for x in 0..width {
            let x_i32 = x as i32;

            // Generate random offsets in [-radius, radius]
            let dx = (xorshift() % diameter) as i32 - r_i32;
            let dy = (xorshift() % diameter) as i32 - r_i32;

            let nx = (x_i32 + dx).clamp(0, w_i32 - 1);
            let ny = (y_i32 + dy).clamp(0, h_i32 - 1);

            let src_idx = (ny * w_i32 + nx) as usize;
            row[x] = src[src_idx];
        }
    });

    fb.as_mut_slice().copy_from_slice(&temp_buffer);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_frosted_glass_modifies_pixels() {
        let width = 100;
        let height = 100;
        let mut fb = Framebuffer::new(width, height).unwrap();

        // Fill with a gradient pattern to easily detect changes
        for y in 0..height {
            for x in 0..width {
                let color = x | (y << 8);
                fb.set_pixel(x as i32, y as i32, color);
            }
        }

        // Clone original to compare
        let mut original_fb = Framebuffer::new(width, height).unwrap();
        original_fb.as_mut_slice().copy_from_slice(fb.as_slice());

        apply_frosted_glass(&mut fb, 5);

        // Ensure at least some pixels were displaced/changed
        let mut changes = 0;
        for i in 0..(width * height) as usize {
            if fb.as_slice()[i] != original_fb.as_slice()[i] {
                changes += 1;
            }
        }

        // Expect a significant number of pixels to have shifted
        assert!(changes > (width * height) / 4, "Frosted glass effect should modify a significant portion of the image");
    }
}
