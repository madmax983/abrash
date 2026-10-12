//! Deterministic profiling harness for `examples/tunnel_demo.rs`'s per-frame
//! work: `framebuffer.clear` + `apply_tunnel` on the demo's 800x600
//! framebuffer with the demo's 256x256 checkerboard texture, for N frames
//! (default 60) at a fixed 1/60 s timestep.
//!
//! Not a criterion benchmark: wall-clock is unreliable on this machine. Run
//! under `valgrind --tool=callgrind`. The windowing/present step is skipped;
//! everything else mirrors the demo.
use abrash::experimental::tunnel::apply_tunnel;
use abrash::framebuffer::Framebuffer;
use abrash::texture::Texture;
use std::hint::black_box;

const WIDTH: u32 = 800;
const HEIGHT: u32 = 600;

fn main() {
    let frames: usize = std::env::args()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(60);

    let mut texture = Texture::new(256, 256).unwrap();
    for y in 0..256 {
        for x in 0..256 {
            let color = if ((x / 32) + (y / 32)) % 2 == 0 {
                0xFF_FF_FF_FF
            } else {
                0xFF_00_00_FF
            };
            texture.set_pixel(x, y, color);
        }
    }
    let mut fb = Framebuffer::new(WIDTH, HEIGHT).unwrap();
    let mut time = 0.0f32;
    let mut checksum: u64 = 0;

    for _ in 0..frames {
        time += 1.0 / 60.0;
        fb.clear(0xFF_00_00_00);
        apply_tunnel(&mut fb, time, &texture);
        checksum = fb.as_slice().iter().fold(checksum, |acc, &p| {
            acc.wrapping_mul(31).wrapping_add(u64::from(p))
        });
    }
    black_box(checksum);
    println!("checksum {checksum}");
}
