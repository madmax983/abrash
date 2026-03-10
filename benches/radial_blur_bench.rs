use abrash::experimental::radial_blur::apply_radial_blur;
use abrash::framebuffer::Framebuffer;
use criterion::{Criterion, black_box, criterion_group, criterion_main};

fn bench_radial_blur(c: &mut Criterion) {
    let mut fb = Framebuffer::new(1024, 1024).unwrap();

    // Fill with a checkerboard pattern
    for y in 0..1024 {
        for x in 0..1024 {
            let color = if (x + y) % 2 == 0 { 0xFFFFFF } else { 0x000000 };
            fb.set_pixel(x, y, color);
        }
    }

    c.bench_function("radial_blur_1024x1024", |b| {
        b.iter(|| {
            apply_radial_blur(
                black_box(&mut fb),
                black_box(512),
                black_box(512),
                black_box(0.5),
                black_box(16),
            );
        });
    });
}

criterion_group!(benches, bench_radial_blur);
criterion_main!(benches);
