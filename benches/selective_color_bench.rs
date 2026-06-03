use abrash_core::framebuffer::Framebuffer;
use abrash_render::experimental::selective_color::{SelectiveColorConfig, apply_selective_color};
use criterion::{Criterion, black_box, criterion_group, criterion_main};

fn benchmark_selective_color(c: &mut Criterion) {
    let mut fb = Framebuffer::new(1920, 1080).unwrap();
    let config = SelectiveColorConfig::default();

    c.bench_function("selective_color_1080p", |b| {
        b.iter(|| apply_selective_color(black_box(&mut fb), black_box(&config)));
    });
}

criterion_group!(benches, benchmark_selective_color);
criterion_main!(benches);
