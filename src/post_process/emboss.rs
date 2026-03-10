use crate::framebuffer::Framebuffer;
#[cfg(feature = "parallel")]
use rayon::prelude::*;

pub fn apply_emboss(fb: &mut Framebuffer) {
    let width = fb.width() as usize;
    let height = fb.height() as usize;
    if width < 3 || height < 3 {
        return;
    }

    // We need a clone of the source to read from, since we modify in-place
    let src = fb.as_slice().to_vec();
    let dest = fb.as_mut_slice();

    #[cfg(feature = "parallel")]
    let iter = dest.par_chunks_mut(width).enumerate();
    #[cfg(not(feature = "parallel"))]
    let iter = dest.chunks_mut(width).enumerate();

    iter.for_each(|(y, row)| {
        for (x, dst_pixel) in row.iter_mut().enumerate() {
            let mut r_acc: i32 = 0;
            let mut g_acc: i32 = 0;
            let mut b_acc: i32 = 0;

            for dy in -1..=1 {
                for dx in -1..=1 {
                    let ny = (y as isize + dy).clamp(0, (height - 1) as isize) as usize;
                    let nx = (x as isize + dx).clamp(0, (width - 1) as isize) as usize;

                    let pixel = src[ny * width + nx];
                    let r = ((pixel >> 16) & 0xFF) as i32;
                    let g = ((pixel >> 8) & 0xFF) as i32;
                    let b = (pixel & 0xFF) as i32;

                    // Kernel:
                    // [-1, -1,  0]
                    // [-1,  0,  1]
                    // [ 0,  1,  1]
                    let weight = match (dx, dy) {
                        (-1, -1) => -1,
                        (0, -1) => -1,
                        (1, -1) => 0,
                        (-1, 0) => -1,
                        (0, 0) => 0,
                        (1, 0) => 1,
                        (-1, 1) => 0,
                        (0, 1) => 1,
                        (1, 1) => 1,
                        _ => 0,
                    };

                    r_acc += r * weight;
                    g_acc += g * weight;
                    b_acc += b * weight;
                }
            }

            // Calculate a single intensity value for the emboss effect
            let diff = (r_acc + g_acc + b_acc) / 3;
            let bias = 128;
            let final_val = (diff + bias).clamp(0, 255) as u32;

            *dst_pixel = 0xFF00_0000 | (final_val << 16) | (final_val << 8) | final_val;
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_apply_emboss() {
        let mut fb = Framebuffer::new(5, 5).unwrap();

        // Fill with white
        fb.clear(0xFFFFFFFF);

        // Put a black pixel in the center to create an "edge"
        fb.set_pixel(2, 2, 0xFF000000);

        apply_emboss(&mut fb);

        // The top-left corner should be a flat area, so it should become neutral grey (128)
        let top_left = fb.get_pixel(0, 0).unwrap();

        let r = (top_left >> 16) & 0xFF;
        let g = (top_left >> 8) & 0xFF;
        let b = top_left & 0xFF;
        assert_eq!(r, 128, "R should be 128 in flat areas, but was {r}");
        assert_eq!(g, 128, "G should be 128 in flat areas, but was {g}");
        assert_eq!(b, 128, "B should be 128 in flat areas, but was {b}");

        // Let's just check that the pixel adjacent to the center (which is an edge) is NOT just the flat grey value.
        // At the exact center of a single black pixel, the convolution might be symmetrical.
        // Wait, for (2,2) in a 5x5:
        // dy=-1, dx=-1 -> src(1,1)=white
        // dy=1, dx=1 -> src(3,3)=white
        // The emboss kernel is anti-symmetric: [-1 -1 0; -1 0 1; 0 1 1].
        // If all neighbors are white, their differences cancel out!
        // At (2,2), pixel is black, but weight at (0,0) is 0. Wait, the kernel doesn't use the center pixel itself!
        // weight for (0,0) is 0.
        // So at (2,2), ALL neighbors are white. It evaluates to 0, which maps to 128!
        // We should check (1,1) or (3,3) which are adjacent to the black pixel.
        let edge_pixel = fb.get_pixel(1, 1).unwrap();
        assert_ne!(edge_pixel, 0xFF808080, "Edges should be highlighted");
    }
}
