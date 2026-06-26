use abrash_core::framebuffer::Framebuffer;
use abrash_render::post_process::filters::apply_dithering;
use criterion::{Criterion, criterion_group, criterion_main};

fn bench_apply_dithering(c: &mut Criterion) {
    let width = 1920;
    let height = 1080;
    let mut fb = Framebuffer::new(width, height).unwrap();

    // Fill with a gradient
    let pixels = fb.as_mut_slice();
    for y in 0..height {
        for x in 0..width {
            let luma = (x as f32 / width as f32 * 255.0) as u8;
            pixels[(y * width + x) as usize] =
                (luma as u32) << 16 | (luma as u32) << 8 | (luma as u32) | 0xFF000000;
        }
    }

    c.bench_function("apply_dithering_1080p", |b| {
        b.iter(|| {
            apply_dithering(&mut fb);
            std::hint::black_box(&fb);
        })
    });
}

criterion_group!(benches, bench_apply_dithering);
criterion_main!(benches);
