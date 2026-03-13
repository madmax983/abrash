use criterion::{Criterion, black_box, criterion_group, criterion_main};
use rayon::prelude::*;

fn chunk_iteration_benchmark(c: &mut Criterion) {
    let width = 1920;
    let height = 1080;
    let mut fb = vec![0u32; width * height];
    let mut fb_exact = vec![0u32; width * height];

    let mut group = c.benchmark_group("chunk_iteration");

    group.bench_function("chunks_mut", |b| {
        b.iter(|| {
            let dest = black_box(&mut fb);
            dest.chunks_mut(width)
                .enumerate()
                .for_each(|(y, row)| {
                    for x in 0..width {
                        row[x] = (x + y) as u32;
                    }
                });
        });
    });

    group.bench_function("chunks_exact_mut", |b| {
        b.iter(|| {
            let dest = black_box(&mut fb_exact);
            dest.chunks_exact_mut(width)
                .enumerate()
                .for_each(|(y, row)| {
                    for x in 0..width {
                        row[x] = (x + y) as u32;
                    }
                });
        });
    });

    group.bench_function("par_chunks_mut", |b| {
        b.iter(|| {
            let dest = black_box(&mut fb);
            dest.par_chunks_mut(width)
                .enumerate()
                .for_each(|(y, row)| {
                    for x in 0..width {
                        row[x] = (x + y) as u32;
                    }
                });
        });
    });

    group.bench_function("par_chunks_exact_mut", |b| {
        b.iter(|| {
            let dest = black_box(&mut fb_exact);
            dest.par_chunks_exact_mut(width)
                .enumerate()
                .for_each(|(y, row)| {
                    for x in 0..width {
                        row[x] = (x + y) as u32;
                    }
                });
        });
    });

    group.finish();
}

criterion_group!(benches, chunk_iteration_benchmark);
criterion_main!(benches);
