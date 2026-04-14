use abrash::framebuffer::Framebuffer;
use abrash::post_process::apply_gamma_correction;
use criterion::{Criterion, black_box, criterion_group, criterion_main};

fn benchmark_gamma(c: &mut Criterion) {
    let width = 1920;
    let height = 1080;
    let mut fb = Framebuffer::new(width, height).unwrap();

    // Fill with a gradient pattern to avoid easy branch prediction / zero optimizations
    for y in 0..height {
        for x in 0..width {
            let color = 0xFF00_0000 | (x & 0xFF) << 16 | (y & 0xFF) << 8 | ((x + y) & 0xFF);
            fb.set_pixel(x as i32, y as i32, color);
        }
    }

    c.bench_function("apply_gamma_correction 1080p (gamma=2.2)", |b| {
        b.iter(|| {
            apply_gamma_correction(black_box(&mut fb), black_box(2.2));
        });
    });
}

criterion_group!(benches, benchmark_gamma);
criterion_main!(benches);
