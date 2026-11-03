//! Deterministic profiling harness for `examples/mode7_demo.rs`'s
//! `render_mode7` call: 800x600 framebuffer, the demo's 256x256 checkerboard
//! texture and camera config (cy 30, fov 300, horizon 200, sky-blue fog), with
//! the demo's auto-pilot camera advancing and turning each frame, 30 frames.
//! Not a criterion benchmark: run under `valgrind --tool=callgrind`.
use abrash_render::experimental::mode7::{Mode7Config, render_mode7};
use abrash::framebuffer::Framebuffer;
use abrash::texture::Texture;
use std::hint::black_box;

const WIDTH: u32 = 800;
const HEIGHT: u32 = 600;
const TEX: usize = 256;

fn main() {
    let frames: usize = std::env::args()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(30);

    let mut texture = Texture::new(TEX as u32, TEX as u32).unwrap();
    for y in 0..TEX {
        for x in 0..TEX {
            let check = ((x / 32) + (y / 32)) % 2 == 0;
            let color = if check { 0xFF44_AA44 } else { 0xFF22_6622 };
            let border = (x % 32 == 0) || (y % 32 == 0);
            texture.set_pixel(x as u32, y as u32, if border { 0xFF88_FF88 } else { color });
        }
    }

    let mut config = Mode7Config {
        cy: 30.0,
        fov: 300.0,
        horizon: 200.0,
        fog_start: 300.0,
        fog_end: 1500.0,
        fog_color: 0xFF88_CCFF,
        ..Default::default()
    };

    let mut fb = Framebuffer::new(WIDTH, HEIGHT).unwrap();
    let mut checksum: u64 = 0;
    for _ in 0..frames {
        config.cx += config.angle.sin() * 50.0 / 60.0;
        config.cz -= config.angle.cos() * 50.0 / 60.0;
        config.angle += 0.02;
        render_mode7(&mut fb, &texture, &config);
        checksum = fb.as_slice().iter().fold(checksum, |acc, &p| {
            acc.wrapping_mul(31).wrapping_add(u64::from(p))
        });
    }
    black_box(checksum);
    println!("checksum {checksum}");
}
