use abrash::post_process::blur::box_blur_horizontal;
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn bench_box_blur(c: &mut Criterion) {
    let width = 1920;
    let height = 1080;
    let mut src = vec![0xFFFFFFFF; width * height];
    let mut dest = vec![0; width * height];
    let radius = 5;

    c.bench_function("box_blur_horizontal_1080p", |b| {
        b.iter(|| {
            box_blur_horizontal(
                black_box(&src),
                black_box(&mut dest),
                black_box(width),
                black_box(height),
                black_box(radius),
            );
        })
    });
}

criterion_group!(benches, bench_box_blur);
criterion_main!(benches);
