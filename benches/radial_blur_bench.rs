#[cfg(feature = "nova")]
use abrash::experimental::radial_blur::apply_radial_blur;
#[cfg(feature = "nova")]
use abrash::framebuffer::Framebuffer;
#[cfg(feature = "nova")]
use criterion::{Criterion, black_box, criterion_group, criterion_main};

#[cfg(feature = "nova")]
fn radial_blur_benchmark(c: &mut Criterion) {
    let width = 1920;
    let height = 1080;
    let mut fb = Framebuffer::new(width, height).unwrap();

    let pixels = fb.as_mut_slice();
    for y in 0..height {
        for x in 0..width {
            let val = ((x + y) % 256) as u32;
            pixels[(y * width + x) as usize] = 0xFF00_0000 | (val << 16) | (val << 8) | val;
        }
    }

    let mut group = c.benchmark_group("radial_blur");
    group.sample_size(10);

    group.bench_function("1080p", |b| {
        b.iter(|| {
            apply_radial_blur(black_box(&mut fb), black_box(width / 2), black_box(height / 2), black_box(0.05), black_box(16));
        })
    });

    group.finish();
}

#[cfg(feature = "nova")]
criterion_group!(benches, radial_blur_benchmark);

#[cfg(feature = "nova")]
criterion_main!(benches);

#[cfg(not(feature = "nova"))]
fn main() {}
