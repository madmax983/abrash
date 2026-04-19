use abrash_core::framebuffer::Framebuffer;
use abrash_render::experimental::crt::apply_crt;
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn bench_crt_filter(c: &mut Criterion) {
    let mut group = c.benchmark_group("crt_filter");

    let mut fb = Framebuffer::new(800, 600).unwrap();
    fb.clear(0xFFFF_FFFF);

    group.bench_function("crt_800x600", |b| {
        b.iter(|| {
            apply_crt(black_box(&mut fb), black_box(0.2));
        });
    });

    group.finish();
}

criterion_group!(benches, bench_crt_filter);
criterion_main!(benches);
