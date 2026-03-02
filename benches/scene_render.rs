use abrash::framebuffer::Framebuffer;
use abrash::math::{Mat4, Vec3};
use abrash::mesh::Mesh;
use abrash::rasterizer::TileRenderer;
use abrash::scene::{Camera, Scene, SceneObject};
use abrash::zbuffer::ZBuffer;
use criterion::{Criterion, criterion_group, criterion_main};
use std::sync::Arc;

fn generate_grid_mesh(size: usize) -> Mesh {
    let mut mesh = Mesh::new();
    let offset = size as f32 * 0.5;

    // Vertices
    for y in 0..=size {
        for x in 0..=size {
            mesh.vertices
                .push(Vec3::new(x as f32 - offset, 0.0, y as f32 - offset));
        }
    }

    // Indices
    for y in 0..size {
        for x in 0..size {
            let i0 = y * (size + 1) + x;
            let i1 = i0 + 1;
            let i2 = (y + 1) * (size + 1) + x;
            let i3 = i2 + 1;

            mesh.indices.push([i0, i1, i2]);
            mesh.indices.push([i1, i3, i2]);
        }
    }

    mesh
}

fn bench_scene_render(c: &mut Criterion) {
    let width = 640;
    let height = 480;

    // Setup Scene
    // Use 100 objects of 10x10 grid (121 vertices, 200 triangles each)
    // Total: 20,000 triangles (same as before) but spread across 100 draw calls
    let mesh = Arc::new(generate_grid_mesh(10));

    let view = Mat4::look_at(
        Vec3::new(0.0, 50.0, 50.0), // Eye
        Vec3::new(0.0, 0.0, 0.0),   // Target
        Vec3::new(0.0, 1.0, 0.0),   // Up
    );
    let proj = Mat4::perspective(1.0, width as f32 / height as f32, 0.1, 1000.0);
    let camera = Camera::new(view, proj);

    let mut scene = Scene::new(camera);

    // Add 100 objects in a grid pattern
    for i in 0..100 {
        let x = (i % 10) as f32 * 15.0 - 75.0;
        let z = (i / 10) as f32 * 15.0 - 75.0;
        let transform = Mat4::translation(x, 0.0, z);
        let object = SceneObject::new(mesh.clone(), transform, 0xFFFFFFFF);
        scene.add_object(object);
    }

    let mut renderer = TileRenderer::new(width, height);
    let mut fb = Framebuffer::new(width, height).unwrap();
    let mut zb = ZBuffer::new(width, height).unwrap();

    c.bench_function("scene_render_100_objects", |b| {
        b.iter(|| {
            fb.clear(0xFF000000);
            zb.clear();
            scene.render(&mut renderer, &mut fb, &mut zb);
        });
    });
}

criterion_group!(benches, bench_scene_render);
criterion_main!(benches);
