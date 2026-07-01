use abrash_render::procedural::plasma;
use criterion::{Criterion, black_box, criterion_group, criterion_main};

fn bench_plasma_procedural(c: &mut Criterion) {
    let mut group = c.benchmark_group("Plasma Procedural");

    let resolutions = [(64, 64), (128, 128), (256, 256)];

    for (w, h) in resolutions {
        group.bench_function(format!("{w}x{h}"), |b| {
            b.iter(|| {
                black_box(plasma(w, h).unwrap());
            });
        });
    }

    group.finish();
}

criterion_group!(benches, bench_plasma_procedural);
criterion_main!(benches);
