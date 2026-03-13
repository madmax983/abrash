use abrash::framebuffer::Framebuffer;
use abrash::rasterizer::{
    TexturedGouraudGradients, TexturedGouraudSpanStart, draw_scanline_textured_gouraud,
};
use abrash::texture::Texture;
use abrash::zbuffer::ZBuffer;

#[test]
fn test_draw_scanline_textured_gouraud_correctness() {
    let width = 100;
    let height = 10;
    let mut fb = Framebuffer::new(width, height).unwrap();
    let mut zb = ZBuffer::new(width, height).unwrap();

    // Texture: 2x2. (0,0)=White, others Black.
    // We will sample (0,0) mostly.
    let mut tex = Texture::new(2, 2).unwrap();
    tex.set_pixel(0, 0, 0xFFFFFFFF); // White

    // Gradients
    // z=1. constant.
    // q=1. constant.
    // u=0. v=0. constant (Sample (0,0) -> White).
    // Color Gradients:
    // R: 0.0 -> 1.0 over 100 pixels. dr_dx = 0.01.
    // G: 1.0 -> 0.0 over 100 pixels. dg_dx = -0.01.
    // B: 0.5 constant. db_dx = 0.0.

    let gradients = TexturedGouraudGradients {
        dz_dx: 0.0,
        dq_dx: 0.0,
        du_dx: 0.0,
        dv_dx: 0.0,
        dr_dx: 0.01,
        dg_dx: -0.01,
        db_dx: 0.0,
        dq_dy: 0.0,
        du_dy: 0.0,
        dv_dy: 0.0,
        dr_dy: 0.0,
        dg_dy: 0.0,
        db_dy: 0.0,
    };

    let start = TexturedGouraudSpanStart {
        z: 1.0,
        q: 1.0,
        u: 0.0,
        v: 0.0,
        r: 0.0,
        g: 1.0,
        b: 0.5,
    };

    draw_scanline_textured_gouraud(&mut fb, &mut zb, 5, 0, 99, start, &gradients, &tex);

    // Verify Pixel 0
    // Tex=White(1.0). R=0.0. G=1.0. B=0.5.
    // Result = (0, 255, 127). (Alpha 255).
    let p0 = fb.get_pixel(0, 5).unwrap();
    let r0 = (p0 >> 16) & 0xFF;
    let g0 = (p0 >> 8) & 0xFF;
    let b0 = p0 & 0xFF;

    assert_eq!(r0, 0, "Pixel 0 R should be 0");
    assert_eq!(g0, 255, "Pixel 0 G should be 255");
    assert!((126..=128).contains(&b0), "Pixel 0 B should be ~127");

    // Verify Pixel 50 (Center)
    // R=0.5. G=0.5. B=0.5.
    // Result = (127, 127, 127).
    let p50 = fb.get_pixel(50, 5).unwrap();
    let r50 = (p50 >> 16) & 0xFF;
    let g50 = (p50 >> 8) & 0xFF;
    let b50 = p50 & 0xFF;

    assert!((126..=129).contains(&r50), "Pixel 50 R should be ~127");
    assert!((126..=129).contains(&g50), "Pixel 50 G should be ~127");
    assert!((126..=129).contains(&b50), "Pixel 50 B should be ~127");

    // Verify Pixel 99
    // R=0.99. G=0.01. B=0.5.
    // Result = (252, 2, 127).
    let p99 = fb.get_pixel(99, 5).unwrap();
    let r99 = (p99 >> 16) & 0xFF;
    let g99 = (p99 >> 8) & 0xFF;

    assert!(r99 >= 250, "Pixel 99 R should be ~252");
    assert!(g99 <= 5, "Pixel 99 G should be ~2");
}
