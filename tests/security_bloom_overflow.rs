#[cfg(test)]
mod tests {
    use abrash::post_process::bloom::apply_bloom;
    use abrash::framebuffer::Framebuffer;

    #[test]
    fn test_bloom_additive_overflow() {
        let width = 16;
        let height = 16;
        let mut fb = Framebuffer::new(width, height).unwrap();

        // Set all pixels to white (255, 255, 255, 255)
        for y in 0..height {
            for x in 0..width {
                fb.set_pixel(x as i32, y as i32, 0xFFFFFFFF);
            }
        }

        // Apply bloom with high intensity and threshold 0 (so all pixels are bright)
        // Intensity 1.0 means we add the pixel value again.
        apply_bloom(&mut fb, 0, 1, 1.0);

        let pixel = fb.get_pixel(8, 8).unwrap();
        let r = (pixel >> 16) & 0xFF;
        let g = (pixel >> 8) & 0xFF;
        let b = pixel & 0xFF;

        assert_eq!(r, 255, "Red channel should be saturated at 255, got {}", r);
        assert_eq!(g, 255, "Green channel should be saturated at 255, got {}", g);
        assert_eq!(b, 255, "Blue channel should be saturated at 255, got {}", b);
    }
}
