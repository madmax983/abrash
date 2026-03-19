use abrash::framebuffer::Framebuffer;
use abrash::rasterizer::{draw_scanline_flat, draw_scanline_flat_blended};
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
    let color = 0xFFFF_0000;

    c.bench_function("draw_scanline_flat_16px", |b| {
        b.iter(|| {
            // No need to clear buffers for this bench as we overwrite anyway
            // and we want to measure throughput, not allocation/clearing.
            // Z-test will fail if we don't clear or reset z, but that's fine,
            // we want to measure the loop overhead even if it doesn't write.
            // Wait, if Z-test fails, it skips writing. We want it to pass.
            // Let's reset Z for the span.
            // Actually, clearing the whole buffer is too slow.
            // We can just ensure z_start is always less than what's there.
            // Or we can just let it overwrite.
            // To ensure we measure the WRITE path, we need z < depth.
            // If we don't clear, after first iter, depth will be updated.
            // So subsequent iters will fail z-test if we use same z.
            // Let's use a fresh buffer every batch or clear only the line?
            // Clearing only the line is better.

            // For micro-benchmarking scanline, we can just assume Z-test passes
            // by setting z very small? But z-buffer stores the small value.
            // We can alternate z?

            // Simplest: just clear the specific slice in zb manually?
            // Or just measure "mostly z-fail" or "mostly z-pass".
            // Ideally "z-pass" because that's the expensive path.

            // We can reset the Z-buffer slice in the loop? That might dominate the benchmark.
            // Let's rely on `zb.clear()` but move it outside if possible? No.
            // Let's just use `black_box` inputs and hopefully `zb.clear()` is fast enough for small buffers?
            // Or we can use a very large buffer and draw different lines? No.

            // Re-initializing ZBuffer slice is probably fast for 16 pixels.
            let idx_start = (y as usize) * (width as usize) + (x_start as usize);
            let idx_end = (y as usize) * (width as usize) + (x_end as usize);
            // Reset Z to infinity
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

fn bench_draw_scanline_flat_medium(c: &mut Criterion) {
    let width = 200;
    let height = 100;
    let mut fb = Framebuffer::new(width, height).unwrap();
    let mut zb = ZBuffer::new(width, height).unwrap();
    let y = 50;
    let x_start = 10;
    let x_end = 110; // 100 pixels
    let z_start = 0.5;
    let dz_dx = 0.001;
    let color = 0xFFFF_0000;

    c.bench_function("draw_scanline_flat_100px", |b| {
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
    let dz_dx = 0.001;
    let color = 0xFFFF_0000;

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

fn bench_draw_scanline_flat_blended_medium(c: &mut Criterion) {
    let width = 200;
    let height = 100;
    let mut fb = Framebuffer::new(width, height).unwrap();
    let mut zb = ZBuffer::new(width, height).unwrap();
    let y = 50;
    let x_start = 10;
    let x_end = 110; // 100 pixels
    let z_start = 0.5;
    let dz_dx = 0.001;
    let color = 0x80FF_0000; // 50% Alpha

    c.bench_function("draw_scanline_flat_blended_100px", |b| {
        b.iter(|| {
            let idx_start = (y as usize) * (width as usize) + (x_start as usize);
            let idx_end = (y as usize) * (width as usize) + (x_end as usize);
            for z in &mut zb.as_mut_slice()[idx_start..=idx_end] {
                *z = f32::INFINITY;
            }

            draw_scanline_flat_blended(
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
    bench_draw_scanline_flat_medium,
    bench_draw_scanline_flat_long,
    bench_draw_scanline_flat_blended_medium,
);
criterion_main!(benches);
