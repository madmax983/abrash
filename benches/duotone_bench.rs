use abrash_core::framebuffer::Framebuffer;
use abrash_render::experimental::duotone::apply_duotone;
use criterion::{Criterion, black_box, criterion_group, criterion_main};

fn bench_duotone(c: &mut Criterion) {
    let mut fb = Framebuffer::new(1920, 1080).unwrap();

    c.bench_function("duotone_1080p", |b| {
        b.iter(|| {
            apply_duotone(
                black_box(&mut fb),
                black_box(0xFFFF0000),
                black_box(0xFF0000FF),
            );
        });
    });
}

criterion_group!(benches, bench_duotone);
criterion_main!(benches);
