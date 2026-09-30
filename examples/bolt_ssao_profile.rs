//! Deterministic profiling harness for the SSAO demo's per-frame work
//! (`examples/ssao_demo.rs`): the same 800x600 scene (floor + 4 cubes, same
//! camera/projection/lighting) rendered with `fill_triangle_lit`, followed by
//! `apply_ssao` with the demo's config, for N frames (default 10).
//!
//! Not a criterion benchmark: meant to be run under
//! `valgrind --tool=callgrind` / `--tool=dhat` for deterministic counters.
use abrash::framebuffer::Framebuffer;
use abrash::math::{Mat4, Vec3};
use abrash::mesh::Mesh;
use abrash::post_process::{apply_ssao, ssao::SsaoConfig};
use abrash::rasterizer::fill_triangle_lit;
use abrash::zbuffer::ZBuffer;
use std::f32::consts::PI;
use std::hint::black_box;

const WIDTH: u32 = 800;
const HEIGHT: u32 = 600;

fn main() {
    let frames: usize = std::env::args()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(10);

    let cube = Mesh::cube(1.0);
    let cube_normals = cube.compute_face_normals();
    let mut floor = Mesh::new();
    let h = 5.0;
    floor.vertices.push(Vec3::new(-h, 0.0, h));
    floor.vertices.push(Vec3::new(h, 0.0, h));
    floor.vertices.push(Vec3::new(h, 0.0, -h));
    floor.vertices.push(Vec3::new(-h, 0.0, -h));
    floor.indices.push([2, 1, 0]);
    floor.indices.push([3, 2, 0]);
    let floor_normals = floor.compute_face_normals();

    let mut fb = Framebuffer::new(WIDTH, HEIGHT).unwrap();
    let mut zb = ZBuffer::new(WIDTH, HEIGHT).unwrap();
    let projection = Mat4::perspective(PI / 3.0, WIDTH as f32 / HEIGHT as f32, 0.1, 100.0);
    let view = Mat4::look_at(
        Vec3::new(0.0, 3.0, 5.0),
        Vec3::new(0.0, 0.0, 0.0),
        Vec3::new(0.0, 1.0, 0.0),
    );
    let ambient = Vec3::new(0.2, 0.2, 0.2);
    let sun_dir = Vec3::new(-0.5, -1.0, -0.3).normalize();
    let sun_color = Vec3::new(0.8, 0.8, 0.8);
    let floor_color = Vec3::new(0.6, 0.6, 0.6);
    let cube_color = Vec3::new(0.9, 0.4, 0.3);
    let cube_positions = [
        Vec3::new(0.0, 0.0, 0.0),
        Vec3::new(-1.2, 0.0, 0.5),
        Vec3::new(1.2, 0.0, -0.5),
        Vec3::new(0.0, 1.0, 0.0),
    ];
    let cfg = SsaoConfig {
        radius: 0.5,
        bias: 0.025,
        intensity: 2.0,
    };

    for _ in 0..frames {
        fb.clear(0xFF1A_1A2E);
        zb.clear();

        let model_floor = Mat4::translation(0.0, -0.5, 0.0);
        let mvp_floor = projection * (view * model_floor);
        for (i, tri) in floor.indices.iter().enumerate() {
            let [a, b, c] = *tri;
            fill_triangle_lit(
                &mut fb,
                &mut zb,
                mvp_floor.transform_point(floor.vertices[a]),
                mvp_floor.transform_point(floor.vertices[b]),
                mvp_floor.transform_point(floor.vertices[c]),
                model_floor.transform_normal(floor_normals[i]),
                floor_color,
                ambient,
                sun_dir,
                sun_color,
            );
        }
        for pos in &cube_positions {
            let model = Mat4::translation(pos.x, pos.y, pos.z);
            let mvp = projection * (view * model);
            for (i, tri) in cube.indices.iter().enumerate() {
                let [a, b, c] = *tri;
                fill_triangle_lit(
                    &mut fb,
                    &mut zb,
                    mvp.transform_point(cube.vertices[a]),
                    mvp.transform_point(cube.vertices[b]),
                    mvp.transform_point(cube.vertices[c]),
                    model.transform_normal(cube_normals[i]),
                    cube_color,
                    ambient,
                    sun_dir,
                    sun_color,
                );
            }
        }
        apply_ssao(&mut fb, &zb, &projection, &cfg);
    }
    black_box(fb.as_slice().iter().fold(0u32, |a, &p| a ^ p));
}
