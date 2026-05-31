use abrash_core::framebuffer::Framebuffer;
use abrash_render::experimental::thermal::{ThermalConfig, apply_thermal};
use criterion::{Criterion, criterion_group, criterion_main};
use std::hint::black_box;

fn bench_thermal(c: &mut Criterion) {
    let mut group = c.benchmark_group("Thermal Vision");

    let resolutions = [(320, 240), (800, 600), (1920, 1080)];

    for (w, h) in resolutions {
        let mut fb = Framebuffer::new(w, h).unwrap();
        // Fill fb with random colors
        for y in 0..h {
            for x in 0..w {
                fb.set_pixel(x as i32, y as i32, (x * y) | 0xFF00_0000);
            }
        }

        let config = ThermalConfig::default();

        group.bench_function(format!("{w}x{h}"), |b| {
            b.iter(|| {
                apply_thermal(black_box(&mut fb), black_box(&config));
            });
        });
    }

    group.finish();
}

criterion_group!(benches, bench_thermal);
criterion_main!(benches);
