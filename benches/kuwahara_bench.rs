use abrash::experimental::kuwahara::apply_kuwahara;
use abrash::framebuffer::Framebuffer;
use criterion::{Criterion, black_box, criterion_group, criterion_main};

fn benchmark_kuwahara(c: &mut Criterion) {
    let width = 640;
    let height = 480;
    let mut fb = Framebuffer::new(width, height).unwrap();

    // Fill with a gradient pattern to avoid zeroing optimizations
    for y in 0..height {
        for x in 0..width {
            let color = 0xFF000000 | (x & 0xFF) << 16 | (y & 0xFF) << 8 | ((x ^ y) & 0xFF);
            fb.set_pixel(x as i32, y as i32, color);
        }
    }

    let mut group = c.benchmark_group("kuwahara");

    group.bench_function("apply_kuwahara 640x480 (radius=2)", |b| {
        b.iter(|| {
            apply_kuwahara(black_box(&mut fb), black_box(2));
        });
    });

    group.bench_function("apply_kuwahara 640x480 (radius=4)", |b| {
        b.iter(|| {
            apply_kuwahara(black_box(&mut fb), black_box(4));
        });
    });

    group.finish();
}

criterion_group!(benches, benchmark_kuwahara);
criterion_main!(benches);
