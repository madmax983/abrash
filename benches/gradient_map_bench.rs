use abrash_core::framebuffer::Framebuffer;
use abrash_core::gradient::Gradient;
use abrash_render::experimental::gradient_map::apply_gradient_map;
use criterion::{Criterion, black_box, criterion_group, criterion_main};

fn bench_gradient_map(c: &mut Criterion) {
    let mut fb = Framebuffer::new(1920, 1080).unwrap();
    let gradient = Gradient::heat();

    c.bench_function("gradient_map_1080p", |b| {
        b.iter(|| {
            apply_gradient_map(black_box(&mut fb), black_box(&gradient));
        });
    });
}

criterion_group!(benches, bench_gradient_map);
criterion_main!(benches);
