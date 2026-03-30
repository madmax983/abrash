use criterion::{black_box, criterion_group, criterion_main, Criterion};
use abrash_core::mesh::Mesh;
use abrash_render::render_api::renderer::Renderer;
use abrash_render::render_api::cpu_renderer::CpuRenderer;

fn bench_update_mesh(c: &mut Criterion) {
    let mut renderer = CpuRenderer::new(800, 600);
    // Use a large mesh to see the effect of avoiding allocation
    let mut mesh = Mesh::sphere(1.0, 100, 100);
    let handle = renderer.create_mesh(&mesh).unwrap();

    c.bench_function("cpu_renderer_update_mesh", |b| {
        b.iter(|| {
            // Modify just a little bit so that it's a new mesh, but structurally same
            mesh.vertices[0].x += 0.0001;
            renderer.update_mesh(black_box(handle), black_box(&mesh)).unwrap();
        });
    });
}

criterion_group!(benches, bench_update_mesh);
criterion_main!(benches);
