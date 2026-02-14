
use abrash::framebuffer::Framebuffer;
use abrash::zbuffer::ZBuffer;
use abrash::rasterizer::draw_scanline_flat;

#[test]
fn test_havoc_scanline_oob() {
    let width = 100;
    let height = 100;
    let mut fb = Framebuffer::new(width, height).unwrap();
    let mut zb = ZBuffer::new(width, height).unwrap();

    // Trigger OOB access by passing Y >= height
    // This should panic safely if bounds checks are present,
    // but might segfault if unsafe unchecked indexing is used.
    // The current implementation assumes caller checks Y.

    let y_oob = 200;

    // This call is "safe" Rust code but might cause UB internally
    draw_scanline_flat(
        &mut fb,
        &mut zb,
        y_oob,
        0,
        50,
        0.5,
        0.0,
        0xFFFF0000
    );
}
