use abrash::framebuffer::Framebuffer;
use abrash::math::{Vec2, Vec3};
use abrash::rasterizer::fill_triangle_textured;
use abrash::texture::Texture;
use abrash::zbuffer::ZBuffer;
use std::time::Instant;

#[test]
fn verify_texture_culling() {
    let width = 100;
    let height = 100;
    let mut fb = Framebuffer::new(width, height).unwrap();
    let mut zb = ZBuffer::new(width, height).unwrap();
    let texture = Texture::new(16, 16).unwrap();

    // Vertices in Clip Space (already projected for this test context? No, fill_triangle_textured takes Clip Space)
    // Actually fill_triangle_textured takes (Vec3, f32) which is clip space.
    // It calls project_triangle_to_screen.

    // CCW is front-facing in this engine.
    // So CW is back-facing.

    // CW Triangle: (0,0), (1,0), (0,1)
    // Wait, screen coordinates: Y is down?
    // Project to screen:
    // Screen X = (x/w + 1) * w/2
    // Screen Y = (1 - y/w) * h/2  <-- Y flip

    // Clip Space:
    // v0: (0, 0, 0)
    // v1: (1, 0, 0)
    // v2: (0, 1, 0)
    // All w=1.

    // NDC:
    // v0: (0, 0)
    // v1: (1, 0)
    // v2: (0, 1)

    // Screen (100x100):
    // v0: (0.5 * 100, 0.5 * 100) = (50, 50)
    // v1: (1.0 * 100, 0.5 * 100) = (100, 50)
    // v2: (0.5 * 100, 0.0 * 100) = (50, 0)

    // Triangle: (50, 50) -> (100, 50) -> (50, 0)
    // Vector 0->1: (50, 0)
    // Vector 0->2: (0, -50)
    // Cross Product (2D): x1*y2 - x2*y1 = 50*(-50) - 0*0 = -2500.
    // nz < 0.
    // is_backface checks nz >= 0.
    // So nz < 0 means Front Facing (kept).

    // Let's make a CW triangle (Back Facing).
    // v0: (0,0)
    // v1: (0,1)
    // v2: (1,0)
    // Screen:
    // v0: (50, 50)
    // v1: (50, 0)
    // v2: (100, 50)
    // 0->1: (0, -50)
    // 0->2: (50, 0)
    // Cross: 0*0 - 50*(-50) = 2500.
    // nz > 0. Back Facing. Culled.

    // Front Facing (CCW in Clip Space, becomes CW in Screen Space due to Y-flip? No.)
    // Standard GL: CCW is front.
    // Let's verify `is_backface` logic.
    // ux*vy - uy*vx >= 0 -> Backface.
    // If Result > 0, it's backface.
    // My calculation for "CW" triangle gave 2500 (> 0). So it IS backface.
    // So (0,0)->(0,1)->(1,0) in Clip Space is Back Facing.

    let v0 = ((Vec3::new(0.0, 0.0, 0.0), 1.0), Vec2::new(0.0, 0.0));
    let v1_cw = ((Vec3::new(0.0, 1.0, 0.0), 1.0), Vec2::new(0.0, 1.0));
    let v2_cw = ((Vec3::new(1.0, 0.0, 0.0), 1.0), Vec2::new(1.0, 0.0));

    // Test Back Face (Should NOT draw)
    fb.clear(0);
    fill_triangle_textured(&mut fb, &mut zb, v0, v1_cw, v2_cw, &texture);
    let pixels_drawn_cw = fb.as_slice().iter().filter(|&&p| p != 0).count();
    assert_eq!(pixels_drawn_cw, 0, "Back-facing triangle should be culled");

    // Test Front Face (Should draw)
    // Swap v1/v2
    let v1_ccw = v2_cw;
    let v2_ccw = v1_cw;

    fb.clear(0);
    zb.clear();
    fill_triangle_textured(&mut fb, &mut zb, v0, v1_ccw, v2_ccw, &texture);
    let pixels_drawn_ccw = fb.as_slice().iter().filter(|&&p| p != 0).count();
    assert!(pixels_drawn_ccw > 0, "Front-facing triangle should be drawn");

    // Benchmark Loop
    let start = Instant::now();
    for _ in 0..10000 {
        // Back face culling check (hot path)
        fill_triangle_textured(&mut fb, &mut zb, v0, v1_cw, v2_cw, &texture);
    }
    let duration = start.elapsed();
    println!("10k Backface Culls took: {:?}", duration);
}
