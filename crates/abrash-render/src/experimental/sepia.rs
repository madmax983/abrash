use abrash_core::framebuffer::Framebuffer;

pub fn apply_sepia(fb: &mut Framebuffer) {
    for pixel in fb.as_mut_slice().iter_mut() {
        let a = (*pixel >> 24) & 0xFF;
        let r = (*pixel >> 16) & 0xFF;
        let g = (*pixel >> 8) & 0xFF;
        let b = *pixel & 0xFF;

        // Multiply by 1024 (>> 10)
        let tr = ((r * 402 + g * 787 + b * 193) >> 10).min(255);
        let tg = ((r * 357 + g * 702 + b * 172) >> 10).min(255);
        let tb = ((r * 278 + g * 546 + b * 134) >> 10).min(255);

        *pixel = (a << 24) | (tr << 16) | (tg << 8) | tb;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_apply_sepia_colors() {
        let mut fb = Framebuffer::new(1, 1).unwrap();
        fb.set_pixel(0, 0, 0xFFFFFFFF); // White
        apply_sepia(&mut fb);
        let p = fb.get_pixel(0, 0).unwrap();
        assert_eq!(p, 0xFFFFFFEE);
    }
}
