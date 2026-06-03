use abrash::framebuffer::Framebuffer;
use abrash_render::experimental::jpeg_artifact::{JpegArtifactConfig, apply_jpeg_artifact};
use criterion::{Criterion, black_box, criterion_group, criterion_main};

fn benchmark_jpeg_artifact(c: &mut Criterion) {
    let width = 800;
    let height = 600;
    let mut fb = Framebuffer::new(width, height).unwrap();

    // Fill with a gradient
    for y in 0..height {
        for x in 0..width {
            let color = 0xFF00_0000 | (x << 16) | y;
            fb.set_pixel(x as i32, y as i32, color);
        }
    }

    let mut group = c.benchmark_group("JpegArtifact Filter");

    let config = JpegArtifactConfig::default();

    group.bench_function("apply_jpeg_artifact 800x600", |b| {
        b.iter(|| {
            apply_jpeg_artifact(black_box(&mut fb), black_box(&config));
        });
    });

    group.finish();
}

criterion_group!(benches, benchmark_jpeg_artifact);
criterion_main!(benches);
