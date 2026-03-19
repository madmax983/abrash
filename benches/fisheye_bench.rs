use abrash_render::experimental::fisheye::{apply_fisheye, FisheyeConfig};
use abrash_render::framebuffer::Framebuffer;
use criterion::{criterion_group, criterion_main, Criterion};

fn fisheye_benchmark(c: &mut Criterion) {
    let mut fb = Framebuffer::new(1920, 1080).unwrap();
    // Fill with some dummy data
    let mut color = 0;
    for pixel in fb.as_mut_slice().iter_mut() {
        *pixel = color;
        color = color.wrapping_add(1);
    }

    let config = FisheyeConfig {
        center_x: 0.5,
        center_y: 0.5,
        strength: 2.0,
    };

    c.bench_function("fisheye_1080p", |b| {
        b.iter(|| {
            apply_fisheye(&mut fb, &config);
        });
    });
}

criterion_group!(benches, fisheye_benchmark);
criterion_main!(benches);
