//! Procedural Texture Generation Module
//!
//! Provides functions to generate textures algorithmically.

use crate::texture::Texture;
use crate::utils::XorShift32;
use abrash_core::math::fast_sin_cos;

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
    for (y, row) in tex.pixels_mut().chunks_exact_mut(width as usize).enumerate() {
        let y_u32 = y as u32;
        for (x, pixel) in row.iter_mut().enumerate() {
            let v = ((x as u32 ^ y_u32) & 255) as u32;
            *pixel = 0xFF00_0000 | (v << 16) | (v << 8) | v;
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
    for (y, row) in tex.pixels_mut().chunks_exact_mut(width as usize).enumerate() {
        let y_is_line = (y as u32) % cell_size == 0;
        for (x, pixel) in row.iter_mut().enumerate() {
            let is_line = y_is_line || (x as u32 % cell_size == 0);
            *pixel = if is_line { line_color } else { bg_color };
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

    for pixel in tex.pixels_mut().iter_mut() {
        let v = rng.next_u32() & 0xFF;
        *pixel = 0xFF00_0000 | (v << 16) | (v << 8) | v;
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

    for (y, row) in tex.pixels_mut().chunks_exact_mut(width as usize).enumerate() {
        let v = y as f32;
        let (v2, _) = fast_sin_cos(v * 0.1);

        for (x, pixel) in row.iter_mut().enumerate() {
            let u = x as f32;

            let (v1, _) = fast_sin_cos(u * 0.1);
            let (v3, _) = fast_sin_cos((u + v) * 0.1);
            let (v4, _) = fast_sin_cos(u.mul_add(u, v * v).sqrt() * 0.1);

            let val = (v1 + v2 + v3 + v4) * 0.25; // -1 to 1
            let normalized = (val + 1.0) * 0.5; // 0 to 1

            // Map to a psychedelic palette
            let (r_sin, _) = fast_sin_cos(normalized * std::f32::consts::PI);
            let r = (r_sin.abs() * 255.0) as u32;
            let (g_sin, _) = fast_sin_cos((normalized * std::f32::consts::PI) + 2.0);
            let g = (g_sin.abs() * 255.0) as u32;
            let (b_sin, _) = fast_sin_cos((normalized * std::f32::consts::PI) + 4.0);
            let b = (b_sin.abs() * 255.0) as u32;

            *pixel = 0xFF00_0000 | ((r & 0xFF) << 16) | ((g & 0xFF) << 8) | (b & 0xFF);
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
