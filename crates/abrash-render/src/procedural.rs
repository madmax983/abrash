//! Procedural Texture Generation Module
//!
//! Provides functions to generate textures algorithmically.

use crate::texture::Texture;
use crate::utils::XorShift32;
use abrash_core::math::fast_sin_cos;

const SIN_LUT: [i32; 256] = [
    0, 3, 6, 9, 12, 15, 18, 21, 24, 27, 30, 33, 36, 39, 42, 45, 48, 51, 54, 57, 59, 62, 65, 67, 70,
    73, 75, 78, 80, 82, 85, 87, 89, 91, 94, 96, 98, 100, 102, 103, 105, 107, 108, 110, 112, 113,
    114, 116, 117, 118, 119, 120, 121, 122, 123, 123, 124, 125, 125, 126, 126, 126, 126, 126, 127,
    126, 126, 126, 126, 126, 125, 125, 124, 123, 123, 122, 121, 120, 119, 118, 117, 116, 114, 113,
    112, 110, 108, 107, 105, 103, 102, 100, 98, 96, 94, 91, 89, 87, 85, 82, 80, 78, 75, 73, 70, 67,
    65, 62, 59, 57, 54, 51, 48, 45, 42, 39, 36, 33, 30, 27, 24, 21, 18, 15, 12, 9, 6, 3, 0, -3, -6,
    -9, -12, -15, -18, -21, -24, -27, -30, -33, -36, -39, -42, -45, -48, -51, -54, -57, -59, -62,
    -65, -67, -70, -73, -75, -78, -80, -82, -85, -87, -89, -91, -94, -96, -98, -100, -102, -103,
    -105, -107, -108, -110, -112, -113, -114, -116, -117, -118, -119, -120, -121, -122, -123, -123,
    -124, -125, -125, -126, -126, -126, -126, -126, -127, -126, -126, -126, -126, -126, -125, -125,
    -124, -123, -123, -122, -121, -120, -119, -118, -117, -116, -114, -113, -112, -110, -108, -107,
    -105, -103, -102, -100, -98, -96, -94, -91, -89, -87, -85, -82, -80, -78, -75, -73, -70, -67,
    -65, -62, -59, -57, -54, -51, -48, -45, -42, -39, -36, -33, -30, -27, -24, -21, -18, -15, -12,
    -9, -6, -3,
];

/// Generates a classic XOR texture.
///
/// # Errors
/// Returns an error if the texture dimensions are invalid.
///
/// # Examples
///
/// ```
/// use abrash_render::procedural::xor_pattern;
///
/// let tex = xor_pattern(32, 32).unwrap();
/// assert_eq!(tex.width(), 32);
/// assert_eq!(tex.height(), 32);
/// ```
pub fn xor_pattern(width: u32, height: u32) -> Result<Texture, &'static str> {
    let mut tex = Texture::new(width, height)?;
    for y in 0..height {
        for x in 0..width {
            let v = ((x ^ y) & 255) as u8;
            let color = 0xFF00_0000 | (u32::from(v) << 16) | (u32::from(v) << 8) | u32::from(v);
            tex.set_pixel(x, y, color);
        }
    }
    Ok(tex)
}

/// Generates a "Tech Grid" texture.
///
/// # Errors
/// Returns an error if the texture dimensions are invalid or `cell_size` is 0.
///
/// # Examples
///
/// ```
/// use abrash_render::procedural::grid_pattern;
///
/// let tex = grid_pattern(32, 32, 8, 0xFFFF_FFFF, 0xFF00_0000).unwrap();
/// assert_eq!(tex.width(), 32);
/// assert_eq!(tex.height(), 32);
/// ```
pub fn grid_pattern(
    width: u32,
    height: u32,
    cell_size: u32,
    line_color: u32,
    bg_color: u32,
) -> Result<Texture, &'static str> {
    if cell_size == 0 {
        return Err("Cell size must be positive");
    }
    let mut tex = Texture::new(width, height)?;
    for y in 0..height {
        for x in 0..width {
            let is_line = (x % cell_size == 0) || (y % cell_size == 0);
            tex.set_pixel(x, y, if is_line { line_color } else { bg_color });
        }
    }
    Ok(tex)
}

/// Generates static white noise.
///
/// # Errors
/// Returns an error if the texture dimensions are invalid.
///
/// # Examples
///
/// ```
/// use abrash_render::procedural::white_noise;
///
/// let tex = white_noise(32, 32, 12345).unwrap();
/// assert_eq!(tex.width(), 32);
/// assert_eq!(tex.height(), 32);
/// ```
pub fn white_noise(width: u32, height: u32, seed: u32) -> Result<Texture, &'static str> {
    let mut tex = Texture::new(width, height)?;
    let mut rng = XorShift32::new(seed);

    for y in 0..height {
        for x in 0..width {
            let v = (rng.next_u32() & 0xFF) as u8;
            let color = 0xFF00_0000 | (u32::from(v) << 16) | (u32::from(v) << 8) | u32::from(v);
            tex.set_pixel(x, y, color);
        }
    }
    Ok(tex)
}

/// Generates a plasma effect.
///
/// # Errors
/// Returns an error if the texture dimensions are invalid.
///
/// # Examples
///
/// ```
/// use abrash_render::procedural::plasma;
///
/// let tex = plasma(32, 32).unwrap();
/// assert_eq!(tex.width(), 32);
/// assert_eq!(tex.height(), 32);
/// ```
pub fn plasma(width: u32, height: u32) -> Result<Texture, &'static str> {
    let mut tex = Texture::new(width, height)?;

    for y in 0..height {
        for x in 0..width {
            let u = x;
            let v = y;

            // Use integer coordinates and scale for table lookup.
            // Adjust coordinates to approximate the old *0.1 scale mapping to a 256-entry table.
            let idx1 = (u.wrapping_mul(4)) & 255;
            let idx2 = (v.wrapping_mul(4)) & 255;
            let idx3 = ((u.wrapping_add(v)).wrapping_mul(4)) & 255;

            // Reintroduce distance metric calculation
            // Integer fast distance calculation approximation (Manhattan-ish) or true integer square root could be used.
            // Using isqrt on u^2 + v^2. u and v can be up to width/height.
            // For standard sizes (e.g. 1920^2 + 1080^2 = 4,852,800), u32 won't overflow
            let dist_sq = u.wrapping_mul(u).wrapping_add(v.wrapping_mul(v));
            let dist = dist_sq.isqrt();

            let idx4 = (dist.wrapping_mul(4)) & 255;

            let v1 = SIN_LUT[idx1 as usize];
            let v2 = SIN_LUT[idx2 as usize];
            let v3 = SIN_LUT[idx3 as usize];
            let v4 = SIN_LUT[idx4 as usize];

            let val = v1 + v2 + v3 + v4; // -508 to 508

            // Map sum to 0..255 for color lookup.
            // (val + 508) is 0..1016. Divide by 4 is 0..254
            let normalized_int = ((val + 508) / 4) as u32;

            // Use the LUT again for psychedelic color mapping
            let r_idx = (normalized_int / 2) & 255;
            let r = (SIN_LUT[r_idx as usize].abs() * 2) as u32;

            let g_idx = ((normalized_int / 2).wrapping_add(85)) & 255; // offset roughly 2.0 rad
            let g = (SIN_LUT[g_idx as usize].abs() * 2) as u32;

            let b_idx = ((normalized_int / 2).wrapping_add(170)) & 255; // offset roughly 4.0 rad
            let b = (SIN_LUT[b_idx as usize].abs() * 2) as u32;

            let color = 0xFF00_0000 | ((r & 0xFF) << 16) | ((g & 0xFF) << 8) | (b & 0xFF);
            tex.set_pixel(x, y, color);
        }
    }
    Ok(tex)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_grid_pattern() {
        let tex = grid_pattern(10, 10, 5, 0xFFFFFFFF, 0xFF000000).unwrap();
        assert_eq!(tex.width(), 10);
        assert_eq!(tex.height(), 10);
    }

    #[test]
    fn test_white_noise() {
        let tex = white_noise(10, 10, 12345).unwrap();
        assert_eq!(tex.width(), 10);
        assert_eq!(tex.height(), 10);
    }

    #[test]
    fn test_plasma() {
        let tex = plasma(10, 10).unwrap();
        assert_eq!(tex.width(), 10);
        assert_eq!(tex.height(), 10);
    }
}
