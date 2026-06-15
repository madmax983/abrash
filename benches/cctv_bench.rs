use abrash_core::framebuffer::Framebuffer;
use abrash_render::experimental::cctv::{CctvConfig, apply_cctv};
use criterion::{Criterion, black_box, criterion_group, criterion_main};

fn cctv_benchmark(c: &mut Criterion) {
    let mut fb = Framebuffer::new(800, 600).unwrap();
    fb.clear(0xFFFF_FFFF);
    let mut config = CctvConfig::default();

    c.bench_function("apply_cctv_800x600", |b| {
        b.iter(|| {
            config.time += 0.01;
            apply_cctv(black_box(&mut fb), black_box(&config));
        });
    });
}

criterion_group!(benches, cctv_benchmark);
criterion_main!(benches);
