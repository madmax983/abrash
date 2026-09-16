//! Deterministic profiling harness for the raycaster column-render pipeline
//! (`render_raycast_view`: per-column DDA cast + wall-strip fill + z-buffer write).
//!
//! Not a criterion benchmark: wall-clock is unreliable on this machine. This binary
//! is meant to be run under `valgrind --tool=callgrind` / `--tool=dhat` to get
//! deterministic instruction and allocation counts for a realistic workload —
//! 800x600 (matching `examples/raycast_demo.rs`), a 16x16 map with interior walls
//! and pillars, camera orbiting so every frame casts a different set of rays.
use abrash::bam::{ANG90, Bam};
use abrash::framebuffer::Framebuffer;
use abrash::raycast::map::ArrayGridMap;
use abrash::raycast::types::{Cell, Vec2Fixed};
use abrash::raycaster::hybrid::render_raycast_view;
use abrash::zbuffer::ZBuffer;
use std::hint::black_box;

/// Same 16x16 demo map as `examples/raycast_demo.rs`: border walls, two interior
/// partitions, three pillars.
fn make_demo_map() -> ArrayGridMap {
    let mut map = ArrayGridMap::new(16, 16);

    for x in 0..16 {
        map.set(x, 0, Cell::Solid(3));
        map.set(x, 15, Cell::Solid(3));
    }
    for y in 0..16 {
        map.set(0, y, Cell::Solid(3));
        map.set(15, y, Cell::Solid(3));
    }

    for x in 4..8 {
        map.set(x, 4, Cell::Solid(0));
    }
    for y in 4..8 {
        map.set(8, y, Cell::Solid(1));
    }

    map.set(10, 10, Cell::Solid(2));
    map.set(12, 6, Cell::Solid(2));
    map.set(3, 10, Cell::Solid(0));

    map
}

fn main() {
    let width = 800;
    let height = 600;

    let frames: usize = std::env::args()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(30);

    let map = make_demo_map();
    let mut fb = Framebuffer::new(width, height).unwrap();
    let mut zb = ZBuffer::new(width, height).unwrap();

    let camera_pos = Vec2Fixed::from_f32(8.5, 2.5);
    let fov = ANG90;

    // Orbit the camera angle a little each frame so every frame casts a
    // genuinely different set of rays (avoids a compiler/const-folding
    // shortcut collapsing all frames into one).
    let turn_step = Bam::from_raw(0x0100_0000);
    let mut camera_angle = Bam::from_raw(0);

    for _ in 0..frames {
        fb.clear(0xFF1A_1A2E);
        zb.clear();
        render_raycast_view(&mut fb, &mut zb, &map, camera_pos, camera_angle, fov);
        camera_angle = camera_angle + turn_step;
        black_box(&fb);
    }

    let checksum: u64 = fb.as_slice().iter().map(|&p| p as u64).sum();
    println!("frames={frames} checksum={checksum}");
}
