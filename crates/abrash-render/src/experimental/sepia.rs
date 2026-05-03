use crate::framebuffer::Framebuffer;
#[cfg(feature = "parallel")]
use rayon::prelude::*;

pub fn apply_sepia(framebuffer: &mut Framebuffer) {
    let width = framebuffer.width() as usize;

    let process_row = |row: &mut [u32]| {
        for pixel in row.iter_mut() {
            let r = (*pixel >> 16) & 0xFF;
            let g = (*pixel >> 8) & 0xFF;
            let b = *pixel & 0xFF;

            // Fixed-point weights (weight * 65536)
            let tr = ((r * 25755) + (g * 50397) + (b * 12386)) >> 16;
            let tg = ((r * 22872) + (g * 44957) + (b * 11010)) >> 16;
            let tb = ((r * 17825) + (g * 34996) + (b * 8585)) >> 16;

            let tr = tr.min(255);
            let tg = tg.min(255);
            let tb = tb.min(255);

            *pixel = 0xFF00_0000 | (tr << 16) | (tg << 8) | tb;
        }
    };

    #[cfg(feature = "parallel")]
    framebuffer.as_mut_slice().par_chunks_exact_mut(width).for_each(process_row);

    #[cfg(not(feature = "parallel"))]
    framebuffer.as_mut_slice().chunks_exact_mut(width).for_each(process_row);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::framebuffer::Framebuffer;

    #[test]
    fn test_apply_sepia() {
        let mut fb = Framebuffer::new(1, 1).unwrap();
        fb.as_mut_slice()[0] = 0xFFFFFFFF; // White pixel
        apply_sepia(&mut fb);
        let p = fb.as_mut_slice()[0];
        let b = p & 0xFF;
        assert!(b < 255); // The exact value will depend on implementation, but B should drop.
    }
}
