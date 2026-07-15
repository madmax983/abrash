use abrash::framebuffer::Framebuffer;
use abrash_render::experimental::attractor::{render_attractor, CliffordAttractor};

fn main() {
    let mut fb = Framebuffer::new(800, 600).unwrap();
    fb.clear(0xFF000000);
    let attractor = CliffordAttractor::new(-1.4, 1.6, 1.0, 0.7);
    let mut pts = Vec::new();
    attractor.run_steps(0.0, 0.0, 100_000, &mut pts);
    render_attractor(&mut fb, &pts, 150.0, 400.0, 300.0, 0x0001_0101); // Render very faintly
    println!("Done");
}
