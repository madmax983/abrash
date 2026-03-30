use abrash_core::framebuffer::Framebuffer;
use abrash_core::zbuffer::ZBuffer;

/// Experimental Depth Visualizer.
///
/// Replaces the Framebuffer contents with a grayscale depth map derived from the `ZBuffer`.
/// Closer objects appear lighter (closer to white), while further objects fade to black.
/// Infinity depth (background) is colored solid black or an optional background color.
pub fn depth_visualize(
    framebuffer: &mut Framebuffer,
    zbuffer: &ZBuffer,
    near_plane: f32,
    far_plane: f32,
) {
    if near_plane >= far_plane {
        return;
    }

    let fb_slice = framebuffer.as_mut_slice();
    let z_slice = zbuffer.as_slice();
    let min_len = fb_slice.len().min(z_slice.len());

    let range = far_plane - near_plane;
    let inv_range = if range > 0.0 { 1.0 / range } else { 1.0 };

    for i in 0..min_len {
        let depth = z_slice[i];
        if depth.is_infinite() || depth > far_plane {
            fb_slice[i] = 0xFF00_0000; // Black background
        } else {
            // Normalize depth to 0.0 .. 1.0
            let clamped = depth.clamp(near_plane, far_plane);
            let normalized = (clamped - near_plane) * inv_range;

            // Invert so near = 1.0 (white), far = 0.0 (black)
            let intensity = 1.0 - normalized;

            // Convert to 8-bit color
            let c = (intensity * 255.0) as u32;

            // Grayscale color: 0xFFRRGGBB
            fb_slice[i] = 0xFF00_0000 | (c << 16) | (c << 8) | c;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_depth_visualize() {
        let mut fb = Framebuffer::new(2, 2).unwrap();
        let mut zb = ZBuffer::new(2, 2).unwrap();

        // Pixel (0, 0): Very close (at near_plane)
        zb.test_and_set(0, 0, 1.0);

        // Pixel (1, 0): Very far (at far_plane)
        zb.test_and_set(1, 0, 100.0);

        // Pixel (0, 1): Middle
        zb.test_and_set(0, 1, 50.0);

        // Pixel (1, 1): Infinity (background)
        // Background remains at f32::INFINITY by default

        depth_visualize(&mut fb, &zb, 1.0, 100.0);

        // Near plane should map to bright white
        let p00 = fb.get_pixel(0, 0).unwrap() & 0x00FFFFFF;
        assert!(
            p00 > 0x00E0E0E0,
            "Close pixel should be bright, got {p00:#08X}"
        );

        // Far plane should map to very dark
        let p10 = fb.get_pixel(1, 0).unwrap() & 0x00FFFFFF;
        assert!(p10 < 0x00202020, "Far pixel should be dark, got {p10:#08X}");

        // Infinity should map to black (or dark color)
        let p11 = fb.get_pixel(1, 1).unwrap() & 0x00FFFFFF;
        assert_eq!(p11, 0x00000000, "Infinity pixel should be black");
    }
}
