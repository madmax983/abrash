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
            let u = x as f32;
            let v = y as f32;

            let (v1, _) = fast_sin_cos(u * 0.1);
            let (v2, _) = fast_sin_cos(v * 0.1);
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

            let color = 0xFF00_0000 | ((r & 0xFF) << 16) | ((g & 0xFF) << 8) | (b & 0xFF);
            tex.set_pixel(x, y, color);
        }
    }
    Ok(tex)
}

/// Generates a plasma effect (optimized version).
///
/// # Errors
/// Returns an error if the texture dimensions are invalid.
///
/// # Examples
///
/// ```
/// use abrash_render::procedural::plasma_fast;
///
/// let tex = plasma_fast(32, 32).unwrap();
/// assert_eq!(tex.width(), 32);
/// assert_eq!(tex.height(), 32);
/// ```
const LUT_SIZE: usize = 1024;
pub fn plasma_fast(width: u32, height: u32) -> Result<Texture, &'static str> {
    let mut tex = Texture::new(width, height)?;
    if width == 0 || height == 0 {
        return Ok(tex);
    }

    // ⚡ Bolt: Pre-calculate the palette to a Look-Up Table (LUT)
    let mut lut = [0u32; LUT_SIZE];
    for i in 0..LUT_SIZE {
        let normalized = i as f32 / (LUT_SIZE - 1) as f32;
        let (r_sin, _) = fast_sin_cos(normalized * std::f32::consts::PI);
        let r = (r_sin.abs() * 255.0) as u32;
        let (g_sin, _) = fast_sin_cos((normalized * std::f32::consts::PI) + 2.0);
        let g = (g_sin.abs() * 255.0) as u32;
        let (b_sin, _) = fast_sin_cos((normalized * std::f32::consts::PI) + 4.0);
        let b = (b_sin.abs() * 255.0) as u32;
        lut[i] = 0xFF00_0000 | ((r & 0xFF) << 16) | ((g & 0xFF) << 8) | (b & 0xFF);
    }

    let pixels = tex.pixels.as_mut_slice();

    // ⚡ Bolt: Use .chunks_exact_mut() to elide bounds checking
    for (y, row) in pixels
        .chunks_exact_mut(width as usize)
        .enumerate()
        .take(height as usize)
    {
        let v = y as f32;
        // ⚡ Bolt: Extract y-dependent calculations outside inner loop
        let (v2, _) = fast_sin_cos(v * 0.1);
        let v_squared = v * v;

        for (x, pixel) in row.iter_mut().enumerate() {
            let u = x as f32;
            let (v1, _) = fast_sin_cos(u * 0.1);
            let (v3, _) = fast_sin_cos((u + v) * 0.1);
            // Integer fast distance squared fallback to float due to exact math requirement
            let (v4, _) = fast_sin_cos(u.mul_add(u, v_squared).sqrt() * 0.1);

            let val = (v1 + v2 + v3 + v4) * 0.25; // -1 to 1
            let normalized = (val + 1.0) * 0.5; // 0 to 1

            // ⚡ Bolt: Replace 3 trigonometric calls with a single LUT lookup
            let lut_index = (normalized * (LUT_SIZE - 1) as f32) as usize;
            let lut_index = lut_index.clamp(0, LUT_SIZE - 1);
            *pixel = lut[lut_index];
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

    #[test]
    fn test_plasma_fast() {
        let tex = plasma_fast(10, 10).unwrap();
        assert_eq!(tex.width(), 10);
        assert_eq!(tex.height(), 10);

        let mut has_non_black = false;
        for &p in tex.pixels.iter() {
            if p != 0xFF_00_00_00 {
                has_non_black = true;
                break;
            }
        }
        assert!(
            has_non_black,
            "Texture should not be entirely black after plasma_fast effect"
        );
    }
}
