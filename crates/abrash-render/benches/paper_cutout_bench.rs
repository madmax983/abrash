use abrash_core::framebuffer::Framebuffer;
use abrash_core::zbuffer::ZBuffer;
use abrash_render::experimental::paper_cutout::{PaperCutoutConfig, apply_paper_cutout};
use criterion::{Criterion, criterion_group, criterion_main};

fn paper_cutout_benchmark(c: &mut Criterion) {
    let mut fb = Framebuffer::new(800, 600).unwrap();
    let mut zb = ZBuffer::new(800, 600).unwrap();

    fb.clear(0xFFF0_F0F0);
    zb.clear();

    // Draw some mock depths
    for y in 100..500 {
        for x in 100..500 {
            fb.set_pixel(x, y, 0xFF00_FF00);
            zb.test_and_set(x, y, 10.0);
        }
    }

    let config = PaperCutoutConfig::default();

    c.bench_function("apply_paper_cutout_800x600", |b| {
        b.iter(|| {
            apply_paper_cutout(&mut fb, &zb, &config);
            std::hint::black_box(&fb);
        })
    });
}

criterion_group!(benches, paper_cutout_benchmark);
criterion_main!(benches);
