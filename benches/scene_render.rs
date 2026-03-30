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
        let object = SceneObject::new(mesh.clone(), transform, 0xFFFF_FFFF);
        scene.add_object(object);
    }

    let mut renderer = TileRenderer::new(width, height);
    let mut fb = Framebuffer::new(width, height).unwrap();
    let mut zb = ZBuffer::new(width, height).unwrap();

    c.bench_function("scene_render_100_objects", |b| {
        b.iter(|| {
            fb.clear(0xFF00_0000);
            zb.clear();
            scene.render(&mut renderer, &mut fb, &mut zb);
        });
    });
}

fn build_scene(width: u32, height: u32) -> Scene {
    let mesh = Arc::new(generate_grid_mesh(10));
    let view = Mat4::look_at(
        Vec3::new(0.0, 50.0, 50.0),
        Vec3::new(0.0, 0.0, 0.0),
        Vec3::new(0.0, 1.0, 0.0),
    );
    let proj = Mat4::perspective(1.0, width as f32 / height as f32, 0.1, 1000.0);
    let mut scene = Scene::new(Camera::new(view, proj));
    for i in 0..100 {
        let x = (i % 10) as f32 * 15.0 - 75.0;
        let z = (i / 10) as f32 * 15.0 - 75.0;
        scene.add_object(SceneObject::new(
            mesh.clone(),
            Mat4::translation(x, 0.0, z),
            0xFFFF_FFFF,
        ));
    }
    scene
}

fn bench_extract_only(c: &mut Criterion) {
    let scene = build_scene(640, 480);
    c.bench_function("scene_extract_only_100_objects", |b| {
        b.iter(|| scene.extract());
    });
}

fn bench_clear_only(c: &mut Criterion) {
    let width = 640;
    let height = 480;
    let mut fb = Framebuffer::new(width, height).unwrap();
    let mut zb = ZBuffer::new(width, height).unwrap();
    c.bench_function("scene_clear_only", |b| {
        b.iter(|| {
            fb.clear(0xFF00_0000);
            zb.clear();
        });
    });
}

fn bench_rasterize_only(c: &mut Criterion) {
    let width = 640;
    let height = 480;
    let scene = build_scene(width, height);
    let draw_list = scene.extract();
    let mut renderer = TileRenderer::new(width, height);
    let mut fb = Framebuffer::new(width, height).unwrap();
    let mut zb = ZBuffer::new(width, height).unwrap();
    c.bench_function("scene_rasterize_only_100_objects", |b| {
        b.iter(|| {
            renderer.begin_frame();
            for batch in &draw_list.batches {
                renderer.submit_mesh(&batch.indices, &batch.vertices, batch.color);
            }
            renderer.end_frame(&mut fb, &mut zb);
        });
    });
}

/// Measures `submit_mesh` only (`prepare_triangle` × 20K) — no binning or rasterization.
fn bench_submit_only(c: &mut Criterion) {
    let width = 640;
    let height = 480;
    let scene = build_scene(width, height);
    let draw_list = scene.extract();
    let mut renderer = TileRenderer::new(width, height);
    c.bench_function("submit_mesh_only_20k_tris", |b| {
        b.iter(|| {
            renderer.begin_frame();
            for batch in &draw_list.batches {
                renderer.submit_mesh(&batch.indices, &batch.vertices, batch.color);
            }
            // Skip end_frame — isolates clipping + projection + backface cull
        });
    });
}

/// Full pipeline with integrated tile-level clearing (no separate fb.clear + zb.clear).
fn bench_scene_render_integrated_clear(c: &mut Criterion) {
    let width = 640;
    let height = 480;
    let scene = build_scene(width, height);
    let mut renderer = TileRenderer::new(width, height);
    renderer.set_clear_color(Some(0xFF00_0000));
    let mut fb = Framebuffer::new(width, height).unwrap();
    let mut zb = ZBuffer::new(width, height).unwrap();
    c.bench_function("scene_render_integrated_clear_100_objects", |b| {
        b.iter(|| {
            scene.render(&mut renderer, &mut fb, &mut zb);
        });
    });
}

/// Rasterize with integrated clear — measures `end_frame` clearing ALL tiles.
fn bench_rasterize_integrated_clear(c: &mut Criterion) {
    let width = 640;
    let height = 480;
    let scene = build_scene(width, height);
    let draw_list = scene.extract();
    let mut renderer = TileRenderer::new(width, height);
    renderer.set_clear_color(Some(0xFF00_0000));
    let mut fb = Framebuffer::new(width, height).unwrap();
    let mut zb = ZBuffer::new(width, height).unwrap();
    c.bench_function("scene_rasterize_integrated_clear_100_objects", |b| {
        b.iter(|| {
            renderer.begin_frame();
            for batch in &draw_list.batches {
                renderer.submit_mesh(&batch.indices, &batch.vertices, batch.color);
            }
            renderer.end_frame(&mut fb, &mut zb);
        });
    });
}

criterion_group!(
    benches,
    bench_scene_render,
    bench_scene_render_integrated_clear,
    bench_extract_only,
    bench_clear_only,
    bench_rasterize_only,
    bench_rasterize_integrated_clear,
    bench_submit_only
);
criterion_main!(benches);
