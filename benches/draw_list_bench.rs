use abrash_core::math::{Mat4, Vec3};
use abrash_core::mesh::Mesh;
use abrash_render::render_api::cpu_renderer::CpuRenderer;
use abrash_render::render_api::frame::{Frame, FrameCamera};
use abrash_render::render_api::material::Material;
use criterion::{Criterion, black_box, criterion_group, criterion_main};

fn draw_list_benchmark(c: &mut Criterion) {
    let mut renderer = CpuRenderer::new(800, 600);

    // Create a heavy mesh: a sphere instead of a cube to have more vertices
    let mesh = Mesh::sphere(1.0, 32, 32);
    let mesh_h = renderer.create_mesh(&mesh).unwrap();
    let mat_h = renderer
        .create_material(Material::flat(0xFFFF_0000))
        .unwrap();

    let camera = FrameCamera::new(
        Mat4::look_at(
            Vec3::new(0.0, 0.0, 50.0),
            Vec3::ZERO,
            Vec3::new(0.0, 1.0, 0.0),
        ),
        Mat4::perspective(1.57, 800.0 / 600.0, 0.1, 100.0),
    );

    let mut group = c.benchmark_group("draw_list_extraction");

    for &num_objects in &[10, 100, 1000] {
        let mut frame = Frame::with_capacity(camera, num_objects, 0);
        for i in 0..num_objects {
            let offset = (i as f32) * 2.0;
            frame.draw(mesh_h, mat_h, Mat4::translation(offset, 0.0, 0.0));
        }

        group.bench_function(format!("extract_draw_list_{num_objects}objs"), |b| {
            b.iter(|| {
                black_box(renderer.extract_draw_list(black_box(&frame)).unwrap());
            });
        });
    }

    group.finish();
}

criterion_group!(benches, draw_list_benchmark);
criterion_main!(benches);
