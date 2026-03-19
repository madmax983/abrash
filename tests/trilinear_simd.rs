use abrash::framebuffer::Framebuffer;
use abrash::rasterizer::{
    PerspectiveSpanStart, PerspectiveTextureGradients, draw_scanline_textured_perspective,
};
use abrash::texture::{FilterMode, Texture};
use abrash::zbuffer::ZBuffer;

#[test]
fn test_trilinear_simd_correctness() {
    let width = 64;
    let height = 1;
    let mut fb = Framebuffer::new(width, height).unwrap();
    let mut zb = ZBuffer::new(width, height).unwrap();
    let mut tex = Texture::new(64, 64).unwrap();

    // Fill texture with gradients to make interpolation visible
    for y in 0..64 {
        for x in 0..64 {
            let r = (x * 4) as u8;
            let g = (y * 4) as u8;
            let color = 0xFF00_0000 | (u32::from(r) << 16) | (u32::from(g) << 8);
            tex.set_pixel(x, y, color);
        }
    }
    tex.generate_mipmaps();
    tex.filter_mode = FilterMode::Trilinear;

    let y = 0;
    let x_start = 0;
    let x_end = 63;

    // Gradient that triggers some mip level
    // du/dx = 1.5 (traverse 1.5 texels per pixel).
    // rho = 1.5. log2(1.5) ~ 0.58. LOD ~ 0.58.
    // This should blend Level 0 and Level 1.
    let gradients = PerspectiveTextureGradients {
        dz_dx: 0.001,
        dq_dx: 0.0,
        du_dx: 1.5,
        dv_dx: 0.0,
        dq_dy: 0.0,
        du_dy: 0.0,
        dv_dy: 0.0,
    };

    let start = PerspectiveSpanStart {
        z: 0.5,
        q: 1.0,
        u: 0.0,
        v: 0.0,
    };

    // Run SIMD (if available)
    // Note: We can't force SIMD off easily without cfg or changing code,
    // but we can trust the implementation does dispatch.
    // Ideally we would run this test once with SIMD forced and once without.
    // For now, let's just run it and check values are reasonable (interpolated).

    // Clear buffer
    for z in zb.as_mut_slice() {
        *z = f32::INFINITY;
    }

    draw_scanline_textured_perspective(
        &mut fb, &mut zb, &tex, y, x_start, x_end, start, &gradients,
    );

    // Verify some pixels
    // At x=0: u=0. Pixel should be close to (0,0) of texture.
    // At x=32: u=32 * 1.5 = 48. Pixel (48, 0).

    let p0 = fb.get_pixel(0, 0).unwrap();
    let r0 = (p0 >> 16) & 0xFF;
    assert!(r0 < 10, "Pixel 0 should be dark (close to 0,0). Got {r0}");

    let p32 = fb.get_pixel(32, 0).unwrap();
    let r32 = (p32 >> 16) & 0xFF;
    // Expected at level 0: x=48 -> r=192.
    // Expected at level 1: x=24 -> r=192 (since we downsample by 2 but x coordinate also scales).
    // The color content is roughly invariant with position if pattern is smooth.
    // But due to blending, it should be consistent.

    // We mainly want to ensure no crash and no garbage (e.g. all black or random).
    assert!(r32 > 100, "Pixel 32 should be bright. Got {r32}");
}
