use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn generate_lut() -> [u32; 1024] {
    let mut lut = [0u32; 1024];
    for t in 0..1024 {
        lut[t] = t as u32; // Dummy LUT for benchmark
    }
    lut
}

pub fn scalar_fallback_loop(pixels: &mut [u32], depths: &[f32], min_z: f32, scale: f32, lut: &[u32; 1024]) {
    for (pixel, &depth) in pixels.iter_mut().zip(depths.iter()) {
        if depth.to_bits() == 0x7F80_0000 { // 0x7F80_0000 is f32::INFINITY
            *pixel = 0xFF00_0010; // Very Dark Blue Background
            continue;
        }

        let t = ((depth - min_z) * scale) as u32;
        let t = t.min(1023); // Clamp strictly to 1023

        // SAFETY: t is strictly clamped to 1023 above, which is within the bounds of the 1024-element LUT.
        *pixel = unsafe { *lut.get_unchecked(t as usize) };
    }
}

fn bench_scalar_fallback(c: &mut Criterion) {
    let mut group = c.benchmark_group("Scalar Fallback Infinity Check");

    let len = 1920 * 1080;
    let mut pixels = vec![0u32; len];
    let mut depths = vec![0f32; len];
    let lut = generate_lut();

    // Fill with random mix of depths and infinities
    for i in 0..len {
        if i % 10 == 0 {
            depths[i] = f32::INFINITY;
        } else {
            depths[i] = (i as f32 % 100.0) + 1.0;
        }
    }

    group.bench_function("1920x1080", |b| {
        b.iter(|| {
            scalar_fallback_loop(
                black_box(&mut pixels),
                black_box(&depths),
                black_box(1.0),
                black_box(10.0),
                black_box(&lut),
            );
        });
    });

    group.finish();
}

criterion_group!(benches, bench_scalar_fallback);
criterion_main!(benches);
