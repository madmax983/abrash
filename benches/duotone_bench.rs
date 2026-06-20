use abrash::framebuffer::Framebuffer;
use abrash_render::experimental::duotone::apply_duotone;
use criterion::{Criterion, black_box, criterion_group, criterion_main};

fn bench_duotone(c: &mut Criterion) {
    let mut fb = Framebuffer::new(1920, 1080).unwrap();
    // Fill with some data
    for p in fb.as_mut_slice() {
        *p = 0xFF_123456;
    }
    c.bench_function("apply_duotone 1080p", |b| {
        b.iter(|| {
            apply_duotone(
                black_box(&mut fb),
                black_box(0xFFFF_0000),
                black_box(0xFF00_00FF),
            );
        });
    });
}

criterion_group!(benches, bench_duotone);
criterion_main!(benches);
