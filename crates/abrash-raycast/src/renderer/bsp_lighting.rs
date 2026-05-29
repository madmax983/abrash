//! BSP texture provider struct and distance-based colormap lighting.
//!
//! Consumers implement [`BspTextureCache`] to bridge their texture caches (WAD
//! lumps, custom atlases, etc.) into the BSP renderer.  The [`colormap_index`]
//! function converts a distance + sector light level into a Doom-style
//! colormap row index (0 = bright, 31 = dark).

use abrash_core::fixed16_16::Fixed16_16;

// ---------------------------------------------------------------------------
// BspTextureCache
// ---------------------------------------------------------------------------

/// Texture data provider for BSP rendering.
pub struct BspTextureCache {
    wall_column_data: Vec<u8>,
    flat: [u8; 4096],
    colormap_data: [[u8; 256]; 32],
    palette: [u32; 256],
}

impl BspTextureCache {
    #[must_use]
    pub fn new() -> Self {
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

        // Palette: simple grayscale
        let mut palette = [0u32; 256];
        palette[0] = 0xFF00_0000; // black
        palette[1] = 0xFFFF_0000; // red (wall)
        palette[2] = 0xFF00_00FF; // blue (flat)

        Self {
            wall_column_data,
            flat,
            colormap_data,
            palette,
        }
    }

    /// Get a single column of wall texture data (palette-indexed, top to bottom).
    /// Returns palette indices, one per texel row.  Tiles vertically.
    #[must_use]
    pub fn wall_column(&self, _texture_id: u16, _col: usize) -> &[u8] {
        &self.wall_column_data
    }

    /// Get a 64x64 flat texture (4096 bytes, palette-indexed, row-major).
    #[must_use]
    pub const fn flat_data(&self, _texture_id: u16) -> &[u8; 4096] {
        &self.flat
    }

    /// Get a 256-byte colormap row.  index 0 = bright, 31 = darkest.
    #[must_use]
    pub fn colormap(&self, index: u8) -> &[u8; 256] {
        let index = index.min(31) as usize;
        &self.colormap_data[index]
    }

    /// Look up palette entry (palette index -> 0xAARRGGBB).
    #[must_use]
    pub const fn palette_argb(&self, palette_idx: u8) -> u32 {
        self.palette[palette_idx as usize]
    }
}

impl Default for BspTextureCache {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Colormap index computation
// ---------------------------------------------------------------------------

/// Maximum colormap index (darkest shade).
pub const MAX_COLORMAP: u8 = 31;

/// Compute colormap index for distance-based shading.
///
/// Returns 0 (brightest) to 31 (darkest).
/// Doom's model: farther distance + darker sector -> higher index.
///
/// * `distance` -- world-space distance in 16.16 fixed-point.
/// * `light_level` -- sector light level (0 = pitch black, 255 = full bright).
#[inline]
#[must_use]
pub fn colormap_index(distance: Fixed16_16, light_level: u16) -> u8 {
    // base_light: map 0-255 into roughly 0-31
    let base_light = (light_level / 8) as u8;

    // distance_fade: 1 unit of fade per 16 world units, clamped to 0-31
    let raw_fade = (distance.to_f32() / 16.0) as i32;
    let distance_fade = raw_fade.clamp(0, i32::from(MAX_COLORMAP)) as u8;

    // Final index: more distance and less light -> higher (darker) index
    let index = distance_fade.saturating_sub(base_light);
    index.min(MAX_COLORMAP)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn colormap_close_bright_sector_is_zero() {
        let distance = Fixed16_16::from_int(1);
        let idx = colormap_index(distance, 255);
        assert_eq!(idx, 0, "close + bright should be 0");
    }

    #[test]
    fn colormap_far_dark_sector_is_max() {
        let distance = Fixed16_16::from_int(2048);
        let idx = colormap_index(distance, 0);
        assert_eq!(idx, MAX_COLORMAP, "far + dark should be MAX_COLORMAP");
    }

    #[test]
    fn colormap_increases_with_distance() {
        // Use light_level=0 so base_light=0, making distance the sole factor
        let near = colormap_index(Fixed16_16::from_int(32), 0);
        let far = colormap_index(Fixed16_16::from_int(256), 0);
        assert!(
            near < far,
            "near ({near}) should be less than far ({far}) at same light"
        );
    }

    #[test]
    fn colormap_decreases_with_light() {
        // Use a large distance so the fade is significant, then vary light
        let dark = colormap_index(Fixed16_16::from_int(400), 64);
        let bright = colormap_index(Fixed16_16::from_int(400), 200);
        assert!(
            bright < dark,
            "bright ({bright}) should be less than dark ({dark}) at same distance"
        );
    }

    #[test]
    fn colormap_clamped_to_range() {
        // Very far, very dark -- should not exceed 31
        let far_dark = colormap_index(Fixed16_16::from_int(10_000), 0);
        assert!(far_dark <= MAX_COLORMAP);

        // Very close, very bright -- should not go below 0
        let close_bright = colormap_index(Fixed16_16::from_int(0), 255);
        assert_eq!(close_bright, 0);

        // Negative distance (edge case) -- should clamp to 0 fade
        let negative = colormap_index(Fixed16_16::from_int(-100), 0);
        assert!(negative <= MAX_COLORMAP);
    }
}
