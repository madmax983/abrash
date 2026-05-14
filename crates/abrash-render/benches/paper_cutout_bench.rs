use criterion::{Criterion, criterion_group, criterion_main};
use std::hint::black_box;
use abrash_core::framebuffer::Framebuffer;
use abrash_core::zbuffer::ZBuffer;
use abrash_render::experimental::paper_cutout::{PaperCutoutConfig, apply_paper_cutout};

fn bench_paper_cutout(c: &mut Criterion) {
    let width = 800;
    let height = 600;
    let mut fb = Framebuffer::new(width, height).unwrap();
    let mut zb = ZBuffer::new(width, height).unwrap();

    fb.clear(0xFFF0F0F0);
    zb.clear();

    for y in 100..500 {
        for x in 100..500 {
            fb.set_pixel(x, y, 0xFF00FF00);
            zb.test_and_set(x, y, 10.0);
        }
    }
    for y in 200..400 {
        for x in 200..400 {
            fb.set_pixel(x, y, 0xFFFF0000);
            zb.test_and_set(x, y, 5.0);
        }
    }

    let config = PaperCutoutConfig::default();

    c.bench_function("apply_paper_cutout_800x600", |b| {
        b.iter(|| {
            apply_paper_cutout(black_box(&mut fb), black_box(&zb), black_box(&config));
        })
    });
}

criterion_group!(benches, bench_paper_cutout);
criterion_main!(benches);
