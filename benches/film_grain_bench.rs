use criterion::{criterion_group, criterion_main, Criterion};
use abrash::framebuffer::Framebuffer;
use abrash::post_process::apply_film_grain;

fn bench_film_grain(c: &mut Criterion) {
    let mut group = c.benchmark_group("film_grain");
    let width = 1920;
    let height = 1080;
    let mut fb = Framebuffer::new(width, height).unwrap();

    // Fill with a non-zero color to actually test adding noise to values
    fb.clear(0xFF808080);

    group.bench_function("apply_film_grain_1080p", |b| {
        let mut seed = 0;
        b.iter(|| {
            apply_film_grain(&mut fb, 0.5, seed);
            seed = seed.wrapping_add(1);
        });
    });

    group.finish();
}

criterion_group!(benches, bench_film_grain);
criterion_main!(benches);
