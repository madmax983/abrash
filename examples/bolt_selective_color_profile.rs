//! Deterministic profiling harness for `examples/selective_color_demo.rs`'s
//! per-frame work: render the demo's 800x600 scene (four lit cubes on a dark
//! grey background, same camera/lighting) with `fill_triangle_lit`, then
//! `apply_selective_color` with the demo's config (tolerance 45, animated
//! target hue), for N frames (default 10).
//!
//! Not a criterion benchmark: wall-clock is unreliable on this machine. Run
//! under `valgrind --tool=callgrind --toggle-collect='*apply_selective_color*'`
//! to count only the effect, or without the toggle for the whole frame.
use abrash::framebuffer::Framebuffer;
use abrash::math::{Mat4, Vec3};
use abrash::mesh::Mesh;
use abrash::rasterizer::fill_triangle_lit;
use abrash::zbuffer::ZBuffer;
use abrash_render::experimental::selective_color::{SelectiveColorConfig, apply_selective_color};
use std::f32::consts::PI;
use std::hint::black_box;

const WIDTH: u32 = 800;
const HEIGHT: u32 = 600;

fn main() {
    let frames: usize = std::env::args()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(10);

    let mesh = Mesh::cube(1.0);
    let _ = mesh.compute_face_normals();
    let mut fb = Framebuffer::new(WIDTH, HEIGHT).unwrap();
    let mut zb = ZBuffer::new(WIDTH, HEIGHT).unwrap();
    let light_dir = Vec3::new(-1.0, 1.0, -1.0).normalize();
    let mut config = SelectiveColorConfig {
        target_hue: 0.0,
        tolerance: 45.0,
        desaturation: 1.0,
    };
    let (mut rot_x, mut rot_y) = (0.0_f32, 0.0_f32);
    let mut checksum: u64 = 0;

    for _ in 0..frames {
        rot_y += 0.02;
        rot_x += 0.015;
        config.target_hue = (rot_y * 50.0) % 360.0;

        fb.clear(0xFF11_1111);
        zb.clear();
        let proj = Mat4::perspective(PI / 3.0, WIDTH as f32 / HEIGHT as f32, 0.1, 100.0);
        let view = Mat4::look_at(
            Vec3::new(0.0, 0.0, 5.0),
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
        );
        let view_proj = view * proj;
        let cubes = [
            (Vec3::new(-1.5, 0.0, 0.0), 0xFFFF_0000_u32),
            (Vec3::new(1.5, 0.0, 0.0), 0xFF00_FF00_u32),
            (Vec3::new(0.0, 1.5, 0.0), 0xFF00_00FF_u32),
            (Vec3::new(0.0, -1.5, 0.0), 0xFFFF_FF00_u32),
        ];
        for (pos, color) in &cubes {
            let model = Mat4::translation(pos.x, pos.y, pos.z)
                * Mat4::rotation_x(rot_x)
                * Mat4::rotation_y(rot_y);
            let mvp = model * view_proj;
            let inv_model = model.inverse();
            let local_light_dir = inv_model.transform_vector(light_dir).normalize();
            for tri in &mesh.indices {
                let (v0, v1, v2) = (
                    mesh.vertices[tri[0]],
                    mesh.vertices[tri[1]],
                    mesh.vertices[tri[2]],
                );
                let (c0, c1, c2) = (
                    mvp.transform_point(v0),
                    mvp.transform_point(v1),
                    mvp.transform_point(v2),
                );
                let n = (v1 - v0).cross(v2 - v0).normalize();
                let view_dir = inv_model.transform_point(Vec3::new(0.0, 0.0, 5.0)).0 - v0;
                if n.dot(view_dir) > 0.0 {
                    fill_triangle_lit(
                        &mut fb,
                        &mut zb,
                        (c0.0, c0.1),
                        (c1.0, c1.1),
                        (c2.0, c2.1),
                        n,
                        Vec3::new(
                            (((*color >> 16) & 0xFF) as f32) / 255.0,
                            (((*color >> 8) & 0xFF) as f32) / 255.0,
                            ((*color & 0xFF) as f32) / 255.0,
                        ),
                        Vec3::new(0.2, 0.2, 0.2),
                        local_light_dir,
                        Vec3::new(1.0, 1.0, 1.0),
                    );
                }
            }
        }
        apply_selective_color(&mut fb, &config);
        checksum = fb.as_slice().iter().fold(checksum, |a, &p| {
            a.wrapping_mul(31).wrapping_add(u64::from(p))
        });
    }
    black_box(checksum);
    println!("checksum {checksum}");
}
