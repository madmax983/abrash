use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn check_float_inf(pixels: &mut [u32], depths: &[f32], min_z: f32, scale: f32, lut: &[u32; 1024]) {
    for (pixel, &depth) in pixels.iter_mut().zip(depths.iter()) {
        if depth == f32::INFINITY {
            *pixel = 0xFF00_0010;
            continue;
        }

        let t = ((depth - min_z) * scale) as u32;
        let t = t.min(1023);

        *pixel = unsafe { *lut.get_unchecked(t as usize) };
    }
}

fn check_bits_inf(pixels: &mut [u32], depths: &[f32], min_z: f32, scale: f32, lut: &[u32; 1024]) {
    for (pixel, &depth) in pixels.iter_mut().zip(depths.iter()) {
        if depth.to_bits() == 0x7F80_0000 {
            *pixel = 0xFF00_0010;
            continue;
        }

        let t = ((depth - min_z) * scale) as u32;
        let t = t.min(1023);

        *pixel = unsafe { *lut.get_unchecked(t as usize) };
    }
}

fn check_chunks(pixels: &mut [u32], depths: &[f32], min_z: f32, scale: f32, lut: &[u32; 1024]) {
    // maybe zip chunks?
    let mut i = 0;
    while i + 4 <= pixels.len() {
        for j in 0..4 {
            let depth = depths[i + j];
            let pixel = &mut pixels[i + j];
            if depth == f32::INFINITY {
                *pixel = 0xFF00_0010;
                continue;
            }

            let t = ((depth - min_z) * scale) as u32;
            let t = t.min(1023);

            *pixel = unsafe { *lut.get_unchecked(t as usize) };
        }
        i += 4;
    }
    for j in i..pixels.len() {
        let depth = depths[j];
        let pixel = &mut pixels[j];
        if depth == f32::INFINITY {
            *pixel = 0xFF00_0010;
            continue;
        }

        let t = ((depth - min_z) * scale) as u32;
        let t = t.min(1023);

        *pixel = unsafe { *lut.get_unchecked(t as usize) };
    }
}

fn bench_inf_check(c: &mut Criterion) {
    let mut group = c.benchmark_group("Heat Vision Scalar");

    let size = 1920 * 1080;
    let mut depths = vec![0.5f32; size];
    let mut pixels = vec![0u32; size];
    let lut = [0u32; 1024];
    // Add some infinity
    for i in (0..size).step_by(10) {
        depths[i] = f32::INFINITY;
    }

    group.bench_function("float", |b| b.iter(|| check_float_inf(black_box(&mut pixels), black_box(&depths), 0.0, 1.0, &lut)));
    group.bench_function("bits", |b| b.iter(|| check_bits_inf(black_box(&mut pixels), black_box(&depths), 0.0, 1.0, &lut)));
    group.bench_function("chunks", |b| b.iter(|| check_chunks(black_box(&mut pixels), black_box(&depths), 0.0, 1.0, &lut)));
    group.finish();
}

criterion_group!(benches, bench_inf_check);
criterion_main!(benches);
