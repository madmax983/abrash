use abrash_core::framebuffer::Framebuffer;
use abrash_core::zbuffer::ZBuffer;
use abrash_render::experimental::normal_map::{NormalMapConfig, apply_normal_map};
use std::path::Path;

fn main() {
    let width = 800;
    let height = 600;
    let mut fb = Framebuffer::new(width, height).unwrap();
    let mut zb = ZBuffer::new(width, height).unwrap();

    // Fill ZBuffer with a simple slope
    for y in 0..height {
        for x in 0..width {
            let z = (x as f32) / (width as f32) * 10.0;
            zb.test_and_set(x as i32, y as i32, z);
        }
    }

    let config = NormalMapConfig {
        depth_scale: 1.0,
        rgb_encode: true,
    };
    apply_normal_map(&mut fb, &zb, &config);

    // Save output
    fb.export_ppm(Path::new("normal_map_demo.ppm")).unwrap();
}
