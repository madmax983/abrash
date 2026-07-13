use abrash::experimental::strange_attractor::render_clifford_attractor;
use abrash::framebuffer::Framebuffer;

fn main() {
    let mut fb = Framebuffer::new(800, 600).unwrap();
    println!("Rendering strange attractor...");
    render_clifford_attractor(&mut fb, -1.4, 1.6, 1.0, 0.7, 5_000_000);
    fb.export_ppm("strange_attractor.ppm").unwrap();
    println!("Saved to strange_attractor.ppm");
}
