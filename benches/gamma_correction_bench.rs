use abrash::framebuffer::Framebuffer;
use abrash_render::post_process::filters::apply_gamma_correction;
use criterion::{Criterion, black_box, criterion_group, criterion_main};

fn bench_gamma_correction(c: &mut Criterion) {
    let mut fb = Framebuffer::new(1920, 1080).unwrap();

    // Fill with a gradient pattern to cover many color ranges
    for y in 0..1080 {
        for x in 0..1920 {
            let r = (x * 4) as u32 % 256;
            let g = (y * 4) as u32 % 256;
            let b = ((x + y) * 2) as u32 % 256;
            let color = 0xFF00_0000 | (r << 16) | (g << 8) | b;
            fb.set_pixel(x as i32, y as i32, color);
        }
    }

    c.bench_function("apply_gamma_correction_1080p", |b| {
        b.iter(|| {
            apply_gamma_correction(black_box(&mut fb), black_box(2.2));
        });
    });
}

criterion_group!(benches, bench_gamma_correction);
criterion_main!(benches);
