use abrash_core::framebuffer::Framebuffer;
use abrash_core::math::{Mat4, Vec3};
use abrash_core::zbuffer::ZBuffer;
use abrash_render::experimental::paper_cutout::{PaperCutoutConfig, apply_paper_cutout};
use abrash_render::rasterizer::fill_triangle_3d;
use std::fs::File;
use std::io::Write;

fn main() {
    let width = 800;
    let height = 600;
    let mut fb = Framebuffer::new(width, height).unwrap();
    let mut zb = ZBuffer::new(width, height).unwrap();

    // Clear to a light background color
    fb.clear(0xFFF0F0F0);
    zb.clear();

    // Set up a simple orthographic-ish camera or perspective
    let view = Mat4::look_at(
        Vec3::new(0.0, 0.0, -10.0), // eye
        Vec3::new(0.0, 0.0, 0.0),   // target
        Vec3::new(0.0, 1.0, 0.0),   // up
    );
    let proj = Mat4::perspective(1.0, width as f32 / height as f32, 0.1, 100.0);
    let view_proj = view * proj;

    // Draw some layered triangles (mountains or trees)
    let draw_tri = |fb: &mut Framebuffer,
                    zb: &mut ZBuffer,
                    z: f32,
                    color: u32,
                    size: f32,
                    x_off: f32,
                    y_off: f32| {
        let v0 = view_proj.transform_point(Vec3::new(x_off, size + y_off, z));
        let v1 = view_proj.transform_point(Vec3::new(-size + x_off, -size + y_off, z));
        let v2 = view_proj.transform_point(Vec3::new(size + x_off, -size + y_off, z));
        fill_triangle_3d(fb, zb, v0, v1, v2, color);
    };

    // Deepest layer
    draw_tri(&mut fb, &mut zb, 8.0, 0xFF4488FF, 5.0, -2.0, 2.0); // Blue mountain
    draw_tri(&mut fb, &mut zb, 7.5, 0xFF5599FF, 4.0, 3.0, 1.0); // Lighter blue mountain

    // Mid layer
    draw_tri(&mut fb, &mut zb, 5.0, 0xFF22AA22, 3.0, -3.0, -1.0); // Green tree
    draw_tri(&mut fb, &mut zb, 4.5, 0xFF33BB33, 2.5, 1.0, -2.0); // Green tree

    // Near layer
    draw_tri(&mut fb, &mut zb, 2.0, 0xFFDD4444, 1.5, 0.0, -3.0); // Red house/shape

    // Apply the Paper Cutout filter
    let config = PaperCutoutConfig {
        layers: 5,
        shadow_offset_x: 10,
        shadow_offset_y: 15,
        shadow_opacity: 0.6,
        outline_color: 0x00000000,
    };

    apply_paper_cutout(&mut fb, &zb, &config);

    // Save out an image
    let mut file = File::create("paper_cutout_demo.ppm").unwrap();
    write!(file, "P3\n{} {}\n255\n", width, height).unwrap();
    for pixel in fb.as_slice() {
        let r = (pixel >> 16) & 0xFF;
        let g = (pixel >> 8) & 0xFF;
        let b = pixel & 0xFF;
        write!(file, "{} {} {}\n", r, g, b).unwrap();
    }

    println!("Saved paper_cutout_demo.ppm");
}
