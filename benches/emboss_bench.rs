use abrash::framebuffer::Framebuffer;
use abrash::experimental::emboss;
use criterion::{Criterion, black_box, criterion_group, criterion_main};

fn benchmark_emboss(c: &mut Criterion) {
    let width = 1920;
    let height = 1080;
    let mut fb = Framebuffer::new(width, height).unwrap();

    // Fill with a checkerboard pattern
    for y in 0..height {
        for x in 0..width {
            let color = if (x / 50 + y / 50) % 2 == 0 {
                0xFFFFFFFF
            } else {
                0xFF000000
            };
            fb.set_pixel(x as i32, y as i32, color);
        }
    }

    c.bench_function("apply_emboss 1080p", |b| {
        b.iter(|| {
            emboss::apply_emboss(black_box(&mut fb));
        });
    });
}

criterion_group!(benches, benchmark_emboss);
criterion_main!(benches);
