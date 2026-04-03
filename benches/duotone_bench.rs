use abrash_core::framebuffer::Framebuffer;
use abrash_render::experimental::duotone::Duotone;
use criterion::{Criterion, black_box, criterion_group, criterion_main};

fn criterion_benchmark(c: &mut Criterion) {
    let mut fb = Framebuffer::new(800, 600).unwrap();
    let filter = Duotone::new(0xFF00_00FF, 0xFFFF_0000);

    c.bench_function("duotone_800x600", |b| {
        b.iter(|| {
            filter.apply(black_box(&mut fb));
        });
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
