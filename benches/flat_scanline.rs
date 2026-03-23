use abrash::framebuffer::Framebuffer;
use abrash::rasterizer::draw_scanline_flat;
use abrash::zbuffer::ZBuffer;
use criterion::{Criterion, black_box, criterion_group, criterion_main};

fn bench_draw_scanline_flat_short(c: &mut Criterion) {
    let width = 100;
    let height = 100;
    let mut fb = Framebuffer::new(width, height).unwrap();
    let mut zb = ZBuffer::new(width, height).unwrap();
    let y = 50;
    let x_start = 10;
    let x_end = 26; // 16 pixels
    let z_start = 0.5;
    let dz_dx = 0.001;

    let color = 0xFFFF_0000; // Red

    c.bench_function("draw_scanline_flat_16px", |b| {
        b.iter(|| {
            let idx_start = (y as usize) * (width as usize) + (x_start as usize);
            let idx_end = (y as usize) * (width as usize) + (x_end as usize);
            for z in &mut zb.as_mut_slice()[idx_start..=idx_end] {
                *z = f32::INFINITY;
            }

            draw_scanline_flat(
                &mut fb,
                &mut zb,
                black_box(y),
                black_box(x_start),
                black_box(x_end),
                black_box(z_start),
                black_box(dz_dx),
                black_box(color),
            );
        });
    });
}

fn bench_draw_scanline_flat_long(c: &mut Criterion) {
    let width = 1200;
    let height = 100;
    let mut fb = Framebuffer::new(width, height).unwrap();
    let mut zb = ZBuffer::new(width, height).unwrap();
    let y = 50;
    let x_start = 10;
    let x_end = 1010; // 1000 pixels
    let z_start = 0.5;
    let dz_dx = 0.0001;

    let color = 0xFFFF_0000; // Red

    c.bench_function("draw_scanline_flat_1000px", |b| {
        b.iter(|| {
            let idx_start = (y as usize) * (width as usize) + (x_start as usize);
            let idx_end = (y as usize) * (width as usize) + (x_end as usize);
            for z in &mut zb.as_mut_slice()[idx_start..=idx_end] {
                *z = f32::INFINITY;
            }

            draw_scanline_flat(
                &mut fb,
                &mut zb,
                black_box(y),
                black_box(x_start),
                black_box(x_end),
                black_box(z_start),
                black_box(dz_dx),
                black_box(color),
            );
        });
    });
}

criterion_group!(
    benches,
    bench_draw_scanline_flat_short,
    bench_draw_scanline_flat_long,
);
criterion_main!(benches);
