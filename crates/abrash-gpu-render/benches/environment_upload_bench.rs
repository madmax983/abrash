use criterion::{Criterion, black_box, criterion_group, criterion_main};

fn environment_upload_bench(c: &mut Criterion) {
    let size = 1024;
    let num_pixels = size * size;
    let pixels = vec![0xFFAABBCCu32; num_pixels];

    let mut group = c.benchmark_group("Environment Upload Pixel Conversion");

    group.bench_function("pixel_conversion_extend_from_slice", |b| {
        b.iter(|| {
            let mut rgba = Vec::with_capacity((size * size * 4) as usize);
            for &argb in pixels.iter() {
                let bytes = [
                    ((argb >> 16) & 0xFF) as u8,
                    ((argb >> 8) & 0xFF) as u8,
                    (argb & 0xFF) as u8,
                    ((argb >> 24) & 0xFF) as u8,
                ];
                rgba.extend_from_slice(&bytes);
            }
            black_box(rgba);
        })
    });

    group.bench_function("pixel_conversion_chunks_exact_mut", |b| {
        b.iter(|| {
            let mut rgba = vec![0u8; (size * size * 4) as usize];
            for (chunk, &argb) in rgba.chunks_exact_mut(4).zip(pixels.iter()) {
                chunk[0] = ((argb >> 16) & 0xFF) as u8;
                chunk[1] = ((argb >> 8) & 0xFF) as u8;
                chunk[2] = (argb & 0xFF) as u8;
                chunk[3] = ((argb >> 24) & 0xFF) as u8;
            }
            black_box(rgba);
        })
    });

    group.finish();
}

criterion_group!(benches, environment_upload_bench);
criterion_main!(benches);
