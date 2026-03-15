use abrash::experimental::vignette::{VignetteConfig, apply_vignette};
use abrash::framebuffer::Framebuffer;
use criterion::{Criterion, criterion_group, criterion_main};

fn bench_vignette(c: &mut Criterion) {
    let mut fb = Framebuffer::new(1920, 1080).unwrap();
    fb.clear(0xFF_AA_BB_CC);
    let config = VignetteConfig::default();

    c.bench_function("apply_vignette 1080p", |b| {
        b.iter(|| {
            apply_vignette(&mut fb, &config);
        });
    });
}

criterion_group!(benches, bench_vignette);
criterion_main!(benches);
