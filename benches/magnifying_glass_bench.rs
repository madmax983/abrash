use abrash::experimental::magnifying_glass::{MagnifyingGlassConfig, apply_magnifying_glass};
use abrash::framebuffer::Framebuffer;
use criterion::{Criterion, black_box, criterion_group, criterion_main};

fn bench_magnifying_glass(c: &mut Criterion) {
    let mut fb = Framebuffer::new(1024, 1024).unwrap();

    // Fill with a checkerboard pattern
    for y in 0..1024 {
        for x in 0..1024 {
            let color = if (x + y) % 2 == 0 {
                0xFF_FF_FF_FF
            } else {
                0xFF_00_00_00
            };
            fb.set_pixel(x, y, color);
        }
    }

    let config = MagnifyingGlassConfig {
        center_x: 512,
        center_y: 512,
        radius: 400,
        magnification: 2.0,
        border_thickness: 10,
        border_color: 0xFF_55_55_55,
    };

    c.bench_function("magnifying_glass_1024x1024", |b| {
        b.iter(|| {
            apply_magnifying_glass(black_box(&mut fb), black_box(&config));
        });
    });
}

criterion_group!(benches, bench_magnifying_glass);
criterion_main!(benches);
