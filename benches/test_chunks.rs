use criterion::{Criterion, black_box, criterion_group, criterion_main};

fn clear_unsafe(depths: &mut [f32], w: usize, sx: usize, ex: usize, sy: usize, ey: usize) {
    let start_idx = sy * w;
    let len = ex - sx;
    let mut offset = start_idx + sx;
    let slice = depths;
    for _ in sy..ey {
        unsafe {
            slice
                .get_unchecked_mut(offset..offset + len)
                .fill(f32::INFINITY);
        }
        offset += w;
    }
}

fn clear_safe(depths: &mut [f32], w: usize, sx: usize, ex: usize, sy: usize, ey: usize) {
    let start_idx = sy * w;
    let end_idx = ey * w;
    depths[start_idx..end_idx]
        .chunks_exact_mut(w)
        .for_each(|row| row[sx..ex].fill(f32::INFINITY));
}

fn bench(c: &mut Criterion) {
    let mut depths = vec![0.0; 1920 * 1080];
    let mut group = c.benchmark_group("test_chunks");
    group.bench_function("unsafe", |b| {
        b.iter(|| {
            clear_unsafe(
                &mut depths,
                black_box(1920),
                black_box(100),
                black_box(1000),
                black_box(100),
                black_box(500),
            )
        });
    });
    group.bench_function("safe", |b| {
        b.iter(|| {
            clear_safe(
                &mut depths,
                black_box(1920),
                black_box(100),
                black_box(1000),
                black_box(100),
                black_box(500),
            )
        });
    });
}
criterion_group!(benches, bench);
criterion_main!(benches);
