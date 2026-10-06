//! Deterministic profiling harness for `examples/fire_demo.rs`'s per-frame
//! work: cooling-map noise fill + bottom-row feed (`update`), `apply_fire`,
//! and the heat-to-fire palette pass (`render`), on the demo's 640x480
//! framebuffer with the demo's seed, for N frames (default 60).
//!
//! Not a criterion benchmark: wall-clock is unreliable on this machine. Run
//! under `valgrind --tool=callgrind` / `--tool=dhat`. The windowing/present
//! step is skipped; everything else mirrors the demo exactly.
use abrash::framebuffer::Framebuffer;
use abrash_core::utils::XorShift32;
use abrash_render::experimental::fire::apply_fire;
use std::hint::black_box;

const WIDTH: u32 = 640;
const HEIGHT: u32 = 480;

fn main() {
    let frames: usize = std::env::args()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(60);

    let width = WIDTH as usize;
    let height = HEIGHT as usize;
    let mut fb = Framebuffer::new(WIDTH, HEIGHT).unwrap();
    let mut cooling_map = vec![0u8; width * height];
    let mut rng = XorShift32::new(1337);
    let mut checksum: u64 = 0;

    for _ in 0..frames {
        // update()
        for y in 0..height {
            for x in 0..width {
                let r = rng.next_u32() % 2;
                cooling_map[y * width + x] = r as u8;
            }
        }
        let fb_slice = fb.as_mut_slice();
        for x in 0..width {
            let r = rng.next_u32() % 256;
            let val = if r < 128 { 0xFF } else { 0x00 };
            fb_slice[(height - 1) * width + x] = (val << 16) | (val << 8) | val;
        }

        // render()
        apply_fire(&mut fb, &cooling_map);
        for p in fb.as_mut_slice().iter_mut() {
            let heat = (*p >> 16) & 0xFF;
            let (r, g, b) = if heat > 160 {
                (255, 255, heat)
            } else if heat > 80 {
                (255, heat * 2, 0)
            } else {
                (heat * 3, 0, 0)
            };
            *p = (r << 16) | (g << 8) | b;
        }
        checksum = fb.as_slice().iter().fold(checksum, |acc, &p| {
            acc.wrapping_mul(31).wrapping_add(u64::from(p))
        });
    }
    black_box(checksum);
    println!("checksum {checksum}");
}
