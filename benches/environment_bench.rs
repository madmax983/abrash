use criterion::{criterion_group, criterion_main, Criterion};

fn bench_environment_conversion(c: &mut Criterion) {
    let mut group = c.benchmark_group("Environment Texture Conversion");

    let size = 1024 * 1024; // 1024x1024 face
    let pixels = vec![0xFFAABBCCu32; size]; // Dummy ARGB pixels

    group.bench_function("extend_from_slice", |b| {
        b.iter(|| {
            let mut rgba = Vec::with_capacity((size * 4) as usize);
            for &argb in &pixels {
                let bytes = [
                    ((argb >> 16) & 0xFF) as u8,
                    ((argb >> 8) & 0xFF) as u8,
                    (argb & 0xFF) as u8,
                    ((argb >> 24) & 0xFF) as u8,
                ];
                rgba.extend_from_slice(&bytes);
            }
            rgba
        })
    });

    group.bench_function("chunks_exact_mut", |b| {
        b.iter(|| {
            let mut rgba = vec![0u8; pixels.len() * 4];
            for (chunk, &argb) in rgba.chunks_exact_mut(4).zip(pixels.iter()) {
                chunk[0] = ((argb >> 16) & 0xFF) as u8;
                chunk[1] = ((argb >> 8) & 0xFF) as u8;
                chunk[2] = (argb & 0xFF) as u8;
                chunk[3] = ((argb >> 24) & 0xFF) as u8;
            }
            rgba
        })
    });

    group.finish();
}

criterion_group!(benches, bench_environment_conversion);
criterion_main!(benches);
