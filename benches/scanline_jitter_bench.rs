use abrash_core::framebuffer::Framebuffer;
use abrash_render::experimental::scanline_jitter::{ScanlineJitterConfig, apply_scanline_jitter};
use criterion::{Criterion, black_box, criterion_group, criterion_main};

fn bench_scanline_jitter(c: &mut Criterion) {
    let mut fb = Framebuffer::new(800, 600).unwrap();
    fb.clear(0xFF_FFFFFF);

    let config = ScanlineJitterConfig {
        max_shift: 10,
        probability: 0.1,
        seed: 42,
    };

    c.bench_function("scanline_jitter_800x600", |b| {
        b.iter(|| {
            apply_scanline_jitter(black_box(&mut fb), black_box(&config));
        });
    });
}

criterion_group!(benches, bench_scanline_jitter);
criterion_main!(benches);
