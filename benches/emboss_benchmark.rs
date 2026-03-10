use abrash::framebuffer::Framebuffer;
use abrash::post_process::apply_emboss;
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn bench_apply_emboss(c: &mut Criterion) {
    let mut group = c.benchmark_group("emboss_filter");

    // Test on a typical 1080p resolution
    let width = 1920;
    let height = 1080;
    let mut fb = Framebuffer::new(width, height).unwrap();
    fb.clear(0xFF_80_80_80);

    // Setup some basic pattern
    for y in 0..height {
        for x in 0..width {
            if (x / 10) % 2 == 0 {
                unsafe { fb.set_pixel_unchecked(x as usize, y as usize, 0xFF_FF_FF_FF) };
            }
        }
    }

    group.bench_function("1920x1080_emboss", |b| {
        b.iter(|| {
            apply_emboss(black_box(&mut fb));
        })
    });
    group.finish();
}

criterion_group!(benches, bench_apply_emboss);
criterion_main!(benches);
