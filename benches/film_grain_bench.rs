use abrash::framebuffer::Framebuffer;
use abrash::post_process::filters::{FilmGrainConfig, apply_film_grain};
use criterion::{Criterion, black_box, criterion_group, criterion_main};

fn bench_film_grain(c: &mut Criterion) {
    let mut fb = Framebuffer::new(1920, 1080).unwrap();
    // Fill with a uniform color
    fb.clear(0xFF808080);
    let config = FilmGrainConfig {
        intensity: 0.25,
        seed: 0xBADF00D,
    };

    c.bench_function("apply_film_grain (1080p)", |b| {
        b.iter(|| {
            apply_film_grain(black_box(&mut fb), black_box(&config));
        });
    });
}

criterion_group!(benches, bench_film_grain);
criterion_main!(benches);
