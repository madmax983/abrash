use abrash_core::framebuffer::Framebuffer;
use abrash_render::experimental::plasma::apply_plasma;
use criterion::{Criterion, black_box, criterion_group, criterion_main};

fn bench_plasma_procedural(c: &mut Criterion) {
    let mut group = c.benchmark_group("Plasma Effect Optimization");

    let resolutions = [(400, 300), (800, 600), (1920, 1080)];
    let time = 1.0;
    let scale = 0.05;

    for (width, height) in resolutions {
        let mut fb = Framebuffer::new(width, height).unwrap();
        group.bench_function(format!("plasma_{width}x{height}"), |b| {
            b.iter(|| {
                apply_plasma(black_box(&mut fb), black_box(time), black_box(scale));
            });
        });
    }

    group.finish();
}

criterion_group!(benches, bench_plasma_procedural);
criterion_main!(benches);
