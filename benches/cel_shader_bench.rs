use abrash_render::experimental::cel_shader::{CelShaderConfig, apply_cel_shader};
use abrash_render::framebuffer::Framebuffer;
use criterion::{Criterion, black_box, criterion_group, criterion_main};

fn bench_cel_shader(c: &mut Criterion) {
    let mut fb = Framebuffer::new(1920, 1080).unwrap();
    let config = CelShaderConfig::default();

    c.bench_function("cel_shader_1080p", |b| {
        b.iter(|| {
            apply_cel_shader(black_box(&mut fb), black_box(&config));
        });
    });
}

criterion_group!(benches, bench_cel_shader);
criterion_main!(benches);
