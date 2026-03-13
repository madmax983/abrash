use abrash::experimental::radial_blur::apply_radial_blur;
use abrash::framebuffer::Framebuffer;
use criterion::{Criterion, criterion_group, criterion_main};

fn bench_radial_blur(c: &mut Criterion) {
    let mut fb = Framebuffer::new(1024, 1024).unwrap();
    // Fill fb with something to avoid optimizing out
    for (i, p) in fb.as_mut_slice().iter_mut().enumerate() {
        *p = i as u32;
    }

    c.bench_function("radial_blur_1024x1024", |b| {
        b.iter(|| {
            apply_radial_blur(&mut fb, 512, 512, 0.5, 10);
        });
    });
}

criterion_group!(benches, bench_radial_blur);
criterion_main!(benches);
