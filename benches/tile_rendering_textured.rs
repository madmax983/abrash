use abrash::framebuffer::Framebuffer;
use abrash::math::{Mat4, Vec2, Vec3};
use abrash::mesh::Mesh;
use abrash::rasterizer::fill_triangle_textured;
use abrash::rasterizer::{TexturedClipTriangle, TileRenderer};
use abrash::texture::Texture;
use abrash::zbuffer::ZBuffer;
use criterion::{BenchmarkId, Criterion, black_box, criterion_group, criterion_main};

/// Setup a cube's transformed vertices and MVP for consistent benchmarks
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
fn generate_overlapping_triangles(count: usize) -> Vec<TexturedClipTriangle> {
    let mut tris = Vec::with_capacity(count);
    for i in 0..count {
        let t = i as f32 * 0.1;
        let offset_x = (t * 1.7).sin() * 0.15;
        let offset_y = (t * 2.3).cos() * 0.15;
        let depth = 3.0 + (t * 0.5).sin() * 2.0;

        let v0 = (Vec3::new(offset_x, 0.5 + offset_y, depth), depth);
        let uv0 = Vec2::new(0.0, 0.0);

        let v1 = (Vec3::new(-0.5 + offset_x, -0.5 + offset_y, depth), depth);
        let uv1 = Vec2::new(0.0, 1.0);

        let v2 = (Vec3::new(0.5 + offset_x, -0.5 + offset_y, depth), depth);
        let uv2 = Vec2::new(1.0, 0.0);

        tris.push((v0, uv0, v1, uv1, v2, uv2));
    }
    tris
}

// --- 800x600 A/B comparisons (kept for regression tracking) ---

fn bench_cube_tiled_textured(c: &mut Criterion) {
    let (mesh, transformed) = setup_cube(800.0 / 600.0);
    let mut fb = Framebuffer::new(800, 600).unwrap();
    let mut zb = ZBuffer::new(800, 600).unwrap();
    let mut tr = TileRenderer::new(800, 600);
    let texture = Texture::new(32, 32).unwrap();

    let batch: Vec<TexturedClipTriangle> = mesh
        .indices
        .iter()
        .map(|idx| {
            (
                transformed[idx[0]],
                Vec2::new(0.0, 0.0),
                transformed[idx[1]],
                Vec2::new(0.0, 1.0),
                transformed[idx[2]],
                Vec2::new(1.0, 0.0),
            )
        })
        .collect();

    c.bench_function("fill_cube_tiled_textured", |b| {
        b.iter(|| {
            zb.clear();
            tr.render_batch_textured(&mut fb, &mut zb, black_box(&batch), &texture);
        });
    });
}

fn bench_cube_scanline_textured(c: &mut Criterion) {
    let (mesh, transformed) = setup_cube(800.0 / 600.0);
    let mut fb = Framebuffer::new(800, 600).unwrap();
    let mut zb = ZBuffer::new(800, 600).unwrap();
    let texture = Texture::new(32, 32).unwrap();

    // Use dummy UVs
    let uvs = [
        Vec2::new(0.0, 0.0),
        Vec2::new(0.0, 1.0),
        Vec2::new(1.0, 0.0),
    ];

    c.bench_function("fill_cube_scanline_textured", |b| {
        b.iter(|| {
            zb.clear();
            for indices in &mesh.indices {
                fill_triangle_textured(
                    &mut fb,
                    &mut zb,
                    (black_box(transformed[indices[0]]), uvs[0]),
                    (black_box(transformed[indices[1]]), uvs[1]),
                    (black_box(transformed[indices[2]]), uvs[2]),
                    &texture,
                );
            }
        });
    });
}

// --- Resolution x triangle-count crossover sweep ---
// Measures ONLY rasterization cost (no zb.clear in the loop).

struct Resolution {
    name: &'static str,
    width: u32,
    height: u32,
}

const RESOLUTIONS: &[Resolution] = &[
    Resolution {
        name: "800x600",
        width: 800,
        height: 600,
    },
    Resolution {
        name: "1920x1080",
        width: 1920,
        height: 1080,
    },
    Resolution {
        name: "3840x2160",
        width: 3840,
        height: 2160,
    },
];

const TRIANGLE_COUNTS: &[usize] = &[10, 50, 100, 200, 500];

fn bench_crossover_tiled_textured_multi_res(c: &mut Criterion) {
    let tris_by_count: Vec<(usize, Vec<TexturedClipTriangle>)> = TRIANGLE_COUNTS
        .iter()
        .map(|&n| (n, generate_overlapping_triangles(n)))
        .collect();

    let texture = Texture::new(32, 32).unwrap();

    for res in RESOLUTIONS {
        let mut group = c.benchmark_group(format!("tiled_textured_{}", res.name));
        let mut fb = Framebuffer::new(res.width, res.height).unwrap();
        let mut zb = ZBuffer::new(res.width, res.height).unwrap();
        let mut tr = TileRenderer::new(res.width, res.height);

        for (count, tris) in &tris_by_count {
            group.bench_with_input(BenchmarkId::from_parameter(count), tris, |b, tris| {
                b.iter(|| {
                    zb.clear();
                    tr.render_batch_textured(&mut fb, &mut zb, black_box(tris), &texture);
                });
            });
        }
        group.finish();
    }
}

fn bench_crossover_scanline_textured_multi_res(c: &mut Criterion) {
    let tris_by_count: Vec<(usize, Vec<TexturedClipTriangle>)> = TRIANGLE_COUNTS
        .iter()
        .map(|&n| (n, generate_overlapping_triangles(n)))
        .collect();

    let texture = Texture::new(32, 32).unwrap();

    for res in RESOLUTIONS {
        let mut group = c.benchmark_group(format!("scanline_textured_{}", res.name));
        let mut fb = Framebuffer::new(res.width, res.height).unwrap();
        let mut zb = ZBuffer::new(res.width, res.height).unwrap();

        for (count, tris) in &tris_by_count {
            group.bench_with_input(BenchmarkId::from_parameter(count), tris, |b, tris| {
                b.iter(|| {
                    zb.clear();
                    for &(v0, uv0, v1, uv1, v2, uv2) in black_box(tris) {
                        fill_triangle_textured(
                            &mut fb,
                            &mut zb,
                            (v0, uv0),
                            (v1, uv1),
                            (v2, uv2),
                            &texture,
                        );
                    }
                });
            });
        }
        group.finish();
    }
}

criterion_group!(
    benches,
    bench_cube_tiled_textured,
    bench_cube_scanline_textured,
    bench_crossover_tiled_textured_multi_res,
    bench_crossover_scanline_textured_multi_res,
);
criterion_main!(benches);
