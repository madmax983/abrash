use abrash::framebuffer::Framebuffer;
use abrash::rasterizer::draw_scanline_gouraud;
use abrash::zbuffer::ZBuffer;
use criterion::{Criterion, black_box, criterion_group, criterion_main};

fn bench_draw_scanline_gouraud_short(c: &mut Criterion) {
    let width = 100;
    let height = 100;
    let mut fb = Framebuffer::new(width, height).unwrap();
    let mut zb = ZBuffer::new(width, height).unwrap();
    let y = 50;
    let x_start = 10;
    let x_end = 26; // 16 pixels
    let z_start = 0.5;
    let dz_dx = 0.001;

    // Gradient Red -> Green over 16 pixels
    // Start: (255, 0, 0)
    // End: (0, 255, 0)
    // Delta: (-255, 255, 0) per 16 pixels -> (-15.9, 15.9, 0) per pixel
    let c_start = (255i64 << 16, 0, 0);
    let dc_dx = ((-15i32) << 16, 15i32 << 16, 0);

    c.bench_function("draw_scanline_gouraud_16px", |b| {
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

fn bench_draw_scanline_gouraud_medium(c: &mut Criterion) {
    let width = 200;
    let height = 100;
    let mut fb = Framebuffer::new(width, height).unwrap();
    let mut zb = ZBuffer::new(width, height).unwrap();
    let y = 50;
    let x_start = 10;
    let x_end = 110; // 100 pixels
    let z_start = 0.5;
    let dz_dx = 0.001;

    // Gradient Red -> Green over 100 pixels
    let c_start = (255i64 << 16, 0, 0);
    // 255/100 = 2.55 -> ~2 << 16 (approx)
    let dc_dx = ((-2i32) << 16, 2i32 << 16, 0);

    c.bench_function("draw_scanline_gouraud_100px", |b| {
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

fn bench_draw_scanline_gouraud_long(c: &mut Criterion) {
    let width = 1200;
    let height = 100;
    let mut fb = Framebuffer::new(width, height).unwrap();
    let mut zb = ZBuffer::new(width, height).unwrap();
    let y = 50;
    let x_start = 10;
    let x_end = 1010; // 1000 pixels
    let z_start = 0.5;
    let dz_dx = 0.0001;

    // Gradient Red -> Green over 1000 pixels
    let c_start = (255i64 << 16, 0, 0);
    // 255/1000 = 0.255 -> ~0.25 * 65536 = 16384
    let val = (0.255 * 65536.0) as i32;
    let dc_dx = (-val, val, 0);

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

criterion_group!(
    benches,
    bench_draw_scanline_gouraud_short,
    bench_draw_scanline_gouraud_medium,
    bench_draw_scanline_gouraud_long,
);
criterion_main!(benches);
