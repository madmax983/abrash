use criterion::{Criterion, black_box, criterion_group, criterion_main};
use abrash_core::framebuffer::Framebuffer;
use abrash_core::zbuffer::ZBuffer;

#[cfg(feature = "nova")]
use abrash_render::experimental::vision::{apply_vision, VisionConfig, VisionMode};

#[cfg(feature = "nova")]
fn bench_vision(c: &mut Criterion) {
    let mut group = c.benchmark_group("vision");

    // Standard low-res retro resolution
    let width = 320;
    let height = 240;

    let mut fb = Framebuffer::new(width, height).unwrap();
    let zb = ZBuffer::new(width, height).unwrap();

    // Fill with dummy data
    let fb_slice = fb.as_mut_slice();
    for i in 0..fb_slice.len() {
        fb_slice[i] = 0xFF80_8080;
    }

    let config = VisionConfig {
        mode: VisionMode::Night,
        time: 0.0,
        intensity: 1.0,
    };

    group.bench_function("apply_night_vision", |b| {
        b.iter(|| {
            apply_vision(black_box(&mut fb), black_box(&zb), black_box(&config));
        });
    });

    group.finish();
}

#[cfg(not(feature = "nova"))]
fn bench_vision(c: &mut Criterion) {
    let mut group = c.benchmark_group("vision");
    group.bench_function("stub", |b| b.iter(|| black_box(0)));
    group.finish();
}

criterion_group!(benches, bench_vision);
criterion_main!(benches);
