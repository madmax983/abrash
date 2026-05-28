use abrash_core::framebuffer::Framebuffer;
use abrash_render::experimental::infinite_grid::{InfiniteGridConfig, apply_infinite_grid};
use criterion::{Criterion, criterion_group, criterion_main};

fn bench_infinite_grid(c: &mut Criterion) {
    let mut group = c.benchmark_group("Infinite Grid");
    let mut fb = Framebuffer::new(800, 600).unwrap();
    let config = InfiniteGridConfig::default();

    group.bench_function("800x600", |b| {
        b.iter(|| {
            apply_infinite_grid(&mut fb, &config);
        });
    });

    group.finish();
}

criterion_group!(benches, bench_infinite_grid);
criterion_main!(benches);
