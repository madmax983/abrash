use abrash_core::framebuffer::Framebuffer;
use abrash_render::experimental::julia_set::{JuliaSetConfig, render_julia_set};
use criterion::{Criterion, black_box, criterion_group, criterion_main};

fn bench_julia_set(c: &mut Criterion) {
    let mut fb = Framebuffer::new(800, 600).unwrap();
    let config = JuliaSetConfig::default();

    c.bench_function("julia_set_800x600", |b| {
        b.iter(|| {
            render_julia_set(black_box(&mut fb), black_box(&config));
        });
    });
}

criterion_group!(benches, bench_julia_set);
criterion_main!(benches);
