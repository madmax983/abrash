use abrash_core::framebuffer::Framebuffer;
use abrash_core::zbuffer::ZBuffer;
use abrash_render::heat_vision::apply_heat_vision;
use criterion::{Criterion, criterion_group, criterion_main, BenchmarkId};
use rand::Rng;
use std::hint::black_box;

fn apply_heat_vision_zip(fb: &mut Framebuffer, zb: &ZBuffer) {
    // A copy of the old algorithm to compare against
    const LUT: [u32; 1024] = generate_lut();

    if fb.width() != zb.width() || fb.height() != zb.height() {
        return;
    }

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
        for p in pixels.iter_mut() {
            *p = 0xFF00_0020;
        }
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

        *pixel = unsafe { *LUT.get_unchecked(t as usize) };
    }
}

const fn generate_lut() -> [u32; 1024] {
    let mut lut = [0u32; 1024];
    let mut t = 0;
    while t < 1024 {
        let (r, g, b) = if t < 256 {
            (255, t, 0)
        } else if t < 512 {
            let local_t = t - 256;
            (255 - local_t, 255, 0)
        } else if t < 768 {
            let local_t = t - 512;
            (0, 255, local_t)
        } else {
            let local_t = t - 768;
            (0, 255 - local_t, 255)
        };
        lut[t as usize] = 0xFF00_0000 | (r << 16) | (g << 8) | b;
        t += 1;
    }
    lut
}


fn bench_heat_vision_comparison(c: &mut Criterion) {
    let mut group = c.benchmark_group("Heat Vision Zip vs Manual");

    let resolutions = [(320, 240), (800, 600), (1920, 1080)];

    for (w, h) in resolutions {
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

        group.bench_with_input(BenchmarkId::new("Zip (Old)", format!("{w}x{h}")), &w, |b, _| {
            b.iter(|| {
                apply_heat_vision_zip(black_box(&mut fb), black_box(&zb));
            });
        });

        group.bench_with_input(BenchmarkId::new("Manual (New)", format!("{w}x{h}")), &w, |b, _| {
            b.iter(|| {
                apply_heat_vision(black_box(&mut fb), black_box(&zb));
            });
        });
    }

    group.finish();
}

criterion_group!(benches, bench_heat_vision_comparison);
criterion_main!(benches);
