use abrash::framebuffer::Framebuffer;
use abrash::post_process::apply_chromatic_aberration;
use criterion::{Criterion, black_box, criterion_group, criterion_main};

fn criterion_benchmark(c: &mut Criterion) {
    let width = 1920;
    let height = 1080;
    let mut fb = Framebuffer::new(width, height).unwrap();
    fb.clear(0xFF_FF_FF_FF);

    c.bench_function("apply_chromatic_aberration_1080p", |b| {
        b.iter(|| {
            apply_chromatic_aberration(
                black_box(&mut fb),
                black_box(&abrash::post_process::ChromaticAberrationConfig { offset: 5 }),
            );
        });
    });

    c.bench_function("apply_chromatic_aberration_4k", |b| {
        let mut fb_4k = Framebuffer::new(3840, 2160).unwrap();
        fb_4k.clear(0xFF_FF_FF_FF);
        b.iter(|| {
            apply_chromatic_aberration(
                black_box(&mut fb_4k),
                black_box(&abrash::post_process::ChromaticAberrationConfig { offset: 10 }),
            );
        });
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
