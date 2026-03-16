use criterion::{black_box, criterion_group, criterion_main, Criterion};
use abrash::framebuffer::Framebuffer;
use abrash::experimental::glitch::{apply_glitch, GlitchParams};

fn bench_glitch(c: &mut Criterion) {
    let mut fb = Framebuffer::new(3840, 2160).unwrap();
    let params = GlitchParams {
        intensity: 1.0,
        seed_time: 42,
        color_shift_amount: 10,
        jitter_amount: 20,
        block_displacement_prob: 1.0,
    };

    c.bench_function("glitch_4k", |b| {
        b.iter(|| {
            apply_glitch(black_box(&mut fb), black_box(&params));
        })
    });
}

criterion_group!(benches, bench_glitch);
criterion_main!(benches);
