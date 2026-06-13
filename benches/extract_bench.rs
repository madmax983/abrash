use abrash_core::math::{Mat4, Vec3};
use abrash_core::mesh::Mesh;
use abrash_render::render_api::cpu_renderer::CpuRenderer;
use abrash_render::render_api::frame::{Frame, FrameCamera};
use abrash_render::render_api::material::Material;
use criterion::{Criterion, black_box, criterion_group, criterion_main};

fn bench_extract(c: &mut Criterion) {
    let mut renderer = CpuRenderer::new(800, 600);
    let mut mesh = Mesh::cube(1.0);
    for _ in 0..8 {
        let cloned = mesh.clone();
        for v in &mut mesh.vertices {
            v.x += 2.0;
        }
        mesh.vertices.extend(cloned.vertices);
        mesh.indices.extend(cloned.indices);
    }

    let mesh_h = renderer.create_mesh(mesh.clone()).unwrap();
    let mat_h = renderer
        .create_material(Material::flat(0xFFFF_0000))
        .unwrap();

    let camera = FrameCamera::new(
        Mat4::look_at(
            Vec3::new(0.0, 0.0, 5.0),
            Vec3::ZERO,
            Vec3::new(0.0, 1.0, 0.0),
        ),
        Mat4::perspective(1.57, 800.0 / 600.0, 0.1, 100.0),
    );

    let mut frame = Frame::new(camera);
    for i in 0..100 {
        frame.draw(mesh_h, mat_h, Mat4::translation(i as f32, 0.0, 0.0));
    }

    c.bench_function("extract_draw_list_large", |b| {
        b.iter(|| {
            let dl = renderer.extract_draw_list(black_box(&frame)).unwrap();
            black_box(dl);
        })
    });
}

criterion_group!(benches, bench_extract);
criterion_main!(benches);
