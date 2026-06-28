use criterion::{Criterion, black_box, criterion_group, criterion_main};

fn blitter_pixel_conversion_bench(c: &mut Criterion) {
    let width = 1920;
    let height = 1080;
    let pixels = vec![0xFFAABBCCu32; width * height];

    c.bench_function("pixel_conversion_zeroed", |b| {
        b.iter(|| {
            let mut rgba = vec![0u8; width * height * 4];
            for (&px, chunk) in pixels.iter().zip(rgba.chunks_exact_mut(4)) {
                chunk[0] = ((px >> 16) & 0xFF) as u8;
                chunk[1] = ((px >> 8) & 0xFF) as u8;
                chunk[2] = (px & 0xFF) as u8;
                chunk[3] = ((px >> 24) & 0xFF) as u8;
            }
            black_box(rgba);
        })
    });

    c.bench_function("pixel_conversion_uninit", |b| {
        b.iter(|| {
            let mut rgba = Vec::with_capacity(width * height * 4);
            let slice = rgba.spare_capacity_mut();
            for (&px, chunk) in pixels.iter().zip(slice.chunks_exact_mut(4)) {
                chunk[0].write(((px >> 16) & 0xFF) as u8);
                chunk[1].write(((px >> 8) & 0xFF) as u8);
                chunk[2].write((px & 0xFF) as u8);
                chunk[3].write(((px >> 24) & 0xFF) as u8);
            }
            unsafe {
                rgba.set_len(width * height * 4);
            }
            black_box(rgba);
        })
    });
}

criterion_group!(benches, blitter_pixel_conversion_bench);
criterion_main!(benches);
