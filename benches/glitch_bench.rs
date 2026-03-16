use abrash::experimental::glitch::{GlitchParams, apply_glitch};
use abrash::framebuffer::Framebuffer;
use criterion::{Criterion, black_box, criterion_group, criterion_main};

fn bench_glitch(c: &mut Criterion) {
    let mut fb = Framebuffer::new(1024, 1024).unwrap();

    // Fill with a checkerboard pattern
    for y in 0..1024 {
        for x in 0..1024 {
            let color = if (x + y) % 2 == 0 { 0xFFFFFF } else { 0x000000 };
            fb.set_pixel(x, y, color);
        }
    }

    let config = GlitchParams {
        intensity: 1.0,
        seed_time: 12345,
        color_shift_amount: 10,
        jitter_amount: 20,
        block_displacement_prob: 0.2,
    };

    c.bench_function("glitch_1024x1024", |b| {
        b.iter(|| {
            apply_glitch(black_box(&mut fb), black_box(&config));
        });
    });
}

criterion_group!(benches, bench_glitch);
criterion_main!(benches);
