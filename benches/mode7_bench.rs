use abrash::experimental::mode7::{Mode7Config, render_mode7};
use abrash::framebuffer::Framebuffer;
use abrash::texture::Texture;
use criterion::{Criterion, black_box, criterion_group, criterion_main};

fn mode7_benchmark(c: &mut Criterion) {
    let mut fb = Framebuffer::new(800, 600).unwrap();
    let tex = Texture::new(1024, 1024).unwrap();

    let config = Mode7Config::default();

    c.bench_function("render_mode7_baseline", |b| {
        b.iter(|| {
            render_mode7(black_box(&mut fb), black_box(&tex), black_box(&config));
        });
    });
}

criterion_group!(benches, mode7_benchmark);
criterion_main!(benches);
