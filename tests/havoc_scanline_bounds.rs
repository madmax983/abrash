use abrash::framebuffer::Framebuffer;
use abrash::rasterizer::draw_scanline_flat;
use abrash::rasterizer::gouraud::draw_scanline_gouraud;
use abrash::zbuffer::ZBuffer;

#[test]
#[ignore = "👺 Havoc: Triggers OOB indexing if the renderer doesn't clamp coordinates"]
fn fuzz_scanline_oob() {
    let w = 100u32;
    let h = 100u32;
    let y = 1000i32; // Way out of bounds
    let x0 = 0i32;
    let x1 = 10i32;
    let z0 = 0.0f32;
    let z_step = 0.0f32;

    let mut fb = Framebuffer::new(w, h).unwrap();
    let mut zb = ZBuffer::new(w, h).unwrap();
    let c0 = (0, 0, 0);
    let c1 = (1, 1, 1);

    // In release mode or without debug assertions, get_unchecked_mut causes a segfault.
    let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        draw_scanline_flat(&mut fb, &mut zb, y, x0, x1, z0, z_step, 0xFFFFFFFF);
        draw_scanline_gouraud(&mut fb, &mut zb, y, x0, x1, z0, c0, z_step, c1);
    }));
}
