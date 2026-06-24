use abrash_core::framebuffer::Framebuffer;
use abrash_core::zbuffer::ZBuffer;
use criterion::{Criterion, criterion_group, criterion_main};
use rand::Rng;
use std::hint::black_box;

// A copy of the scalar loop from heat_vision.rs to test its raw performance
// without the AVX SIMD detection interference.

fn apply_heat_vision_scalar_old(fb: &mut Framebuffer, zb: &ZBuffer, lut: &[u32; 1024]) {
    let pixels = fb.as_mut_slice();
    let depths = zb.as_slice();

    let mut min_z = f32::MAX;
    let mut max_z = f32::MIN;
    let mut has_content = false;

    for &z in depths {
        if z != f32::INFINITY {
            if z < min_z {
                min_z = z;
            }
            if z > max_z {
                max_z = z;
            }
            has_content = true;
        }
    }

    if !has_content {
        return;
    }

    let range = (max_z - min_z).max(0.0001);
    let scale = 1024.0 / range;

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

fn apply_heat_vision_scalar_new(fb: &mut Framebuffer, zb: &ZBuffer, lut: &[u32; 1024]) {
    let pixels = fb.as_mut_slice();
    let depths = zb.as_slice();

    let mut min_z = f32::MAX;
    let mut max_z = f32::MIN;
    let mut has_content = false;

    for &z in depths {
        if z.to_bits() != f32::INFINITY.to_bits() {
            if z < min_z {
                min_z = z;
            }
            if z > max_z {
                max_z = z;
            }
            has_content = true;
        }
    }

    if !has_content {
        return;
    }

    let range = (max_z - min_z).max(0.0001);
    let scale = 1024.0 / range;

    for (pixel, &depth) in pixels.iter_mut().zip(depths.iter()) {
        if depth.to_bits() == f32::INFINITY.to_bits() {
            *pixel = 0xFF00_0010;
            continue;
        }

        let t = ((depth - min_z) * scale) as u32;
        let t = t.min(1023);
        *pixel = unsafe { *lut.get_unchecked(t as usize) };
    }
}

fn bench_heat_vision_scalar(c: &mut Criterion) {
    let mut group = c.benchmark_group("Heat Vision Scalar Infinity Checks");

    let mut lut = [0u32; 1024];
    for (i, val) in lut.iter_mut().enumerate() {
        *val = i as u32;
    }

    let w = 800;
    let h = 600;
    let mut fb = Framebuffer::new(w, h).unwrap();
    let mut zb = ZBuffer::new(w, h).unwrap();

    let mut rng = rand::thread_rng();
    for y in 0..h {
        for x in 0..w {
            let depth = if rng.gen_bool(0.1) {
                f32::INFINITY
            } else {
                rng.gen_range(0.1..100.0)
            };
            unsafe {
                zb.test_and_set_unchecked(x as usize, y as usize, depth);
            }
        }
    }

    group.bench_function("Old (Float ==)", |b| {
        b.iter(|| {
            apply_heat_vision_scalar_old(black_box(&mut fb), black_box(&zb), black_box(&lut));
        });
    });

    group.bench_function("New (Bits ==)", |b| {
        b.iter(|| {
            apply_heat_vision_scalar_new(black_box(&mut fb), black_box(&zb), black_box(&lut));
        });
    });

    group.finish();
}

criterion_group!(benches, bench_heat_vision_scalar);
criterion_main!(benches);
