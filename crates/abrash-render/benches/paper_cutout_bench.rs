use abrash_core::framebuffer::Framebuffer;
use abrash_core::zbuffer::ZBuffer;
use abrash_render::experimental::paper_cutout::{PaperCutoutConfig, apply_paper_cutout};
use criterion::{Criterion, criterion_group, criterion_main};

fn paper_cutout_benchmark(c: &mut Criterion) {
    let width = 800;
    let height = 600;
    let mut fb = Framebuffer::new(width, height).unwrap();
    let mut zb = ZBuffer::new(width, height).unwrap();

    // Fill with some dummy depth data
    for y in 0..height {
        for x in 0..width {
            let d = (x as f32 / width as f32) * 100.0;
            zb.test_and_set(x as i32, y as i32, d);
            fb.set_pixel(x as i32, y as i32, 0xFFFFFFFF);
        }
    }

    let config = PaperCutoutConfig::default();

    c.bench_function("paper_cutout_800x600", |b| {
        b.iter(|| {
            apply_paper_cutout(&mut fb, &zb, &config);
            std::hint::black_box(&fb);
        })
    });
}

criterion_group!(benches, paper_cutout_benchmark);
criterion_main!(benches);
