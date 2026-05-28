use criterion::{Criterion, black_box, criterion_group, criterion_main};

use abrash_core::framebuffer::Framebuffer;
use abrash_core::math::Vec3;
use abrash_core::zbuffer::ZBuffer;
use abrash_render::rasterizer::line::draw_line_3d;

fn bench_draw_line_3d(c: &mut Criterion) {
    let mut fb = Framebuffer::new(800, 600).unwrap();
    let mut zb = ZBuffer::new(800, 600).unwrap();

    let color = 0xFFFF_FFFF;
    // Draw a long diagonal line across the screen to test Bresenham loop performance
    let v0 = (Vec3::new(-1.0, -1.0, 5.0), 5.0);
    let v1 = (Vec3::new(1.0, 1.0, 5.0), 5.0);

    c.bench_function("draw_line_3d_diagonal", |b| {
        b.iter(|| {
            // Re-initialize ZBuffer just in the line path to ensure Z-test passes?
            // Actually, for pure performance comparison of the inner loop, we can just clear it.
            // Or let it fail/pass. It's fine to just measure the rasterization overhead.
            draw_line_3d(
                &mut fb,
                &mut zb,
                black_box(v0),
                black_box(v1),
                black_box(color),
            );
        });
    });
}

criterion_group!(benches, bench_draw_line_3d);
criterion_main!(benches);
