//! BSP software renderer: wall columns, floor/ceiling spans, and the main
//! `render_bsp_view` entry point.
//!
//! # Architecture
//!
//! The renderer processes a BSP map in front-to-back order. For each subsector
//! visited by the BSP traversal, it iterates the subsector's segs and for each
//! seg:
//!
//! 1. Clips the seg's angular span to the camera's FOV.
//! 2. Projects clipped angles to screen columns.
//! 3. For each screen column, computes wall top/bottom, draws wall texture
//!    columns via [`draw_wall_column`], records floor/ceiling spans in
//!    [`VisplaneAllocator`], and updates [`ColumnClip`] state.
//!
//! After all walls are processed, [`draw_visplane_spans`] renders the
//! accumulated floor/ceiling horizontal spans with perspective-correct flat
//! texture mapping.

use abrash_core::bam::{ANG90, Bam};
use abrash_core::fixed16_16::Fixed16_16;
use abrash_core::framebuffer::Framebuffer;
use abrash_core::zbuffer::ZBuffer;

use crate::bsp::{BspMap, BspSector, BspSeg};
use crate::bsp_clip::{
    ColumnClip, angle_to_column, clip_seg_angles, point_to_angle, projection_distance,
    seg_perpendicular_distance, wall_scale_at_column,
};
use crate::bsp_visplane::VisplaneAllocator;
use crate::types::Vec2Fixed;

use super::bsp_lighting::{BspTextures, colormap_index};

#[inline]
/// Draw a single textured wall column from `y_top` to `y_bot` (inclusive).
///
/// # Parameters
///
/// * `fb`            -- destination framebuffer.
/// * `zbuf`          -- destination z-buffer.
/// * `textures`      -- texture/colormap/palette provider.
/// * `col`           -- screen column (x coordinate).
/// * `y_top`         -- topmost screen row of the column (inclusive).
/// * `y_bot`         -- bottommost screen row of the column (inclusive).
/// * `texture_id`    -- wall texture identifier.
/// * `texture_col`   -- column within the texture (horizontal offset).
/// * `colormap_idx`  -- pre-computed colormap row (0 = bright, 31 = dark).
/// * `tex_y_frac`    -- starting texture Y in 16.16 fixed-point.
/// * `tex_y_step`    -- texture Y increment per screen row in 16.16.
/// * `distance`      -- wall distance for z-buffer writes.
pub fn draw_wall_column(
    fb: &mut Framebuffer,
    zbuf: &mut ZBuffer,
    textures: &impl BspTextures,
    col: i32,
    y_top: i32,
    y_bot: i32,
    texture_id: u16,
    texture_col: usize,
    colormap_idx: u8,
    tex_y_frac: Fixed16_16,
    tex_y_step: Fixed16_16,
    distance: f32,
) {
    // Early-out: empty or inverted range, or column off-screen
    if y_top > y_bot || col < 0 || col >= fb.width() as i32 {
        return;
    }

    let tex_data = textures.wall_column(texture_id, texture_col);
    let colormap = textures.colormap(colormap_idx);
    let tex_height = tex_data.len() as i32;

    if tex_height == 0 {
        return;
    }

    let mut cur_frac = tex_y_frac;

    for row in y_top..=y_bot {
        // Compute wrapped texture Y
        let mut tex_y = cur_frac.to_int() % tex_height;
        if tex_y < 0 {
            tex_y += tex_height;
        }

        let palette_idx = tex_data[tex_y as usize];
        let shaded_idx = colormap[palette_idx as usize];
        let argb = textures.palette_argb(shaded_idx);

        fb.set_pixel(col, row, argb);
        zbuf.test_and_set(col, row, distance);

        cur_frac += tex_y_step;
    }
}

// ---------------------------------------------------------------------------
// Floor/Ceiling span renderer
// ---------------------------------------------------------------------------

/// Draw all visplane spans (floors and ceilings).
///
/// For each visplane, iterates columns in `[min_x, max_x]`. For each column
/// with data, draws horizontal pixels using perspective-correct flat texture
/// mapping. Each pixel's world coordinate is computed from the camera position,
/// angle, projection distance, and the visplane's height relative to the
/// camera.
pub fn draw_visplane_spans(
    fb: &mut Framebuffer,
    zbuf: &mut ZBuffer,
    textures: &impl BspTextures,
    visplanes: &VisplaneAllocator,
    camera_pos: Vec2Fixed,
    camera_angle: Bam,
    camera_z: Fixed16_16,
    projection: Fixed16_16,
) {
    let screen_w = fb.width() as f32;
    let screen_h = fb.height() as f32;
    let half_h = screen_h / 2.0;
    let proj_f = projection.to_f32();
    let cam_x = camera_pos.x.to_f32();
    let cam_y = camera_pos.y.to_f32();
    let cam_z_f = camera_z.to_f32();
    let angle_rad = camera_angle.to_radians();
    let cos_a = angle_rad.cos();
    let sin_a = angle_rad.sin();

    for plane in visplanes.planes() {
        let flat_data = textures.flat_data(plane.texture);

        for col in plane.min_x..=plane.max_x {
            let Some(top) = plane.top(col) else {
                continue;
            };
            let Some(bot) = plane.bottom(col) else {
                continue;
            };

            let height_diff = (cam_z_f - f32::from(plane.height)).abs();

            for row in top..=bot {
                let row_offset = (row as f32 - half_h).abs();
                if row_offset < 0.5 {
                    continue; // avoid division by zero at horizon
                }

                let step_distance = height_diff * proj_f / row_offset;

                // Compute world-space coordinates via rotation
                let dx = (col as f32 - screen_w / 2.0) * step_distance / proj_f;
                let dy = step_distance;
                let world_x = cam_x + dx * cos_a - dy * sin_a;
                let world_y = cam_y + dx * sin_a + dy * cos_a;

                // Sample flat texture (64x64, tiling)
                let tx = (world_x as i32) & 63;
                let ty = (world_y as i32) & 63;
                let tex_idx = (ty * 64 + tx) as usize;
                // Clamp to valid range for safety
                let tex_idx = tex_idx.min(4095);
                let palette_idx = flat_data[tex_idx];

                // Apply distance-based colormap shading
                let cmap = colormap_index(Fixed16_16::from_f32(step_distance), plane.light_level);
                let colormap = textures.colormap(cmap);
                let shaded_idx = colormap[palette_idx as usize];
                let argb = textures.palette_argb(shaded_idx);

                fb.set_pixel(col, row, argb);
                zbuf.test_and_set(col, row, step_distance);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Texture column helper
// ---------------------------------------------------------------------------

/// Compute the horizontal texture column for a wall at a given screen column.
///
/// Uses the seg's texture offset plus a linear interpolation along the seg's
/// angular span to approximate the texture U coordinate.
fn compute_texture_col(
    seg: &BspSeg,
    screen_col: i32,
    camera_pos: Vec2Fixed,
    camera_angle: Bam,
    projection: Fixed16_16,
) -> usize {
    let half_w = projection.to_f32(); // approximate: projection ~ half_width for 90deg FOV
    let proj_f = projection.to_f32();

    // Column angle relative to center
    let col_offset = (screen_col as f32) - half_w;
    let col_angle = Bam::from_radians((col_offset / proj_f).atan());

    // Global ray angle
    let ray_angle = camera_angle + col_angle;

    // Compute intersection with the seg's line.
    // The seg runs from v1 to v2. We find the parameter t along the seg
    // where the ray from camera_pos at ray_angle intersects.
    let (ray_sin, ray_cos) = ray_angle.to_radians().sin_cos();

    let dx = seg.v2.x.to_f32() - seg.v1.x.to_f32();
    let dy = seg.v2.y.to_f32() - seg.v1.y.to_f32();
    #[allow(clippy::imprecise_flops)]
    let seg_len = dx.mul_add(dx, dy * dy).sqrt();

    // Vector from camera to seg v1
    let cx = seg.v1.x.to_f32() - camera_pos.x.to_f32();
    let cy = seg.v1.y.to_f32() - camera_pos.y.to_f32();

    // Solve for t: camera + t_ray * ray_dir = v1 + t_seg * seg_dir
    // Using cross products to find t_seg:
    // t_seg = (cx * ray_sin - cy * ray_cos) / (dx * ray_sin - dy * ray_cos)
    let denom = dx * ray_sin - dy * ray_cos;
    let t_seg = if denom.abs() > 1e-6 {
        (cx * ray_sin - cy * ray_cos) / denom
    } else {
        0.0
    };

    let along_seg = t_seg * seg_len;
    let tex_u = seg.offset.to_f32() + along_seg;

    // Wrap to positive and convert to integer column
    let tex_col = tex_u as i32;
    let tex_col = ((tex_col % 128) + 128) % 128; // wrap to [0, 127]
    tex_col as usize
}

// ---------------------------------------------------------------------------
// Main BSP rendering entry point
// ---------------------------------------------------------------------------

/// Render a complete BSP view: walls, floors, and ceilings.
///
/// This is the top-level function that drives the Doom-style BSP software
/// renderer. It:
///
/// 1. Traverses the BSP tree front-to-back from the camera position.
/// 2. For each subsector, clips and projects each seg to screen columns.
/// 3. Draws wall texture columns, recording floor/ceiling spans.
/// 4. After all walls are processed, renders floor/ceiling flat spans.
///
/// # Parameters
///
/// * `fb`           -- destination framebuffer.
/// * `zbuf`         -- destination z-buffer (must be same dimensions as fb).
/// * `map`          -- BSP map data (tree, segs, sectors).
/// * `textures`     -- texture/colormap/palette provider.
/// * `camera_pos`   -- camera position in world space (16.16 fixed-point).
/// * `camera_angle` -- camera facing direction in BAM units.
/// * `camera_z`     -- camera height (eye level) in 16.16 fixed-point.
/// * `fov`          -- horizontal field of view in BAM units.
pub fn render_bsp_view(
    fb: &mut Framebuffer,
    zbuf: &mut ZBuffer,
    map: &impl BspMap,
    textures: &impl BspTextures,
    camera_pos: Vec2Fixed,
    camera_angle: Bam,
    camera_z: Fixed16_16,
    fov: Bam,
) {
    let w = fb.width();
    let h = fb.height();
    if w == 0 || h == 0 {
        return;
    }

    let half_fov = Bam::from_raw(fov.raw() / 2);
    let projection = projection_distance(w, fov);
    let mut clip = ColumnClip::new(w, h);
    let mut visplanes = VisplaneAllocator::new(w);
    let half_h = h as i32 / 2;

    map.traverse_front_to_back(camera_pos, &mut |ssector_idx| {
        if clip.all_filled() {
            return;
        }

        for seg in map.subsector_segs(ssector_idx) {
            // 1. Compute angles to seg endpoints
            let angle1 = point_to_angle(camera_pos, seg.v1);
            let angle2 = point_to_angle(camera_pos, seg.v2);

            // 2. Clip to FOV -- returns view-relative angles or None
            let Some((a1, a2)) = clip_seg_angles(angle1, angle2, camera_angle, half_fov) else {
                continue;
            };

            // 3. Map clipped angles to screen columns
            let x1 = angle_to_column(a1, projection, w);
            let x2 = angle_to_column(a2, projection, w);
            if x1 > x2 {
                continue; // degenerate after clipping
            }

            // 4. Per-seg constants
            let front = map.seg_front_sector(seg);
            let back = map.seg_back_sector(seg);
            let seg_normal = seg.angle.wrapping_add(ANG90);
            let rw_distance = seg_perpendicular_distance(seg.v1, seg.angle, camera_pos);
            let cmap = colormap_index(rw_distance, front.light_level);

            // 5. Process each column in the seg's screen range
            for x in x1.max(0)..=x2.min(w as i32 - 1) {
                if !clip.is_open(x) {
                    continue;
                }

                let scale =
                    wall_scale_at_column(x, w, projection, camera_angle, seg_normal, rw_distance);
                let scale_f = scale.to_f32();
                let cam_z_i = camera_z.to_int();

                // Screen rows for ceiling and floor
                let ceil_row =
                    half_h - ((i32::from(front.ceil_height) - cam_z_i) as f32 * scale_f) as i32;
                let floor_row =
                    half_h - ((i32::from(front.floor_height) - cam_z_i) as f32 * scale_f) as i32;

                let open_t = clip.top(x);
                let open_b = clip.bot(x);
                let distance_f = rw_distance.to_f32().abs().max(0.001);

                if let Some(back_s) = back {
                    // --- TWO-SIDED (portal) ---
                    let back_ceil_row = half_h
                        - ((i32::from(back_s.ceil_height) - cam_z_i) as f32 * scale_f) as i32;
                    let back_floor_row = half_h
                        - ((i32::from(back_s.floor_height) - cam_z_i) as f32 * scale_f) as i32;

                    // Ceiling visplane
                    let cy_top = open_t;
                    let cy_bot = (ceil_row - 1).min(open_b);
                    if cy_top <= cy_bot {
                        let vp = visplanes.find_or_create(
                            front.ceil_height,
                            front.ceil_texture,
                            front.light_level,
                            x,
                        );
                        visplanes.set_span(vp, x, cy_top, cy_bot);
                    }

                    // Upper wall (back ceil < front ceil)
                    if seg.upper_texture != 0 && back_s.ceil_height < front.ceil_height {
                        let wy_top = ceil_row.max(open_t);
                        let wy_bot = (back_ceil_row - 1).min(open_b);
                        if wy_top <= wy_bot {
                            let wall_height = f32::from(front.ceil_height - back_s.ceil_height);
                            let tex_step = if (floor_row - ceil_row) > 0 {
                                Fixed16_16::from_f32(
                                    wall_height / (floor_row - ceil_row).max(1) as f32,
                                )
                            } else {
                                Fixed16_16::ZERO
                            };
                            let tex_start = if ceil_row < open_t {
                                Fixed16_16::from_f32(
                                    wall_height * (open_t - ceil_row) as f32
                                        / (floor_row - ceil_row).max(1) as f32,
                                )
                            } else {
                                Fixed16_16::ZERO
                            };
                            let tex_col =
                                compute_texture_col(seg, x, camera_pos, camera_angle, projection);
                            draw_wall_column(
                                fb,
                                zbuf,
                                textures,
                                x,
                                wy_top,
                                wy_bot,
                                seg.upper_texture,
                                tex_col,
                                cmap,
                                tex_start,
                                tex_step,
                                distance_f,
                            );
                        }
                    }

                    // Lower wall (back floor > front floor)
                    if seg.lower_texture != 0 && back_s.floor_height > front.floor_height {
                        let wy_top = back_floor_row.max(open_t);
                        let wy_bot = floor_row.min(open_b);
                        if wy_top <= wy_bot {
                            let wall_height = f32::from(back_s.floor_height - front.floor_height);
                            let tex_step = if (floor_row - ceil_row) > 0 {
                                Fixed16_16::from_f32(wall_height / (floor_row - ceil_row) as f32)
                            } else {
                                Fixed16_16::ZERO
                            };
                            let tex_col =
                                compute_texture_col(seg, x, camera_pos, camera_angle, projection);
                            draw_wall_column(
                                fb,
                                zbuf,
                                textures,
                                x,
                                wy_top,
                                wy_bot,
                                seg.lower_texture,
                                tex_col,
                                cmap,
                                Fixed16_16::ZERO,
                                tex_step,
                                distance_f,
                            );
                        }
                    }

                    // Floor visplane
                    let fy_top = (floor_row + 1).max(open_t);
                    let fy_bot = open_b;
                    if fy_top <= fy_bot {
                        let vp = visplanes.find_or_create(
                            front.floor_height,
                            front.floor_texture,
                            front.light_level,
                            x,
                        );
                        visplanes.set_span(vp, x, fy_top, fy_bot);
                    }

                    // Update clips for portal opening
                    let new_top = back_ceil_row.max(ceil_row).max(open_t);
                    let new_bot = back_floor_row.min(floor_row).min(open_b);
                    if new_top > open_t {
                        clip.set_top(x, new_top);
                    }
                    if new_bot < open_b {
                        clip.set_bot(x, new_bot);
                    }
                    if new_top > new_bot {
                        clip.mark_solid(x);
                    }
                } else {
                    // --- ONE-SIDED (solid wall) ---

                    // Ceiling visplane
                    let cy_top = open_t;
                    let cy_bot = (ceil_row - 1).min(open_b);
                    if cy_top <= cy_bot {
                        let vp = visplanes.find_or_create(
                            front.ceil_height,
                            front.ceil_texture,
                            front.light_level,
                            x,
                        );
                        visplanes.set_span(vp, x, cy_top, cy_bot);
                    }

                    // Middle wall
                    let wy_top = ceil_row.max(open_t);
                    let wy_bot = floor_row.min(open_b);
                    if wy_top <= wy_bot && seg.middle_texture != 0 {
                        let wall_height = f32::from(front.ceil_height - front.floor_height);
                        let screen_wall = (floor_row - ceil_row).max(1) as f32;
                        let tex_step = Fixed16_16::from_f32(wall_height / screen_wall);
                        let tex_start = if ceil_row < open_t {
                            Fixed16_16::from_f32(
                                wall_height * (open_t - ceil_row) as f32 / screen_wall,
                            )
                        } else {
                            Fixed16_16::ZERO
                        };
                        let tex_col =
                            compute_texture_col(seg, x, camera_pos, camera_angle, projection);
                        draw_wall_column(
                            fb,
                            zbuf,
                            textures,
                            x,
                            wy_top,
                            wy_bot,
                            seg.middle_texture,
                            tex_col,
                            cmap,
                            tex_start,
                            tex_step,
                            distance_f,
                        );
                    }

                    // Floor visplane
                    let fy_top = (floor_row + 1).max(open_t);
                    let fy_bot = open_b;
                    if fy_top <= fy_bot {
                        let vp = visplanes.find_or_create(
                            front.floor_height,
                            front.floor_texture,
                            front.light_level,
                            x,
                        );
                        visplanes.set_span(vp, x, fy_top, fy_bot);
                    }

                    clip.mark_solid(x);
                }
            }
        }
    });

    // Draw floor/ceiling spans
    draw_visplane_spans(
        fb,
        zbuf,
        textures,
        &visplanes,
        camera_pos,
        camera_angle,
        camera_z,
        projection,
    );
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::super::bsp_lighting::BspTextures;
    use super::*;

    /// Mock texture provider for unit tests.
    struct MockTextures {
        wall_column_data: Vec<u8>,
        flat: [u8; 4096],
        colormap_data: [[u8; 256]; 32],
        palette: [u32; 256],
    }

    impl MockTextures {
        fn new() -> Self {
            // Wall column: 128 texels, all palette index 1
            let wall_column_data = vec![1u8; 128];

            // Flat: all palette index 2
            let flat = [2u8; 4096];

            // Colormaps: row 0 = identity, rows 1..31 = all map to 0 (dark)
            let mut colormap_data = [[0u8; 256]; 32];
            for i in 0..256 {
                colormap_data[0][i] = i as u8; // identity
            }
            // rows 1..31 are already zeroed (all map to palette 0 = black)

            // Palette
            let mut palette = [0xFF00_0000u32; 256]; // default black+alpha
            palette[0] = 0xFF00_0000; // black
            palette[1] = 0xFFFF_0000; // red
            palette[2] = 0xFF00_FF00; // green

            Self {
                wall_column_data,
                flat,
                colormap_data,
                palette,
            }
        }
    }

    impl BspTextures for MockTextures {
        fn wall_column(&self, _texture_id: u16, _col: usize) -> &[u8] {
            &self.wall_column_data
        }

        fn flat_data(&self, _texture_id: u16) -> &[u8; 4096] {
            &self.flat
        }

        fn colormap(&self, index: u8) -> &[u8; 256] {
            &self.colormap_data[index as usize]
        }

        fn palette_argb(&self, palette_idx: u8) -> u32 {
            self.palette[palette_idx as usize]
        }
    }

    #[test]
    fn draw_wall_column_fills_pixels() {
        let mut fb = Framebuffer::new(320, 200).unwrap();
        let mut zbuf = ZBuffer::new(320, 200).unwrap();
        let textures = MockTextures::new();

        // Draw a column at x=160, from y=80 to y=120
        draw_wall_column(
            &mut fb,
            &mut zbuf,
            &textures,
            160,                     // col
            80,                      // y_top
            120,                     // y_bot
            0,                       // texture_id
            0,                       // texture_col
            0,                       // colormap_idx (identity)
            Fixed16_16::ZERO,        // tex_y_frac
            Fixed16_16::from_int(1), // tex_y_step
            42.0,                    // distance
        );

        // All pixels in the column should now be red (palette[1] = 0xFFFF0000)
        // because wall_column_data is all palette index 1, colormap 0 is identity,
        // and palette[1] = red.
        for row in 80..=120 {
            let pixel = fb.get_pixel(160, row).unwrap();
            assert_ne!(
                pixel, 0xFF00_0000,
                "pixel at (160, {row}) should not be black"
            );
            assert_eq!(pixel, 0xFFFF_0000, "pixel at (160, {row}) should be red");
        }

        // Pixels outside the column should still be black (untouched)
        assert_eq!(fb.get_pixel(160, 79).unwrap(), 0xFF00_0000);
        assert_eq!(fb.get_pixel(160, 121).unwrap(), 0xFF00_0000);
    }

    #[test]
    fn draw_wall_column_writes_zbuffer() {
        let mut fb = Framebuffer::new(320, 200).unwrap();
        let mut zbuf = ZBuffer::new(320, 200).unwrap();
        let textures = MockTextures::new();

        draw_wall_column(
            &mut fb,
            &mut zbuf,
            &textures,
            100,
            50,
            60,
            0,
            0,
            0,
            Fixed16_16::ZERO,
            Fixed16_16::from_int(1),
            42.0,
        );

        // Every pixel in the drawn range should have depth ~42.0
        for row in 50..=60 {
            let depth = zbuf.get_depth(100, row).unwrap();
            assert!(
                (depth - 42.0).abs() < f32::EPSILON,
                "zbuf at (100, {row}) should be 42.0, got {depth}"
            );
        }

        // A pixel outside the column should still be infinity
        let untouched = zbuf.get_depth(100, 49).unwrap();
        assert!(untouched.is_infinite(), "untouched depth should be inf");
    }

    #[test]
    fn draw_wall_column_empty_range_no_op() {
        let mut fb = Framebuffer::new(320, 200).unwrap();
        let mut zbuf = ZBuffer::new(320, 200).unwrap();
        let textures = MockTextures::new();

        // Inverted range: y_top > y_bot
        draw_wall_column(
            &mut fb,
            &mut zbuf,
            &textures,
            160,
            120,
            80,
            0,
            0,
            0,
            Fixed16_16::ZERO,
            Fixed16_16::from_int(1),
            42.0,
        );

        // Nothing should have been drawn
        for row in 80..=120 {
            let pixel = fb.get_pixel(160, row).unwrap();
            assert_eq!(
                pixel, 0xFF00_0000,
                "pixel at (160, {row}) should be black (untouched)"
            );
            let depth = zbuf.get_depth(160, row).unwrap();
            assert!(
                depth.is_infinite(),
                "zbuf at (160, {row}) should be inf (untouched)"
            );
        }
    }

    // -----------------------------------------------------------------------
    // draw_visplane_spans tests
    // -----------------------------------------------------------------------

    #[test]
    fn draw_visplane_spans_fills_floor() {
        let mut fb = Framebuffer::new(320, 200).unwrap();
        let mut zbuf = ZBuffer::new(320, 200).unwrap();
        let textures = MockTextures::new();

        // Build a visplane spanning cols 100-200, rows 150-180
        let mut visplanes = VisplaneAllocator::new(320);
        for col in 100..=200 {
            let vp = visplanes.find_or_create(0, 1, 160, col);
            visplanes.set_span(vp, col, 150, 180);
        }

        let camera_pos = Vec2Fixed::from_ints(256, 256);
        let camera_angle = Bam::ZERO;
        let camera_z = Fixed16_16::from_int(41); // eye height above floor 0
        let projection = Fixed16_16::from_f32(160.0);

        draw_visplane_spans(
            &mut fb,
            &mut zbuf,
            &textures,
            &visplanes,
            camera_pos,
            camera_angle,
            camera_z,
            projection,
        );

        // Pixels in the visplane region should be drawn (not black).
        // Flat data is all palette index 2, colormap 0 = identity, palette[2] = green.
        let mut drawn_count = 0;
        for col in 100..=200 {
            for row in 150..=180 {
                let pixel = fb.get_pixel(col, row).unwrap();
                if pixel != 0xFF00_0000 {
                    drawn_count += 1;
                }
            }
        }
        assert!(
            drawn_count > 0,
            "visplane should have drawn at least some pixels"
        );

        // Pixels outside the visplane region should be untouched.
        assert_eq!(
            fb.get_pixel(50, 150).unwrap(),
            0xFF00_0000,
            "pixel outside visplane should be black"
        );
        assert_eq!(
            fb.get_pixel(160, 10).unwrap(),
            0xFF00_0000,
            "pixel above visplane should be black"
        );
    }

    #[test]
    fn draw_visplane_spans_empty_allocator_is_noop() {
        let mut fb = Framebuffer::new(320, 200).unwrap();
        let mut zbuf = ZBuffer::new(320, 200).unwrap();
        let textures = MockTextures::new();
        let visplanes = VisplaneAllocator::new(320);

        draw_visplane_spans(
            &mut fb,
            &mut zbuf,
            &textures,
            &visplanes,
            Vec2Fixed::from_ints(256, 256),
            Bam::ZERO,
            Fixed16_16::from_int(41),
            Fixed16_16::from_f32(160.0),
        );

        // Nothing should have been drawn -- check a sampling of pixels.
        for col in (0..320).step_by(20) {
            for row in (0..200).step_by(20) {
                let pixel = fb.get_pixel(col, row).unwrap();
                assert_eq!(
                    pixel, 0xFF00_0000,
                    "pixel at ({col}, {row}) should be black with empty allocator"
                );
            }
        }
    }

    // -----------------------------------------------------------------------
    // render_bsp_view integration tests
    // -----------------------------------------------------------------------

    use crate::bsp::{BspMap, BspSector, BspSeg};
    use abrash_core::bam::{ANG90, ANG180, ANG270};

    /// Integration BSP map: a single 512x512 room with 4 solid walls.
    struct IntegrationBspMap {
        sector: BspSector,
        segs: Vec<BspSeg>,
    }

    impl IntegrationBspMap {
        fn new() -> Self {
            let sector = BspSector {
                floor_height: 0,
                ceil_height: 128,
                light_level: 160,
                floor_texture: 1,
                ceil_texture: 2,
            };

            // Four walls of a 512x512 room. Segs are wound so the front sector
            // (interior) is on the RIGHT side of the v1->v2 direction, following
            // Doom's BSP convention. This means from a camera inside the room,
            // point_to_angle(camera, v1) > point_to_angle(camera, v2) holds,
            // giving a valid angular span < 180 degrees.
            let segs = vec![
                // North wall: (512,0) -> (0,0), seg runs west, normal=ANG90+ANG180=south-ish
                // From camera at center: v1 at -45deg, v2 at -135deg... no.
                // Actually let's think about this differently.
                // For camera at (256,256) facing east:
                //
                // East wall: must present v1 at +angle, v2 at -angle from camera.
                //   v1=(512,512) -> angle=+45, v2=(512,0) -> angle=-45. Span=90. GOOD.
                //   Seg direction is south (ANG270). Normal = ANG270+ANG90 = ANG0 = east.
                //   But we want the normal pointing WEST (inward). So seg angle = ANG90,
                //   meaning seg runs north. Let's use v1=(512,512), v2=(512,0), angle=ANG90.
                //   Normal = ANG90 + ANG90 = ANG180 = west. That's inward. GOOD.
                //
                // Actually Doom computes seg angle from v1 to v2:
                //   angle = atan2(v2.y - v1.y, v2.x - v1.x)
                //   For (512,512)->(512,0): dx=0, dy=-512 => angle=atan2(-512,0) = -90 = ANG270
                //   Normal = ANG270 + ANG90 = ANG0 = east (pointing outward, wrong!)
                //
                // We need the normal to point INWARD (toward 256,256).
                // For east wall: inward normal points west (ANG180).
                // Normal = seg_angle + ANG90 = ANG180 means seg_angle = ANG90.
                // seg_angle=ANG90 means atan2(dy,dx)=90deg, so dy>0, dx=0.
                // That's v2.y > v1.y, so v1=(512,0) -> v2=(512,512).
                // From camera: angle_to_v1 = atan2(0-256, 512-256) = atan2(-256,256) = -45deg
                //              angle_to_v2 = atan2(512-256, 512-256) = atan2(256,256) = +45deg
                // span = angle1 - angle2 = -45 - 45 = -90deg. In BAM unsigned, that wraps
                // to a huge value >= ANG180, so the seg is rejected as backfacing.
                //
                // The REAL fix: we need to swap v1/v2 AND update the angle accordingly.
                // If seg runs from (512,512) to (512,0), angle = atan2(-512,0) = ANG270.
                // Normal = ANG270 + ANG90 = 0 = east (outward). Still wrong.
                //
                // The issue is that in Doom, the "front sector" is on the RIGHT side
                // of the seg direction. For a wall at x=512 with interior to the left
                // (west), the seg should run NORTH (v1=(512,0)->v2=(512,512), angle=ANG90).
                // Front sector is on the right = west side = interior. Normal points LEFT
                // of travel = west = ANG180. But seg_perpendicular_distance uses
                // angle+ANG90 as the normal, and ANG90+ANG90 = ANG180 = west. GOOD.
                //
                // But then from camera: angle1=atan2(-256,256)=-45, angle2=atan2(256,256)=+45.
                // span = -45 - 45 = -90 in degrees => BAM wraps to >ANG180 => rejected.
                //
                // The problem is clip_seg_angles expects angle1 > angle2 (left > right).
                // For this seg, v1 is to the RIGHT and v2 is to the LEFT of center.
                // In Doom, the node builder ensures segs are ordered so that
                // angle_to_v1 > angle_to_v2 from any viewpoint in the front sector.
                // For the east wall with the camera at center, we need v1 to subtend
                // the LARGER angle. So v1 should be at (512,512) (+45 deg from camera)
                // and v2 at (512,0) (-45 deg). That gives:
                //   seg runs south: angle = atan2(0-512, 512-512) = atan2(-512,0) = ANG270
                //   normal = ANG270 + ANG90 = 0 = east (OUTWARD)
                // But we need normal inward!
                //
                // Resolution: seg_perpendicular_distance takes abs(), so the sign of
                // the normal doesn't matter for distance. And colormap_index just uses
                // distance. The wall DOES get drawn regardless of normal direction.
                // The only issue is whether the seg passes clip_seg_angles.
                //
                // So the fix is simple: order vertices so angle1 > angle2 from the
                // camera's perspective. For east wall with camera facing east:
                //   v1=(512,512) at +45deg, v2=(512,0) at -45deg. span=90deg. VALID.

                // East wall: v1=(512,512) -> v2=(512,0), seg angle = ANG270 (south)
                BspSeg {
                    v1: Vec2Fixed::from_ints(512, 512),
                    v2: Vec2Fixed::from_ints(512, 0),
                    offset: Fixed16_16::ZERO,
                    angle: ANG270,
                    front_sector: 0,
                    back_sector: None,
                    upper_texture: 0,
                    middle_texture: 1,
                    lower_texture: 0,
                    line_flags: 0x0001,
                },
                // North wall: v1=(512,0) -> v2=(0,0), seg angle = ANG180 (west)
                BspSeg {
                    v1: Vec2Fixed::from_ints(512, 0),
                    v2: Vec2Fixed::from_ints(0, 0),
                    offset: Fixed16_16::ZERO,
                    angle: ANG180,
                    front_sector: 0,
                    back_sector: None,
                    upper_texture: 0,
                    middle_texture: 1,
                    lower_texture: 0,
                    line_flags: 0x0001,
                },
                // West wall: v1=(0,0) -> v2=(0,512), seg angle = ANG90 (north)
                BspSeg {
                    v1: Vec2Fixed::from_ints(0, 0),
                    v2: Vec2Fixed::from_ints(0, 512),
                    offset: Fixed16_16::ZERO,
                    angle: ANG90,
                    front_sector: 0,
                    back_sector: None,
                    upper_texture: 0,
                    middle_texture: 1,
                    lower_texture: 0,
                    line_flags: 0x0001,
                },
                // South wall: v1=(0,512) -> v2=(512,512), seg angle = Bam::ZERO (east)
                BspSeg {
                    v1: Vec2Fixed::from_ints(0, 512),
                    v2: Vec2Fixed::from_ints(512, 512),
                    offset: Fixed16_16::ZERO,
                    angle: Bam::ZERO,
                    front_sector: 0,
                    back_sector: None,
                    upper_texture: 0,
                    middle_texture: 1,
                    lower_texture: 0,
                    line_flags: 0x0001,
                },
            ];

            Self { sector, segs }
        }
    }

    impl BspMap for IntegrationBspMap {
        fn traverse_front_to_back(&self, _pos: Vec2Fixed, visitor: &mut dyn FnMut(usize)) {
            visitor(0);
        }

        fn subsector_segs(&self, ssector_idx: usize) -> &[BspSeg] {
            assert_eq!(ssector_idx, 0, "integration map only has subsector 0");
            &self.segs
        }

        fn seg_front_sector(&self, _seg: &BspSeg) -> &BspSector {
            &self.sector
        }

        fn seg_back_sector(&self, seg: &BspSeg) -> Option<&BspSector> {
            seg.back_sector.map(|_| &self.sector)
        }
    }

    #[test]
    fn render_bsp_view_does_not_panic() {
        let mut fb = Framebuffer::new(320, 200).unwrap();
        let mut zbuf = ZBuffer::new(320, 200).unwrap();
        let map = IntegrationBspMap::new();
        let textures = MockTextures::new();

        // Camera at center of 512x512 room, facing east, eye at z=41
        render_bsp_view(
            &mut fb,
            &mut zbuf,
            &map,
            &textures,
            Vec2Fixed::from_ints(256, 256),
            Bam::ZERO,
            Fixed16_16::from_int(41),
            ANG90,
        );
        // If we get here without panic, the test passes.
    }

    #[test]
    fn render_bsp_view_draws_walls() {
        let mut fb = Framebuffer::new(320, 200).unwrap();
        let mut zbuf = ZBuffer::new(320, 200).unwrap();
        let map = IntegrationBspMap::new();
        let textures = MockTextures::new();

        render_bsp_view(
            &mut fb,
            &mut zbuf,
            &map,
            &textures,
            Vec2Fixed::from_ints(256, 256),
            Bam::ZERO,
            Fixed16_16::from_int(41),
            ANG90,
        );

        // The center pixel (160, 100) should have been drawn (wall or floor/ceil)
        let center_pixel = fb.get_pixel(160, 100).unwrap();
        assert_ne!(
            center_pixel, 0xFF00_0000,
            "center pixel should not be black -- a wall should be visible"
        );
    }

    #[test]
    fn render_bsp_view_writes_depth() {
        let mut fb = Framebuffer::new(320, 200).unwrap();
        let mut zbuf = ZBuffer::new(320, 200).unwrap();
        let map = IntegrationBspMap::new();
        let textures = MockTextures::new();

        render_bsp_view(
            &mut fb,
            &mut zbuf,
            &map,
            &textures,
            Vec2Fixed::from_ints(256, 256),
            Bam::ZERO,
            Fixed16_16::from_int(41),
            ANG90,
        );

        // Z-buffer at center should be finite and positive (a wall was drawn there).
        let depth = zbuf.get_depth(160, 100).unwrap();
        assert!(
            depth.is_finite() && depth > 0.0,
            "z-buffer at center should be finite+positive, got {depth}"
        );
    }

    #[test]
    fn render_bsp_view_zero_size_noop() {
        // 1x1 framebuffer -- should not panic.
        let mut fb = Framebuffer::new(1, 1).unwrap();
        let mut zbuf = ZBuffer::new(1, 1).unwrap();
        let map = IntegrationBspMap::new();
        let textures = MockTextures::new();

        render_bsp_view(
            &mut fb,
            &mut zbuf,
            &map,
            &textures,
            Vec2Fixed::from_ints(256, 256),
            Bam::ZERO,
            Fixed16_16::from_int(41),
            ANG90,
        );
        // No panic = success.
    }
}
