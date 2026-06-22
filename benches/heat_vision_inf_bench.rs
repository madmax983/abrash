use abrash_core::framebuffer::Framebuffer;
use abrash_core::zbuffer::ZBuffer;
use abrash_render::heat_vision::apply_heat_vision;
use criterion::{Criterion, criterion_group, criterion_main};
use std::hint::black_box;

fn bench_heat_vision_inf(c: &mut Criterion) {
    let mut group = c.benchmark_group("Heat Vision Infinity Check Optimization");

    // We will benchmark 800x600 which is enough to measure the pure pixel loop processing.
    let w = 800;
    let h = 600;

    let mut fb = Framebuffer::new(w, h).unwrap();
    let mut zb = ZBuffer::new(w, h).unwrap();

    // Fill Z-buffer entirely with INFINITY to stress the fast-path check
    for y in 0..h {
        for x in 0..w {
            unsafe {
                zb.test_and_set_unchecked(x as usize, y as usize, f32::INFINITY);
            }
        }
    }

    group.bench_function(format!("{w}x{h}"), |b| {
        b.iter(|| {
            apply_heat_vision(black_box(&mut fb), black_box(&zb));
        });
    });

    group.finish();
}

criterion_group!(benches, bench_heat_vision_inf);
criterion_main!(benches);
