with open("src/experimental/directional_blur.rs", "r") as f:
    lines = f.readlines()

for i in range(len(lines) - 1, -1, -1):
    if lines[i].strip() == "}":
        insert_idx = i
        break

new_tests = """
    #[test]
    fn test_directional_blur_vertical() {
        let mut fb = Framebuffer::new(1, 4).unwrap();
        fb.set_pixel(0, 0, 0xFFFFFFFF); // White pixel
        fb.set_pixel(0, 1, 0xFF000000); // Black pixels
        fb.set_pixel(0, 2, 0xFF000000);
        fb.set_pixel(0, 3, 0xFF000000);

        // Blur downwards by 3 pixels, 3 samples
        let config = DirectionalBlurConfig {
            dx: 0.0,
            dy: 3.0,
            num_samples: 3,
        };
        apply_directional_blur(&mut fb, &config);

        // The white pixel should be spread
        let p0 = fb.get_pixel(0, 0).unwrap();
        let p1 = fb.get_pixel(0, 1).unwrap();

        // At y=0, samples at y=0, 1, 2. (White, Black, Black) -> ~1/3 White
        assert!(p0 != 0xFFFFFFFF);
        assert!(p0 != 0xFF000000);

        // At y=1, samples at y=1, 2, 3. (Black, Black, Black) -> Black
        assert_eq!(p1, 0xFF000000);
    }

    #[test]
    fn test_directional_blur_diagonal() {
        let mut fb = Framebuffer::new(3, 3).unwrap();
        fb.set_pixel(0, 0, 0xFFFFFFFF); // White pixel top-left
        fb.set_pixel(1, 1, 0xFF000000);
        fb.set_pixel(2, 2, 0xFF000000);

        let config = DirectionalBlurConfig {
            dx: 2.0,
            dy: 2.0,
            num_samples: 2,
        };
        apply_directional_blur(&mut fb, &config);

        let p00 = fb.get_pixel(0, 0).unwrap();

        assert!(p00 != 0xFFFFFFFF);
        assert!(p00 != 0xFF000000);
    }

    #[test]
    fn test_directional_blur_zero_size_framebuffer() {
        let mut fb = Framebuffer::new(0, 0).unwrap();

        let config = DirectionalBlurConfig {
            dx: 10.0,
            dy: 0.0,
            num_samples: 5,
        };
        // Should not panic
        apply_directional_blur(&mut fb, &config);
    }

    #[test]
    fn test_directional_blur_huge_samples() {
        let mut fb = Framebuffer::new(2, 2).unwrap();
        fb.set_pixel(0, 0, 0xFFFFFFFF);
        fb.set_pixel(1, 0, 0xFF000000);
        fb.set_pixel(0, 1, 0xFF000000);
        fb.set_pixel(1, 1, 0xFF000000);

        let config = DirectionalBlurConfig {
            dx: 1.0,
            dy: 1.0,
            num_samples: 10000,
        };
        // Should not panic
        apply_directional_blur(&mut fb, &config);
    }
"""

lines.insert(insert_idx, new_tests)

with open("src/experimental/directional_blur.rs", "w") as f:
    f.writelines(lines)
