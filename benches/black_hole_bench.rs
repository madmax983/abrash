use abrash::experimental::black_hole::apply_black_hole;
use abrash::framebuffer::Framebuffer;
use criterion::{Criterion, black_box, criterion_group, criterion_main};

fn bench_apply_black_hole(c: &mut Criterion) {
    let mut fb = Framebuffer::new(1920, 1080).unwrap();
    fb.clear(0xFFFF_FFFF);

    let mut group = c.benchmark_group("black_hole");
    group.bench_function("apply_black_hole_1080p", |b| {
        b.iter(|| {
            apply_black_hole(
                black_box(&mut fb),
                black_box(960.0),
                black_box(540.0),
                black_box(100.0),
            );
        });
    });
    group.finish();
}

criterion_group!(benches, bench_apply_black_hole);
criterion_main!(benches);
