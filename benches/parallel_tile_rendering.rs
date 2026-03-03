use abrash::framebuffer::Framebuffer;
use abrash::math::{Mat4, Vec3};
use abrash::mesh::Mesh;
use abrash::rasterizer::{ClipTriangle, TileRenderer};
use abrash::zbuffer::ZBuffer;
use criterion::{BenchmarkId, Criterion, black_box, criterion_group, criterion_main};

/// Setup a cube's transformed vertices and MVP for consistent benchmarks
#[allow(dead_code)]
fn setup_cube(aspect: f32) -> (Mesh, Vec<(Vec3, f32)>) {
    let mesh = Mesh::cube(2.0);
    let model = Mat4::rotation_x(0.5) * Mat4::rotation_y(0.5);
    let view = Mat4::look_at(
        Vec3::new(0.0, 0.0, -5.0),
        Vec3::new(0.0, 0.0, 0.0),
        Vec3::new(0.0, 1.0, 0.0),
    );
    let projection = Mat4::perspective(1.57, aspect, 0.1, 100.0);
    let mvp = model * view * projection;

    let transformed: Vec<(Vec3, f32)> = mesh
        .vertices
        .iter()
        .map(|v| mvp.transform_point(*v))
        .collect();

    (mesh, transformed)
}

/// Generate overlapping triangles that all hit roughly the same screen region.
fn generate_overlapping_triangles(count: usize) -> Vec<ClipTriangle> {
    let mut tris = Vec::with_capacity(count);
    for i in 0..count {
        let t = i as f32 * 0.1;
        let offset_x = (t * 1.7).sin() * 0.15;
        let offset_y = (t * 2.3).cos() * 0.15;
        let depth = 3.0 + (t * 0.5).sin() * 2.0;

        let v0 = (Vec3::new(offset_x, 0.5 + offset_y, depth), depth);
        let v1 = (Vec3::new(-0.5 + offset_x, -0.5 + offset_y, depth), depth);
        let v2 = (Vec3::new(0.5 + offset_x, -0.5 + offset_y, depth), depth);

        let r = ((i * 37) % 256) as u32;
        let g = ((i * 73) % 256) as u32;
        let b = ((i * 113) % 256) as u32;
        let color = 0xFF00_0000 | (r << 16) | (g << 8) | b;

        tris.push((v0, v1, v2, color));
    }
    tris
}

fn bench_parallel_tile_rendering(c: &mut Criterion) {
    let width = 1920;
    let height = 1080;
    let triangle_counts = [100, 500, 1000];

    let mut group = c.benchmark_group("parallel_tile_rendering");

    for &count in &triangle_counts {
        let tris = generate_overlapping_triangles(count);
        let mut fb = Framebuffer::new(width, height).unwrap();
        let mut zb = ZBuffer::new(width, height).unwrap();
        let mut tr = TileRenderer::new(width, height);

        group.bench_with_input(BenchmarkId::from_parameter(count), &tris, |b, tris| {
            b.iter(|| {
                zb.clear();
                // This will use parallel rendering if the feature is enabled
                tr.render_batch(&mut fb, &mut zb, black_box(tris));
            });
        });
    }
    group.finish();
}

criterion_group!(benches, bench_parallel_tile_rendering);
criterion_main!(benches);
