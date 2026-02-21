use abrash::framebuffer::Framebuffer;
use abrash::geometry::mesh::Mesh;
use abrash::math::{Mat4, Vec3};
use abrash::platform::wasm::run_wasm;
use abrash::rasterizer::fill_triangle_3d;
use abrash::time::FixedTimestep;
use abrash::zbuffer::ZBuffer;
use std::cell::RefCell;
use std::f32::consts::PI;
use std::rc::Rc;

const WIDTH: u32 = 800;
const HEIGHT: u32 = 600;
const BACKGROUND: u32 = 0xFF00_0000;

// Face colors for the cube
const COLORS: [u32; 6] = [
    0xFFFF_0000, // Red - front
    0xFF00_FF00, // Green - back
    0xFF00_00FF, // Blue - top
    0xFFFF_FF00, // Yellow - bottom
    0xFFFF_00FF, // Magenta - right
    0xFF00_FFFF, // Cyan - left
];

fn main() {
    let cube = Mesh::cube(1.0);

    // Camera setup
    let projection = Mat4::perspective(PI / 3.0, WIDTH as f32 / HEIGHT as f32, 0.1, 100.0);
    let view = Mat4::look_at(
        Vec3::new(0.0, 1.5, 3.0), // eye
        Vec3::new(0.0, 0.0, 0.0), // target
        Vec3::new(0.0, 1.0, 0.0), // up
    );

    let angle_y: Rc<RefCell<f32>> = Rc::new(RefCell::new(0.0));
    let angle_x: Rc<RefCell<f32>> = Rc::new(RefCell::new(0.0));
    let timestep: Rc<RefCell<FixedTimestep>> = Rc::new(RefCell::new(FixedTimestep::new(60)));

    run_wasm(
        "Abrash - WASM 3D Cube",
        WIDTH,
        HEIGHT,
        move |framebuffer: &mut Framebuffer, zbuffer: &mut ZBuffer, _events: &[_]| {
            let mut ts = timestep.borrow_mut();
            let steps = ts.update();
            let dt = ts.dt();

            let mut ay = angle_y.borrow_mut();
            let mut ax = angle_x.borrow_mut();
            for _ in 0..steps {
                *ay += 1.0 * dt;
                *ax += 0.5 * dt;
            }

            framebuffer.clear(BACKGROUND);
            zbuffer.clear();

            // Model matrix (rotation)
            let model = Mat4::rotation_y(*ay) * Mat4::rotation_x(*ax);

            // MVP matrix
            let mvp = projection * (view * model);

            // Transform and render each triangle
            for (face_idx, tri_indices) in cube.indices.iter().enumerate() {
                let v0 = cube.vertices[tri_indices[0]];
                let v1 = cube.vertices[tri_indices[1]];
                let v2 = cube.vertices[tri_indices[2]];

                // Transform vertices
                let (clip0, w0) = mvp.transform_point(v0);
                let (clip1, w1) = mvp.transform_point(v1);
                let (clip2, w2) = mvp.transform_point(v2);

                // Skip if all w values are negative (behind camera)
                if w0 < 0.0 && w1 < 0.0 && w2 < 0.0 {
                    continue;
                }

                let color = COLORS[face_idx / 2]; // 2 triangles per face
                fill_triangle_3d(
                    framebuffer,
                    zbuffer,
                    (clip0, w0),
                    (clip1, w1),
                    (clip2, w2),
                    color,
                );
            }
        },
    );
}
