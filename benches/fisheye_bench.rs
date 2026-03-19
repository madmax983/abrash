use abrash::experimental::fisheye::{FisheyeConfig, apply_fisheye};
use abrash::framebuffer::Framebuffer;
use criterion::{Criterion, criterion_group, criterion_main};

fn bench_fisheye(c: &mut Criterion) {
    let mut fb = Framebuffer::new(1920, 1080).unwrap();
    fb.clear(0xFF_AA_BB_CC);
    let config = FisheyeConfig::default();

    c.bench_function("apply_fisheye 1080p", |b| {
        b.iter(|| {
            apply_fisheye(&mut fb, &config);
        });
    });
}

criterion_group!(benches, bench_fisheye);
criterion_main!(benches);
