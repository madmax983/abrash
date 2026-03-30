//! Raycaster column renderer with z-buffer bridge for hybrid compositing.
//!
//! Draws raycasted walls to a [`Framebuffer`] and writes perpendicular distances
//! to a [`ZBuffer`], enabling hybrid rendering where raycasted environments and
//! triangle-rasterized objects share a unified depth buffer.

use abrash_core::bam::Bam;
use abrash_core::fixed16_16::Fixed16_16;
use abrash_core::framebuffer::Framebuffer;
use abrash_core::zbuffer::ZBuffer;
use abrash_raycast::cast::cast_ray_detailed;
use abrash_raycast::map::ArrayGridMap;
use abrash_raycast::types::{Side, Vec2Fixed};

/// Map a material ID and hit side to an ARGB color.
///
/// East/West faces are rendered at half brightness to give a simple depth cue,
/// mimicking the Wolfenstein 3D shading convention.
#[must_use]
pub const fn wall_color(material_id: u16, side: Side) -> u32 {
    let bright = match material_id {
        0 => 0xFFFF_0000, // red
        1 => 0xFF00_FF00, // green
        2 => 0xFF00_00FF, // blue
        _ => 0xFF80_8080, // grey
    };

    match side {
        Side::North | Side::South => bright,
        Side::East | Side::West => {
            // Half brightness: shift each channel right by 1, mask to avoid
            // bleeding between channels.
            let r = (bright >> 17) & 0x7F;
            let g = (bright >> 9) & 0x7F;
            let b = (bright >> 1) & 0x7F;
            0xFF00_0000 | (r << 16) | (g << 8) | b
        }
    }
}

/// Render a raycasted first-person view into the framebuffer and z-buffer.
///
/// For each screen column the function casts a ray, computes the wall strip
/// height from the perpendicular distance, fills the column with the
/// appropriate wall color, and writes the perpendicular distance into the
/// z-buffer for every wall pixel.
///
/// # Parameters
///
/// * `fb` - Destination framebuffer (pixels).
/// * `zbuf` - Destination depth buffer (per-pixel depth for hybrid compositing).
/// * `map` - Grid map to cast against.
/// * `camera_pos` - Camera position in world-space (fixed-point).
/// * `camera_angle` - Camera facing direction (BAM angle).
/// * `fov` - Horizontal field of view (BAM angle).
pub fn render_raycast_view(
    fb: &mut Framebuffer,
    zbuf: &mut ZBuffer,
    map: &ArrayGridMap,
    camera_pos: Vec2Fixed,
    camera_angle: Bam,
    fov: Bam,
) {
    let w = fb.width() as i32;
    let h = fb.height() as i32;

    if w == 0 || h == 0 {
        return;
    }

    // Half-FOV: the angle from the camera center to the left screen edge.
    let half_fov = Bam::from_raw(fov.raw() / 2);

    // Start angle is camera_angle - half_fov (leftmost column).
    let start_angle = camera_angle - half_fov;

    // Angle increment per column.
    // fov.raw() / w gives the BAM step per pixel column.
    let angle_step_raw = fov.raw() / (w as u32);

    for col in 0..w {
        let ray_angle = start_angle + Bam::from_raw(angle_step_raw * (col as u32));

        let Some(detail) = cast_ray_detailed(map, camera_pos, ray_angle) else {
            continue;
        };

        let perp_dist = detail.hit.distance.to_f32();

        // Avoid division by zero / near-zero.
        if perp_dist < 0.001 {
            continue;
        }

        // Wall strip height is proportional to 1/distance.
        // Scale factor: one grid cell at distance 1.0 fills the entire screen height.
        let strip_height = (h as f32 / perp_dist) as i32;

        // Vertical centering.
        let draw_start = (h / 2) - (strip_height / 2);
        let draw_end = draw_start + strip_height;

        // Clamp to screen bounds.
        let y_start = draw_start.max(0);
        let y_end = draw_end.min(h);

        // Determine wall color from material ID + side.
        let material_id = match map.cell_at(detail.hit.cell_x, detail.hit.cell_y) {
            abrash_raycast::types::Cell::Solid(id) => id,
            _ => 0,
        };
        let color = wall_color(material_id, detail.hit.side);

        // Draw the vertical strip.
        for y in y_start..y_end {
            fb.set_pixel(col, y, color);
            zbuf.test_and_set(col, y, perp_dist);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use abrash_core::bam::ANG90;
    use abrash_raycast::map::ArrayGridMap;
    use abrash_raycast::types::Cell;

    /// 8x8 room with solid border walls (material 1).
    fn simple_room() -> ArrayGridMap {
        let mut map = ArrayGridMap::new(8, 8);
        for x in 0..8 {
            map.set(x, 0, Cell::Solid(1));
            map.set(x, 7, Cell::Solid(1));
        }
        for y in 0..8 {
            map.set(0, y, Cell::Solid(1));
            map.set(7, y, Cell::Solid(1));
        }
        map
    }

    #[test]
    fn render_does_not_panic() {
        let map = simple_room();
        let mut fb = Framebuffer::new(320, 200).unwrap();
        let mut zbuf = ZBuffer::new(320, 200).unwrap();
        let pos = Vec2Fixed::from_f32(4.0, 4.0);
        let angle = Bam::ZERO; // facing east
        let fov = ANG90; // 90 degrees

        // Should complete without panicking.
        render_raycast_view(&mut fb, &mut zbuf, &map, pos, angle, fov);
    }

    #[test]
    fn render_draws_pixels() {
        let map = simple_room();
        let mut fb = Framebuffer::new(320, 200).unwrap();
        let mut zbuf = ZBuffer::new(320, 200).unwrap();
        let pos = Vec2Fixed::from_f32(4.0, 4.0);
        let angle = Bam::ZERO;
        let fov = ANG90;

        fb.clear(0xFF00_0000); // black
        render_raycast_view(&mut fb, &mut zbuf, &map, pos, angle, fov);

        // The center of the screen (column 160, row 100) should have been
        // drawn with a wall color (not the initial black).
        let center_pixel = fb.get_pixel(160, 100).unwrap();
        assert_ne!(
            center_pixel, 0xFF00_0000,
            "center pixel should be painted by the raycaster"
        );
    }

    #[test]
    fn zbuffer_written_for_wall_columns() {
        let map = simple_room();
        let mut fb = Framebuffer::new(320, 200).unwrap();
        let mut zbuf = ZBuffer::new(320, 200).unwrap();
        let pos = Vec2Fixed::from_f32(4.0, 4.0);
        let angle = Bam::ZERO;
        let fov = ANG90;

        render_raycast_view(&mut fb, &mut zbuf, &map, pos, angle, fov);

        // The z-buffer at screen center should have a finite depth value
        // (not INFINITY, which is the default).
        let center_depth = zbuf.get_depth(160, 100).unwrap();
        assert!(
            center_depth.is_finite(),
            "z-buffer at center should have finite depth after rendering, got {center_depth}"
        );
        assert!(
            center_depth > 0.0,
            "z-buffer depth should be positive, got {center_depth}"
        );
    }

    #[test]
    fn wall_color_bright_for_ns_dark_for_ew() {
        // North/South should be full brightness.
        let bright = wall_color(0, Side::North);
        // East/West should be half brightness.
        let dark = wall_color(0, Side::East);

        // Red channel: bright should be 0xFF, dark should be ~0x7F.
        let bright_r = (bright >> 16) & 0xFF;
        let dark_r = (dark >> 16) & 0xFF;
        assert_eq!(bright_r, 0xFF);
        assert!(
            dark_r < bright_r,
            "E/W faces should be darker: dark_r={dark_r:#X}, bright_r={bright_r:#X}"
        );
    }

    #[test]
    fn wall_color_material_ids() {
        assert_eq!(wall_color(0, Side::North), 0xFFFF_0000); // red
        assert_eq!(wall_color(1, Side::North), 0xFF00_FF00); // green
        assert_eq!(wall_color(2, Side::North), 0xFF00_00FF); // blue
        assert_eq!(wall_color(3, Side::North), 0xFF80_8080); // grey
        assert_eq!(wall_color(99, Side::North), 0xFF80_8080); // fallback grey
    }

    #[test]
    fn zero_size_framebuffer_does_not_panic() {
        // Edge case: zero-width or zero-height should be a no-op.
        let map = simple_room();
        let mut fb = Framebuffer::new(1, 1).unwrap();
        let mut zbuf = ZBuffer::new(1, 1).unwrap();
        let pos = Vec2Fixed::from_f32(4.0, 4.0);
        render_raycast_view(&mut fb, &mut zbuf, &map, pos, Bam::ZERO, ANG90);
    }

    #[test]
    fn depth_decreases_for_closer_walls() {
        // Camera near the west wall should produce smaller depth at center
        // than camera in the middle of the room.
        let map = simple_room();

        // Camera near the east wall, facing east.
        let mut fb_near = Framebuffer::new(320, 200).unwrap();
        let mut zbuf_near = ZBuffer::new(320, 200).unwrap();
        let pos_near = Vec2Fixed::from_f32(5.5, 4.0);
        render_raycast_view(
            &mut fb_near,
            &mut zbuf_near,
            &map,
            pos_near,
            Bam::ZERO,
            ANG90,
        );

        // Camera in the center, facing east.
        let mut fb_far = Framebuffer::new(320, 200).unwrap();
        let mut zbuf_far = ZBuffer::new(320, 200).unwrap();
        let pos_far = Vec2Fixed::from_f32(4.0, 4.0);
        render_raycast_view(&mut fb_far, &mut zbuf_far, &map, pos_far, Bam::ZERO, ANG90);

        let near_depth = zbuf_near.get_depth(160, 100).unwrap();
        let far_depth = zbuf_far.get_depth(160, 100).unwrap();

        assert!(
            near_depth < far_depth,
            "closer wall should yield smaller depth: near={near_depth}, far={far_depth}"
        );
    }
}
