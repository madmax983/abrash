use abrash::framebuffer::Framebuffer;
use abrash::post_process::filters::apply_barrel_distortion;
use criterion::{Criterion, black_box, criterion_group, criterion_main};

fn bench_barrel_distortion(c: &mut Criterion) {
    let mut fb = Framebuffer::new(1920, 1080).unwrap();

    // Fill with some data
    for y in 0..1080 {
        for x in 0..1920 {
            fb.set_pixel(x, y, if (x + y) % 2 == 0 { 0xFFFFFFFF } else { 0xFF000000 });
        }
    }

    c.bench_function("barrel_distortion_1080p", |b| {
        b.iter(|| {
            apply_barrel_distortion(black_box(&mut fb), black_box(0.5));
        });
    });
}

criterion_group!(benches, bench_barrel_distortion);
criterion_main!(benches);
