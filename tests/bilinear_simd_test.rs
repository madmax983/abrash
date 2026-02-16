use abrash::framebuffer::Framebuffer;
use abrash::rasterizer::{
    PerspectiveSpanStart, PerspectiveTextureGradients, draw_scanline_textured_perspective,
};
use abrash::texture::{FilterMode, Texture};
use abrash::zbuffer::ZBuffer;

#[test]
fn test_bilinear_scanline_output() {
    let width = 200;
    let height = 100;
    let mut fb = Framebuffer::new(width, height).unwrap();
    let mut zb = ZBuffer::new(width, height).unwrap();
    let mut tex = Texture::new(4, 4).unwrap();

    // Fill texture with gradient
    // (0,0)=0, (3,0)=255, (0,3)=0, (3,3)=255
    for y in 0..4 {
        for x in 0..4 {
            let val = (x * 85) as u32; // 0, 85, 170, 255
            let color = 0xFF000000 | val; // Blue channel gradient
            tex.set_pixel(x, y, color);
        }
    }
    tex.filter_mode = FilterMode::Bilinear;

    let y = 50;
    let x_start = 10;
    let x_end = 20; // 10 pixels

    // Traverse u from 0.0 to 1.0 across 10 pixels
    let gradients = PerspectiveTextureGradients {
        dz_dx: 0.0,
        dq_dx: 0.0,
        du_dx: 0.1, // 0.1 per pixel * 10 pixels = 1.0
        dv_dx: 0.0,
        dq_dy: 0.0,
        du_dy: 0.0,
        dv_dy: 0.0,
    };

    let start = PerspectiveSpanStart {
        z: 0.5,
        q: 1.0,
        u: 0.0, // Start at u=0.0
        v: 0.0,
    };

    draw_scanline_textured_perspective(
        &mut fb, &mut zb, &tex, y, x_start, x_end, start, &gradients,
    );

    // Verify output
    // u goes 0.0, 0.1, 0.2 ... 1.0
    // Texture is 4 wide. u=0.0 -> x=0. u=1.0 -> x=4 (wrap/clamp)
    // Bilinear should show smooth gradient.

    let width_usize = width as usize;
    let y_offset = (y as usize) * width_usize;
    let buffer = fb.as_slice();

    for x in x_start..=x_end {
        let pixel = buffer[y_offset + x as usize];
        let blue = pixel & 0xFF;
        println!("x={}: blue={}", x, blue);

        // Expected behavior: monotonic increase
        if x > x_start {
            let prev_pixel = buffer[y_offset + (x - 1) as usize];
            let prev_blue = prev_pixel & 0xFF;
            assert!(blue >= prev_blue, "Gradient should increase monotonically");
        }
    }
}
