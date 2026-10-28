//! Software occluder Hi-Z demo: a wall and a slanted ramp are rasterized
//! as cheap occluders into a tiled `OccluderHiZ`, then a row of test
//! crates is culled against the pyramid — no shading pass required.
//!
//! Run with: `cargo run -p abrash-core --example occluder_hiz_demo`
//! Writes `occluder_hiz_demo.ppm` (occluders dark, culled crates red,
//! visible crates green).

use abrash_core::framebuffer::Framebuffer;
use abrash_core::hiz_buffer::AABB3D;
use abrash_core::occluder_hiz::{OccluderHiZ, ScreenVertex};

fn fill_rect(fb: &mut Framebuffer, x0: i32, y0: i32, x1: i32, y1: i32, color: u32) {
    for y in y0..=y1 {
        for x in x0..=x1 {
            fb.set_pixel(x, y, color);
        }
    }
}

fn main() {
    let (w, h) = (640, 360);
    let mut occluders = OccluderHiZ::new(w, h);

    // Occluder 1: a solid wall over the left half of the screen, depth 5.
    occluders.rasterize_rect_occluder(0, 0, 319, 359, 5.0);
    // Occluder 2: a slanted ramp on the right, depth 3 (near) to 8 (far).
    occluders.rasterize_triangle_occluder(
        ScreenVertex::new(384.0, 360.0, 3.0),
        ScreenVertex::new(640.0, 360.0, 3.0),
        ScreenVertex::new(640.0, 96.0, 8.0),
    );
    occluders.build_pyramid();

    let (gw, gh) = occluders.grid_dimensions();
    println!(
        "occluder_hiz_demo: {w}x{h}, tile {}px, grid {gw}x{gh}, {} levels, {}/{} cells covered",
        occluders.tile_size(),
        occluders.level_count(),
        occluders.covered_cell_count(),
        gw * gh,
    );

    // Test crates: (label, bounds, depth).
    let crates: [(&str, AABB3D); 5] = [
        ("behind-wall", AABB3D::new(64, 128, 64, 128, 10.0, 12.0)),
        ("front-of-wall", AABB3D::new(192, 256, 64, 128, 2.0, 4.0)),
        (
            "straddling-wall-edge",
            AABB3D::new(288, 352, 192, 256, 10.0, 12.0),
        ),
        ("behind-ramp", AABB3D::new(576, 608, 288, 320, 12.0, 14.0)),
        ("open-ground", AABB3D::new(384, 448, 32, 96, 10.0, 12.0)),
    ];

    let mut fb = Framebuffer::new(w, h).expect("framebuffer dimensions are valid");
    fb.clear(0xFF10_1018);
    fill_rect(&mut fb, 0, 0, 319, 359, 0xFF3A_3A44); // Wall.
    for (label, aabb) in &crates {
        let visible = occluders.is_potentially_visible(*aabb);
        println!(
            "  {label:<22} depth {:>4.0}..{:<4.0} -> {}",
            aabb.min_depth,
            aabb.max_depth,
            if visible { "DRAW" } else { "CULLED" }
        );
        let color = if visible { 0xFF2E_CC71 } else { 0xFFE7_4C3C };
        fill_rect(
            &mut fb, aabb.min_x, aabb.min_y, aabb.max_x, aabb.max_y, color,
        );
    }

    let culled = crates
        .iter()
        .filter(|(_, a)| occluders.is_occluded(*a))
        .count();
    println!("occluder_hiz_demo: {culled}/{} crates culled", crates.len());
    fb.export_ppm("occluder_hiz_demo.ppm").expect("write PPM");
    println!("wrote occluder_hiz_demo.ppm");
}
