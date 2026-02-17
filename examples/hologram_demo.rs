use abrash::experimental::hologram::{HologramParams, draw_holographic_mesh};
use abrash::framebuffer::Framebuffer;
use abrash::math::{Mat4, Vec3};
use abrash::mesh::Mesh;
use abrash::platform::{Window, WindowBackend};
use abrash::time::FixedTimestep;
use abrash::zbuffer::ZBuffer;
use std::f32::consts::PI;

const WIDTH: u32 = 800;
const HEIGHT: u32 = 600;
const BACKGROUND: u32 = 0xFF00_0000; // Black background

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut window = Window::new("Abrash - Holographic Projection", WIDTH, HEIGHT)?;
    let mut framebuffer = Framebuffer::new(WIDTH, HEIGHT)?;
    let mut zbuffer = ZBuffer::new(WIDTH, HEIGHT)?;
    let mut timestep = FixedTimestep::new(60);

    // Create a mesh (Cube)
    // We could load a model, but a cube is fine for a demo.
    let mesh = Mesh::cube(2.0);

    // Camera setup
    let projection = Mat4::perspective(PI / 3.0, WIDTH as f32 / HEIGHT as f32, 0.1, 100.0);
    let view = Mat4::look_at(
        Vec3::new(0.0, 2.0, 4.0), // eye
        Vec3::new(0.0, 0.0, 0.0), // target
        Vec3::new(0.0, 1.0, 0.0), // up
    );

    let mut angle_y: f32 = 0.0;
    let mut angle_x: f32 = 0.0;
    let mut time: f32 = 0.0;

    let mut params = HologramParams::default();
    params.color = 0x0000FFFF; // Cyan
    params.intensity = 0.5;
    params.scan_speed = 1.0;
    params.scan_density = 20.0;
    params.jitter_amount = 0.02;

    while window.is_open() {
        window.poll_events();

        let steps = timestep.update();
        for _ in 0..steps {
            let dt = timestep.dt();
            angle_y += 0.5 * dt;
            angle_x += 0.3 * dt;
            time += dt;
        }

        // Update params
        params.time = time;
        // Pulse intensity
        params.intensity = 0.5 + 0.2 * (time * 2.0).sin();
        // Occasional heavy glitch
        if (time * 5.0).sin() > 0.95 {
            params.jitter_amount = 0.2; // GLITCH!
        } else {
            params.jitter_amount = 0.01;
        }

        framebuffer.clear(BACKGROUND);
        zbuffer.clear();

        // Model matrix (rotation)
        let model = Mat4::rotation_y(angle_y) * Mat4::rotation_x(angle_x);

        // MVP matrix
        let mvp = projection * (view * model);

        // Render Hologram
        // We render it twice slightly offset for a "ghosting" effect?
        // Let's just render once first.
        draw_holographic_mesh(
            &mut framebuffer,
            &mut zbuffer,
            &mesh,
            mvp,
            params,
        );

        window.blit_framebuffer(&framebuffer);
    }

    Ok(())
}
