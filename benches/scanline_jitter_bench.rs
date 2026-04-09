use abrash_core::framebuffer::Framebuffer;
use abrash_render::post_process::filters::{apply_scanline_jitter, ScanlineJitterConfig};
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn bench_scanline_jitter(c: &mut Criterion) {
    let mut fb = Framebuffer::new(1920, 1080).unwrap();
    let config = ScanlineJitterConfig { intensity: 10 };
    c.bench_function("scanline_jitter_1080p", |b| {
        b.iter(|| {
            apply_scanline_jitter(black_box(&mut fb), black_box(&config));
        });
    });
}

criterion_group!(benches, bench_scanline_jitter);
criterion_main!(benches);