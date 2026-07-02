use abrash_render::heat_vision::find_depth_min_max;
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use rand::Rng;

fn bench_depth_min_max(c: &mut Criterion) {
    let mut group = c.benchmark_group("Depth Min Max");

    let sizes = [
        (320 * 240, "320x240"),
        (800 * 600, "800x600"),
        (1920 * 1080, "1920x1080"),
    ];

    for (size, name) in sizes {
        let mut rng = rand::thread_rng();
        let mut depths: Vec<f32> = Vec::with_capacity(size);
        for _ in 0..size {
            if rng.gen_bool(0.1) {
                depths.push(f32::INFINITY);
            } else {
                depths.push(rng.gen_range(0.1..100.0));
            }
        }

        group.bench_function(name, |b| {
            b.iter(|| {
                black_box(find_depth_min_max(black_box(&depths)));
            });
        });
    }

    group.finish();
}

criterion_group!(benches, bench_depth_min_max);
criterion_main!(benches);
