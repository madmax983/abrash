use abrash::experimental::hologram::{HologramConfig, apply_hologram};
use abrash::framebuffer::Framebuffer;
use abrash::math::{Mat4, Vec3};
use abrash::rasterizer::fill_triangle_3d;
use abrash::zbuffer::ZBuffer;

fn main() {
    let width = 800;
    let height = 600;
    let mut fb = Framebuffer::new(width, height).unwrap();
    let mut zb = ZBuffer::new(width, height).unwrap();

    let eye = Vec3::new(0.0, 0.0, 5.0);
    let target = Vec3::new(0.0, 0.0, 0.0);
    let up = Vec3::new(0.0, 1.0, 0.0);
    let view = Mat4::look_at(eye, target, up);
    let proj = Mat4::perspective(1.57, width as f32 / height as f32, 0.1, 100.0);
    let view_proj = view * proj;

    fb.clear(0xFF000000);
    zb.clear();

    let v0_local = Vec3::new(0.0, 1.0, 0.0);
    let v1_local = Vec3::new(-1.0, -1.0, 0.0);
    let v2_local = Vec3::new(1.0, -1.0, 0.0);

    let v0_clip = view_proj.transform_point(v0_local);
    let v1_clip = view_proj.transform_point(v1_local);
    let v2_clip = view_proj.transform_point(v2_local);

    fill_triangle_3d(&mut fb, &mut zb, v0_clip, v1_clip, v2_clip, 0xFFFFFFFF);

    let config = HologramConfig {
        color: 0xFF00FFFF, // Cyan
        time: 1.0,
        scanline_intensity: 0.8,
        flicker_speed: 15.0,
        flicker_intensity: 0.3,
        interference_strength: 0.05,
    };

    apply_hologram(&mut fb, &config);

    // Normally this would be rendered to a window, but this is a minimal example
    // just to show it compiles and runs.
    println!("Hologram demo ran successfully!");
}
