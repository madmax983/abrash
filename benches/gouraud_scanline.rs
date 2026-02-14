use abrash::framebuffer::Framebuffer;
use abrash::rasterizer::draw_scanline_gouraud;
use abrash::zbuffer::ZBuffer;
use criterion::{Criterion, black_box, criterion_group, criterion_main};

fn bench_draw_scanline_gouraud_long(c: &mut Criterion) {
    let width = 1200;
    let height = 100;
    let mut fb = Framebuffer::new(width, height).unwrap();
    let mut zb = ZBuffer::new(width, height).unwrap();
    let y = 50;
    let x_start = 10;
    let x_end = 1010; // 1000 pixels
    let z_start = 0.5;
    let dz_dx = 0.001;

    // Fixed point color setup
    // Start color: Red (255, 0, 0) -> (255 << 16, 0, 0)
    let c_start = (255i64 << 16, 0, 0);

    // Color delta: Fade to Blue over 1000 pixels
    // End color: Blue (0, 0, 255)
    // Delta per pixel: (-255/1000, 0, 255/1000)
    // 255 << 16 = 16711680
    // 16711680 / 1000 = 16711
    let dc_dx = (-16711, 0, 16711);

    c.bench_function("draw_scanline_gouraud_1000px", |b| {
        b.iter(|| {
            let idx_start = (y as usize) * (width as usize) + (x_start as usize);
            let idx_end = (y as usize) * (width as usize) + (x_end as usize);
            for z in &mut zb.as_mut_slice()[idx_start..=idx_end] {
                *z = f32::INFINITY;
            }

            draw_scanline_gouraud(
                &mut fb,
                &mut zb,
                black_box(y),
                black_box(x_start),
                black_box(x_end),
                black_box(z_start),
                black_box(c_start),
                black_box(dz_dx),
                black_box(dc_dx),
            );
        });
    });
}

criterion_group!(benches, bench_draw_scanline_gouraud_long);
criterion_main!(benches);
