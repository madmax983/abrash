use abrash::framebuffer::Framebuffer;
use abrash::post_process::apply_chromatic_aberration;

#[test]
fn test_verify_chromatic_aberration() {
    let width = 8;
    let height = 2;
    let mut fb = Framebuffer::new(width, height).unwrap();

    // Pattern: R=10*x, G=10*x+1, B=10*x+2
    for y in 0..height {
        for x in 0..width {
            let val = (x as u32 + 1) * 10;
            let r = val;
            let g = val + 1;
            let b = val + 2;
            let p = 0xFF00_0000 | (r << 16) | (g << 8) | b;
            fb.set_pixel(x as i32, y as i32, p);
        }
    }

    let offset = 2;
    apply_chromatic_aberration(&mut fb, offset);

    for y in 0..height {
        for x in 0..width {
            let p = fb.get_pixel(x as i32, y as i32).unwrap();

            // Expected
            // G, A from x
            let val = (x as u32 + 1) * 10;
            let expected_g = val + 1;
            let expected_a = 0xFF;

            // R from x - offset
            let expected_r = if x >= offset {
                let prev_x = x - offset;
                (prev_x + 1) * 10
            } else {
                0
            };

            // B from x + offset
            let expected_b = if x + offset < width {
                let next_x = x + offset;
                (next_x + 1) * 10 + 2
            } else {
                0
            };

            let got_r = (p >> 16) & 0xFF;
            let got_g = (p >> 8) & 0xFF;
            let got_b = p & 0xFF;
            let got_a = (p >> 24) & 0xFF;

            assert_eq!(got_r, expected_r, "Red mismatch at {},{}", x, y);
            assert_eq!(got_g, expected_g, "Green mismatch at {},{}", x, y);
            assert_eq!(got_b, expected_b, "Blue mismatch at {},{}", x, y);
            assert_eq!(got_a, expected_a, "Alpha mismatch at {},{}", x, y);
        }
    }
}
