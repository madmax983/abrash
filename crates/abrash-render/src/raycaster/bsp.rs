//! BSP wall column renderer.
//!
//! Draws a single textured, colormap-shaded wall column into a framebuffer
//! with depth writes.  This is the innermost loop of a Doom-style BSP
//! renderer -- called once per screen column per visible wall segment.

use abrash_core::fixed16_16::Fixed16_16;
use abrash_core::framebuffer::Framebuffer;
use abrash_core::zbuffer::ZBuffer;

use super::bsp_lighting::BspTextures;

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
}
