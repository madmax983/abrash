use abrash::framebuffer::Framebuffer;
use abrash::math::{Mat4, Vec3};
use abrash::mesh::Mesh;
use abrash::rasterizer::{ClipTriangle, TileRenderer};
use abrash::scene::{Camera, Scene, SceneObject};
use abrash::zbuffer::ZBuffer;
use criterion::{Criterion, criterion_group, criterion_main};
use std::sync::Arc;

// Helper to create a simple cube mesh
fn create_cube() -> Mesh {
    Mesh::cube(1.0)
}

fn object_culling_benchmark(c: &mut Criterion) {
    let width = 1920;
    let height = 1080;
    let mut fb = Framebuffer::new(width, height).unwrap();
    let mut zb = ZBuffer::new(width, height).unwrap();
    let mut renderer = TileRenderer::new(width, height);

    let cube_mesh = create_cube();

    // Scene setup: 1000 objects arranged in a line
    // Camera looks at the first few.
    let mut objects = Vec::with_capacity(1000);
    for i in 0..1000 {
        // Place objects along Z axis. Camera is at origin looking down -Z.
        // Objects at -5, -10, -15...
        // Objects further than -100 (far plane) should be culled.
        let pos = Vec3::new(0.0, 0.0, -5.0 - (i as f32) * 5.0);
        objects.push(pos);
    }

    let camera_pos = Vec3::new(0.0, 0.0, 0.0);
    let camera_target = Vec3::new(0.0, 0.0, -1.0);
    let camera_up = Vec3::new(0.0, 1.0, 0.0);

    let view = Mat4::look_at(camera_pos, camera_target, camera_up);
    let proj = Mat4::perspective(1.57, width as f32 / height as f32, 0.1, 100.0);
    let view_proj = view * proj;

    // Naive rendering: process all objects
    c.bench_function("render_scene_naive", |b| {
        b.iter(|| {
            fb.clear(0xFF00_0000);
            zb.clear();

            let mut triangles: Vec<ClipTriangle> = Vec::with_capacity(1000 * 12);

            for pos in &objects {
                // Model matrix (just translation)
                let model = Mat4::translation(pos.x, pos.y, pos.z);
                // MVP
                let mvp = model * view_proj;

                // Transform all vertices (naive, per object)
                for indices in &cube_mesh.indices {
                    let v0_local = cube_mesh.vertices[indices[0]];
                    let v1_local = cube_mesh.vertices[indices[1]];
                    let v2_local = cube_mesh.vertices[indices[2]];

                    let (v0_clip, w0) = mvp.transform_point(v0_local);
                    let (v1_clip, w1) = mvp.transform_point(v1_local);
                    let (v2_clip, w2) = mvp.transform_point(v2_local);

                    triangles.push(((v0_clip, w0), (v1_clip, w1), (v2_clip, w2), 0xFFFF_FFFF));
                }
            }

            renderer.render_batch(&mut fb, &mut zb, &triangles);
        });
    });

    // Optimized rendering: use Scene with Culling
    c.bench_function("render_scene_optimized", |b| {
        let mesh_arc = Arc::new(create_cube());
        let mut scene = Scene::new(Camera::new(view, proj));

        for pos in &objects {
            let model = Mat4::translation(pos.x, pos.y, pos.z);
            scene.add_object(SceneObject::new(mesh_arc.clone(), model, 0xFFFF_FFFF));
        }

        b.iter(|| {
            fb.clear(0xFF00_0000);
            zb.clear();
            scene.render(&mut renderer, &mut fb, &mut zb);
        });
    });
}

criterion_group!(benches, object_culling_benchmark);
criterion_main!(benches);
