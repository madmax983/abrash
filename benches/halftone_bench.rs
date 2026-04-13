use abrash_core::framebuffer::Framebuffer;
use abrash_render::experimental::halftone::apply_halftone;
use criterion::{Criterion, black_box, criterion_group, criterion_main};

fn bench_halftone(c: &mut Criterion) {
    let mut fb = Framebuffer::new(800, 600).unwrap();
    let dot_size = 5.0;
    let angle_radians = std::f32::consts::PI / 4.0;

    c.bench_function("halftone_800x600", |b| {
        b.iter(|| {
            apply_halftone(
                black_box(&mut fb),
                black_box(dot_size),
                black_box(angle_radians),
            );
        });
    });
}

criterion_group!(benches, bench_halftone);
criterion_main!(benches);
