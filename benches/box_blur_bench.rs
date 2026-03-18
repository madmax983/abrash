use abrash::post_process::blur::{box_blur_horizontal, box_blur_vertical};
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn bench_box_blur(c: &mut Criterion) {
    let width = 1920;
    let height = 1080;
    let radius = 10;
    let len = width * height;

    let src = vec![0xFFFFFFFFu32; len];
    let mut dest = vec![0u32; len];
    let mut acc_buffer = vec![0i32; width * 3];

    let mut group = c.benchmark_group("box_blur");

    group.bench_function("horizontal", |b| {
        b.iter(|| {
            box_blur_horizontal(
                black_box(&src),
                black_box(&mut dest),
                black_box(width),
                black_box(height),
                black_box(radius),
            )
        })
    });

    group.bench_function("vertical", |b| {
        b.iter(|| {
            box_blur_vertical(
                black_box(&src),
                black_box(&mut dest),
                black_box(&mut acc_buffer),
                black_box(width),
                black_box(height),
                black_box(radius),
            )
        })
    });

    group.finish();
}

criterion_group!(benches, bench_box_blur);
criterion_main!(benches);
