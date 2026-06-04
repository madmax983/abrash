use abrash_core::framebuffer::Framebuffer;
use abrash_render::experimental::lava_lamp::{LavaLampConfig, apply_lava_lamp};

fn main() {
    let width = 800;
    let height = 600;
    let mut fb = Framebuffer::new(width, height).unwrap();

    let mut config = LavaLampConfig::default();

    let frames = 60;
    println!("Generating {frames} frames of lava lamp demo...");
    for i in 0..frames {
        config.time = (i as f32) * 0.1;
        apply_lava_lamp(&mut fb, &config);
    }

    // Just a basic demonstration of it running, we could write out a PPM here
    // like the other examples do.
    fb.export_ppm("lava_lamp_demo.ppm").unwrap();
    println!("Saved lava_lamp_demo.ppm");
}
