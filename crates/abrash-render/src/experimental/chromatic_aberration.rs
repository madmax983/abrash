use abrash_core::framebuffer::Framebuffer;
use rayon::prelude::*;

pub fn apply_chromatic_aberration(fb: &mut Framebuffer, offset: i32) {
    let width = fb.width() as i32;

    // Guard against empty buffers as per Nova guidelines
    if width == 0 || fb.height() == 0 {
        return;
    }

    // We must clone the source buffer to prevent read/write tearing,
    // as described in the Persona guidelines for non-linear lookups.
    let src = fb.as_slice().to_vec();
    let dst = fb.as_mut_slice();
    let w = width as usize;

    dst.par_chunks_exact_mut(w)
        .enumerate()
        .for_each(|(y, row)| {
            let y_offset = y * w;

            for x in 0..w {
                let x_i32 = x as i32;

                // Fast clamp logic without branching or expensive method calls
                let mut r_x = x_i32 - offset;
                if r_x < 0 { r_x = 0; }
                else if r_x >= width { r_x = width - 1; }

                let mut b_x = x_i32 + offset;
                if b_x < 0 { b_x = 0; }
                else if b_x >= width { b_x = width - 1; }

                let r_idx = y_offset + r_x as usize;
                let b_idx = y_offset + b_x as usize;
                let src_idx = y_offset + x;

                let r_col = src[r_idx];
                let g_col = src[src_idx];
                let b_col = src[b_idx];

                let a = g_col & 0xFF_00_00_00;
                let r = r_col & 0x00_FF_00_00;
                let g = g_col & 0x00_00_FF_00;
                let b = b_col & 0x00_00_00_FF;

                row[x] = a | r | g | b;
            }
        });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chromatic_aberration_shift() {
        let mut fb = Framebuffer::new(3, 1).unwrap();
        // Set middle pixel to white, others black
        fb.set_pixel(0, 0, 0xFF_00_00_00);
        fb.set_pixel(1, 0, 0xFF_FF_FF_FF); // AARRGGBB
        fb.set_pixel(2, 0, 0xFF_00_00_00);

        apply_chromatic_aberration(&mut fb, 1);

        // With offset 1:
        // R shifts right by 1
        // B shifts left by 1
        // G stays in place
        // Old pixels:
        // 0: 00 00 00
        // 1: FF FF FF
        // 2: 00 00 00
        // New R:
        // 0: 00 (from -1)
        // 1: 00 (from 0)
        // 2: FF (from 1)
        // New G:
        // 0: 00
        // 1: FF
        // 2: 00
        // New B:
        // 0: FF (from 1)
        // 1: 00 (from 2)
        // 2: 00 (from 3)

        assert_eq!(fb.get_pixel(0, 0), Some(0xFF_00_00_FF)); // Pure Blue
        assert_eq!(fb.get_pixel(1, 0), Some(0xFF_00_FF_00)); // Pure Green
        assert_eq!(fb.get_pixel(2, 0), Some(0xFF_FF_00_00)); // Pure Red
    }
}
