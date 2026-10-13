//! Deterministic profiling harness for `examples/cloth_demo.rs`'s per-frame
//! work: Verlet-integration physics (`Cloth::update`, 5 sub-steps per frame,
//! matching the demo exactly), mesh rebuild (`Cloth::to_mesh`), and the
//! CPU-side shading + rasterization loop that projects and fills every
//! triangle into a framebuffer.
//!
//! Not a criterion benchmark: wall-clock is unreliable on this machine. This
//! binary is meant to be run under `valgrind --tool=callgrind` /
//! `--tool=dhat` to get deterministic instruction and allocation counts for a
//! realistic workload — the demo's actual 20x20 pinned-corner cloth grid
//! (800x600 framebuffer, the demo's exact camera/projection), driven for a
//! deterministic sequence of frames with wind varying every frame so no two
//! frames do identical work (avoids constant-folding).
//!
//! Mirrors `cloth_demo.rs`'s `run()` loop exactly for the CPU-side work: a
//! fixed `dt` (no wall clock), the same `sub_steps = 5`, the same wind
//! oscillation formula, `Cloth::to_mesh()`, then the same per-triangle
//! normal/shading/projection/rasterization sequence. The windowing/blit step
//! is skipped (no display in this environment) — everything up to and
//! including `fill_triangle_3d` is reproduced faithfully.
use abrash::framebuffer::Framebuffer;
use abrash::math::{Mat4, Vec3};
use abrash::rasterizer::fill_triangle_3d;
use abrash::zbuffer::ZBuffer;
use abrash_render::experimental::cloth::Cloth;
use std::f32::consts::PI;
use std::hint::black_box;

const WIDTH: usize = 800;
const HEIGHT: usize = 600;

fn main() {
    let frames: usize = std::env::args()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(60);

    let mut fb = Framebuffer::new(WIDTH as u32, HEIGHT as u32).unwrap();
    let mut zb = ZBuffer::new(WIDTH as u32, HEIGHT as u32).unwrap();

    // Exactly cloth_demo.rs: 20x20 grid, spacing 0.2, corners + top-middle pinned.
    let mut cloth = Cloth::new(20, 20, 0.2);
    cloth.pin(0, 0);
    cloth.pin(0, cloth.height - 1);
    cloth.pin(cloth.width - 1, cloth.height - 1);
    cloth.pin(cloth.width / 2, cloth.height - 1);

    let gravity = Vec3::new(0.0, -9.8, 0.0);
    let mut wind = Vec3::new(0.0, 0.0, 2.0);

    // Fixed dt in place of Instant-based wall clock: a typical 60fps frame.
    let dt: f32 = 1.0 / 60.0;
    let mut time: f32 = 0.0;

    let proj = Mat4::perspective(PI / 3.0, WIDTH as f32 / HEIGHT as f32, 0.1, 100.0);
    let view = Mat4::look_at(
        Vec3::new(0.0, 2.0, 6.0),
        Vec3::new(0.0, 0.0, 0.0),
        Vec3::new(0.0, 1.0, 0.0),
    );
    let view_proj = view * proj;

    let mut checksum: u64 = 0;

    for _frame in 0..frames {
        time += dt;
        wind.x = (time * 2.0).sin() * 2.0;
        wind.z = 2.0 + (time * 1.5).cos() * 1.0;

        let sub_steps = 5;
        let sub_dt = dt.min(0.032) / sub_steps as f32;
        for _ in 0..sub_steps {
            cloth.update(sub_dt, gravity, wind);
        }

        fb.clear(0xFF10_1010);
        zb.clear();

        let mesh = cloth.to_mesh();

        let light_dir = Vec3::new(0.5, 1.0, 1.0).normalize();

        for tri in &mesh.indices {
            let i0 = tri[0];
            let i1 = tri[1];
            let i2 = tri[2];

            let v0 = mesh.vertices[i0];
            let v1 = mesh.vertices[i1];
            let v2 = mesh.vertices[i2];

            let edge1 = v1 - v0;
            let edge2 = v2 - v0;
            let normal = edge1.cross(edge2).normalize();

            let ndotl = normal.dot(light_dir).max(0.1);

            let r = (170.0 * ndotl) as u32;
            let g = (20.0 * ndotl) as u32;
            let b = (20.0 * ndotl) as u32;
            let color = 0xFF00_0000 | (r << 16) | (g << 8) | b;

            let (c0, w0) = view_proj.transform_point(v0);
            let (c1, w1) = view_proj.transform_point(v1);
            let (c2, w2) = view_proj.transform_point(v2);

            if w0 < 0.1 || w1 < 0.1 || w2 < 0.1 {
                continue;
            }

            fill_triangle_3d(&mut fb, &mut zb, (c0, w0), (c1, w1), (c2, w2), color);
        }

        black_box(&fb);
        checksum = checksum.wrapping_add(
            cloth
                .particles
                .iter()
                .fold(0u64, |acc: u64, p| acc.wrapping_add(p.pos.x.to_bits() as u64)),
        );
    }

    println!("frames={frames} checksum={checksum}");
}
