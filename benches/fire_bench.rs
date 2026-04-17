use criterion::{black_box, criterion_group, criterion_main, Criterion};
use abrash_core::framebuffer::Framebuffer;
use abrash_render::experimental::fire::apply_fire;

fn fire_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("fire_effect");

    // Test multiple resolutions
    for (width, height) in [(320, 240), (640, 480)] {
        let mut fb = Framebuffer::new(width, height).unwrap();
        let cooling_map = vec![0; (width * height) as usize];

        // Seed the bottom row with heat for a realistic physical initialization
        for x in 0..width {
            fb.as_mut_slice()[((height - 1) * width + x) as usize] = 0x00FFFFFF;
        }

        group.bench_function(format!("apply_fire {}x{}", width, height), |b| {
            b.iter(|| apply_fire(black_box(&mut fb), black_box(&cooling_map)))
        });
    }

    group.finish();
}

criterion_group!(benches, fire_benchmark);
criterion_main!(benches);
