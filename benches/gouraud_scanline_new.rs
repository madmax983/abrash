use abrash::framebuffer::Framebuffer;
use abrash::rasterizer::{FIXED_SCALE, draw_scanline_gouraud};
use abrash::zbuffer::ZBuffer;
use criterion::{Criterion, black_box, criterion_group, criterion_main};

fn bench_draw_scanline_gouraud_100px(c: &mut Criterion) {
    let width = 200;
    let height = 100;
    let mut fb = Framebuffer::new(width, height).unwrap();
    let mut zb = ZBuffer::new(width, height).unwrap();
    let y = 50;
    let x_start = 10;
    let x_end = 110; // 100 pixels
    let z_start = 0.5;
    let dz_dx = 0.001;

    // Red (255, 0, 0) to Green (0, 255, 0)
    let c_start = ((255.0 * FIXED_SCALE) as i64, 0, 0);

    let dc_dx = (
        ((-255.0 * FIXED_SCALE) / 100.0) as i32,
        ((255.0 * FIXED_SCALE) / 100.0) as i32,
        0,
    );

    c.bench_function("draw_scanline_gouraud_100px", |b| {
        b.iter(|| {
            // Reset Z-buffer for the scanline to ensure Z-test passes
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

criterion_group!(benches, bench_draw_scanline_gouraud_100px,);
criterion_main!(benches);
