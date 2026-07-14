use abrash::framebuffer::Framebuffer;
use abrash_render::experimental::flow_field::{FlowFieldConfig, apply_flow_field};
use criterion::{Criterion, black_box, criterion_group, criterion_main};

fn bench_flow_field(c: &mut Criterion) {
    let width = 640;
    let height = 480;
    let mut fb = Framebuffer::new(width, height).unwrap();
    let config = FlowFieldConfig::default();

    c.bench_function("apply_flow_field 640x480", |b| {
        b.iter(|| {
            apply_flow_field(black_box(&mut fb), black_box(&config));
        })
    });
}

criterion_group!(benches, bench_flow_field);
criterion_main!(benches);
