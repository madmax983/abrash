use abrash_core::framebuffer::Framebuffer;
use abrash_render::experimental::cctv::{CctvConfig, apply_cctv};
use criterion::{Criterion, criterion_group, criterion_main};

fn cctv_benchmark(c: &mut Criterion) {
    let mut fb = Framebuffer::new(800, 600).unwrap();
    fb.clear(0xFFFF0000);

    let config = CctvConfig::default();

    c.bench_function("cctv_800x600", |b| {
        b.iter(|| {
            apply_cctv(&mut fb, &config);
        });
    });
}

criterion_group!(benches, cctv_benchmark);
criterion_main!(benches);
