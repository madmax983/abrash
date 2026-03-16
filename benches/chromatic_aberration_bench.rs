use abrash::experimental::chromatic_aberration::{
    ChromaticAberrationConfig, apply_chromatic_aberration,
};
use abrash::framebuffer::Framebuffer;
use criterion::{Criterion, black_box, criterion_group, criterion_main};

fn criterion_benchmark(c: &mut Criterion) {
    let width = 1920;
    let height = 1080;
    let mut fb = Framebuffer::new(width, height).unwrap();
    fb.clear(0xFF_FF_FF_FF);

    let config = ChromaticAberrationConfig {
        red_shift: (5, 0),
        green_shift: (0, 0),
        blue_shift: (-5, 0),
    };

    c.bench_function("apply_chromatic_aberration_1080p", |b| {
        b.iter(|| {
            apply_chromatic_aberration(black_box(&mut fb), black_box(&config));
        })
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
