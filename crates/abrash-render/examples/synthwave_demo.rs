use abrash_core::framebuffer::Framebuffer;
use abrash_core::zbuffer::ZBuffer;
use abrash_render::experimental::synthwave::{SynthwaveConfig, apply_synthwave};

fn main() {
    let width = 800;
    let height = 600;
    let mut fb = Framebuffer::new(width, height).unwrap();
    let zb = ZBuffer::new(width, height).unwrap();

    let config = SynthwaveConfig {
        time: 1.0,
        ..Default::default()
    };

    apply_synthwave(&mut fb, Some(&zb), &config);
    fb.export_ppm("synthwave_demo.ppm").unwrap();
    println!("Exported synthwave_demo.ppm");
}
