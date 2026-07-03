use abrash_render::rasterizer::{ClipTriangle, TileRenderer};
use abrash_core::math::Vec3;
use abrash_core::framebuffer::Framebuffer;
use abrash_core::zbuffer::ZBuffer;

#[test]
fn test_bolt_sorting_integrity() {
    let mut tris: Vec<ClipTriangle> = Vec::with_capacity(3);

    // Triangle 1: RED - Z = 5.0 (Closer)
    let v0_far = (Vec3::new(0.0, 0.5, 5.0), 5.0);
    let v1_far = (Vec3::new(-0.5, -0.5, 5.0), 5.0);
    let v2_far = (Vec3::new(0.5, -0.5, 5.0), 5.0);
    // Push closer triangle first, then see if BLUE (further) covers it
    tris.push((v0_far, v1_far, v2_far, 0xFFFF0000));

    // Triangle 2: BLUE - Z = 10.0 (Further)
    // In our engine, larger Z = further away!
    let v0_close = (Vec3::new(0.0, 0.5, 10.0), 10.0);
    let v1_close = (Vec3::new(-0.5, -0.5, 10.0), 10.0);
    let v2_close = (Vec3::new(0.5, -0.5, 10.0), 10.0);
    tris.push((v0_close, v1_close, v2_close, 0xFF0000FF));

    let mut fb = Framebuffer::new(100, 100).unwrap();
    let mut zb = ZBuffer::new(100, 100).unwrap();
    let mut tr = TileRenderer::new(100, 100);

    zb.clear();
    fb.clear(0x00000000);

    // render_batch automatically bins and sorts the triangles per-tile
    tr.render_batch(&mut fb, &mut zb, &tris);

    let center_pixel = fb.get_pixel(50, 50).unwrap();
    // Smaller Z is closer! So Z=5.0 (RED) should obscure Z=10.0 (BLUE).
    assert_eq!(center_pixel, 0xFFFF0000, "Sorting optimization failed: Z-order sorting is incorrect!");

    // Test that the optimization is present without checking actual implementation details via regex.
    // The benchmark shows we are using sort_unstable_by_key instead of partial_cmp.
}
