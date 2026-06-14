use criterion::{Criterion, criterion_group, criterion_main};
use rand::Rng;

fn bench_heat_vision_scalar(c: &mut Criterion) {
    let mut group = c.benchmark_group("Heat Vision Scalar Comparison");

    let size = 1920 * 1080;

    let mut pixels = vec![0u32; size];
    let mut depths = vec![0.0f32; size];

    let mut rng = rand::thread_rng();
    for i in 0..size {
        depths[i] = if rng.gen_bool(0.1) {
            f32::INFINITY
        } else {
            rng.gen_range(0.1..100.0)
        };
    }

    let min_z = 0.1f32;
    let scale = 10.24f32;
    let mut lut = [0u32; 1024];
    for i in 0..1024 { lut[i] = i as u32; }

    group.bench_function("Iterator Zip", |b| {
        b.iter(|| {
            for (pixel, &depth) in pixels.iter_mut().zip(depths.iter()) {
                if depth == f32::INFINITY {
                    *pixel = 0xFF00_0010;
                    continue;
                }
                let t = ((depth - min_z) * scale) as u32;
                let t = t.min(1023);
                *pixel = unsafe { *lut.get_unchecked(t as usize) };
            }
        });
    });

    group.bench_function("Manual Chunks", |b| {
        b.iter(|| {
            let len = pixels.len().min(depths.len());
            for i in 0..len {
                let depth = unsafe { *depths.get_unchecked(i) };
                if depth == f32::INFINITY {
                    unsafe { *pixels.get_unchecked_mut(i) = 0xFF00_0010; }
                    continue;
                }
                let t = ((depth - min_z) * scale) as u32;
                let t = t.min(1023);
                unsafe { *pixels.get_unchecked_mut(i) = *lut.get_unchecked(t as usize); }
            }
        });
    });

    group.finish();
}

criterion_group!(benches, bench_heat_vision_scalar);
criterion_main!(benches);
