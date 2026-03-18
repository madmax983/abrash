use abrash_core::framebuffer::Framebuffer;
use abrash_core::zbuffer::ZBuffer;
use abrash_render::experimental::vision::{VisionConfig, apply_vision};
use criterion::{Criterion, criterion_group, criterion_main};

fn bench_vignette(c: &mut Criterion) {
    let mut fb = Framebuffer::new(1920, 1080).unwrap();
    let zb = ZBuffer::new(1920, 1080).unwrap();
    fb.clear(0xFF_AA_BB_CC);
    let config = VisionConfig::default();

    c.bench_function("apply_vignette 1080p", |b| {
        b.iter(|| {
            apply_vision(&mut fb, &zb, &config);
        });
    });
}

criterion_group!(benches, bench_vignette);
criterion_main!(benches);
