//! Deterministic profiling harness for `examples/sonar_demo.rs`'s `apply_sonar`
//! call: 800x600 framebuffer + z-buffer (the demo's size) holding a ground plane
//! receding to the horizon plus a nearer sphere-ish bump, default-like config with
//! an animated phase, 30 frames. Not a criterion benchmark: run under
//! `valgrind --tool=callgrind`.
use abrash::framebuffer::Framebuffer;
use abrash::zbuffer::ZBuffer;
use abrash_render::experimental::sonar::{SonarConfig, apply_sonar};
use std::hint::black_box;

const WIDTH: u32 = 800;
const HEIGHT: u32 = 600;

fn main() {
    let frames: usize = std::env::args()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(30);

    let mut fb = Framebuffer::new(WIDTH, HEIGHT).unwrap();
    let mut zb = ZBuffer::new(WIDTH, HEIGHT).unwrap();
    // Depth: floor receding below the horizon (rows > H/2), infinite sky above,
    // and a disc of near geometry in the centre.
    for y in 0..HEIGHT {
        for x in 0..WIDTH {
            let d = if y > HEIGHT / 2 {
                80.0 / (y - HEIGHT / 2) as f32
            } else {
                f32::INFINITY
            };
            let dx = x as f32 - 400.0;
            let dy = y as f32 - 300.0;
            let d = if dx * dx + dy * dy < 80.0 * 80.0 {
                12.0 + (dx * dx + dy * dy).sqrt() * 0.05
            } else {
                d
            };
            zb.test_and_set(x as i32, y as i32, d);
        }
    }

    let mut checksum: u64 = 0;
    for frame in 0..frames {
        fb.clear(0xFF30_6090);
        let config = SonarConfig {
            time: frame as f32 / 60.0,
            ..SonarConfig::default()
        };
        apply_sonar(&mut fb, &zb, &config);
        checksum = fb.as_slice().iter().fold(checksum, |acc, &p| {
            acc.wrapping_mul(31).wrapping_add(u64::from(p))
        });
    }
    black_box(checksum);
    println!("checksum {checksum}");
}
