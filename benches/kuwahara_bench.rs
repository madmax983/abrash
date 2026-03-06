use abrash::experimental::kuwahara::apply_kuwahara;
use abrash::framebuffer::Framebuffer;
use criterion::{Criterion, criterion_group, criterion_main};

fn bench_kuwahara(c: &mut Criterion) {
    let mut fb = Framebuffer::new(800, 600).unwrap();
    // Fill with some pattern
    for y in 0..600 {
        for x in 0..800 {
            let r = (x % 255) as u32;
            let g = (y % 255) as u32;
            unsafe {
                fb.set_pixel_unchecked(x, y, 0xFF000000 | (r << 16) | (g << 8));
            }
        }
    }

    let mut group = c.benchmark_group("kuwahara_filter");

    group.bench_function("radius_1", |b| {
        b.iter(|| {
            apply_kuwahara(&mut fb, 1);
        });
    });

    group.bench_function("radius_3", |b| {
        b.iter(|| {
            apply_kuwahara(&mut fb, 3);
        });
    });

    group.bench_function("radius_5", |b| {
        b.iter(|| {
            apply_kuwahara(&mut fb, 5);
        });
    });

    group.finish();
}

criterion_group!(benches, bench_kuwahara);
criterion_main!(benches);
