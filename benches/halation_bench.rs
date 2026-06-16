use abrash_core::framebuffer::Framebuffer;
use abrash_render::experimental::halation::{HalationConfig, apply_halation};
use criterion::{Criterion, black_box, criterion_group, criterion_main};

fn benchmark_halation(c: &mut Criterion) {
    let width = 1920;
    let height = 1080;
    let mut fb = Framebuffer::new(width, height).unwrap();

    // Fill with a pattern: some bright spots
    for y in 0..height {
        for x in 0..width {
            if x % 100 == 0 && y % 100 == 0 {
                fb.set_pixel(x as i32, y as i32, 0xFFFF_FFFF);
            } else {
                fb.set_pixel(x as i32, y as i32, 0xFF10_1010);
            }
        }
    }

    let mut group = c.benchmark_group("halation");

    let config = HalationConfig {
        threshold: 0.8,
        radius: 8,
        intensity: 1.0,
        tint: (1.0, 0.3, 0.0),
    };

    group.bench_function("apply_halation_1080p", |b| {
        b.iter(|| {
            apply_halation(black_box(&mut fb), black_box(config));
        });
    });

    group.finish();
}

criterion_group!(benches, benchmark_halation);
criterion_main!(benches);
