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

    let config_zero_state = FilmGrainConfig {
        intensity: 0.25,
        // (row_offset as u32).wrapping_mul(0x9E3779B9) for row_offset = 0 is 0.
        // seed + 0 == 0 -> seed = 0. So seed = 0 tests the zero state handling on row 0.
        seed: 0,
    };

    c.bench_function("apply_film_grain zero-state (1080p)", |b| {
        b.iter(|| {
            apply_film_grain(black_box(&mut fb), black_box(&config_zero_state));
        });
    });
}

criterion_group!(benches, bench_film_grain);
criterion_main!(benches);
