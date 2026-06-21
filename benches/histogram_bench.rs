use abrash_core::framebuffer::Framebuffer;
use abrash_render::experimental::histogram::apply_histogram_equalization;
use criterion::{Criterion, black_box, criterion_group, criterion_main};

fn benchmark_histogram(c: &mut Criterion) {
    let width = 1920;
    let height = 1080;
    let mut fb = Framebuffer::new(width, height).unwrap();

    // Fill with a gradient to simulate a somewhat realistic image
    for y in 0..height {
        for x in 0..width {
            let val = (x + y) % 256;
            let color = 0xFF00_0000 | (val << 16) | (val << 8) | val;
            fb.set_pixel(x as i32, y as i32, color);
        }
    }

    c.bench_function("histogram_1080p", |b| {
        b.iter(|| apply_histogram_equalization(black_box(&mut fb)));
    });
}

criterion_group!(benches, benchmark_histogram);
criterion_main!(benches);
