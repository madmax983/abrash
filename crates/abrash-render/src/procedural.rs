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
static PLASMA_LUT: std::sync::OnceLock<[u32; 1024]> = std::sync::OnceLock::new();

/// # Errors
/// Returns an error if the texture dimensions are zero.
pub fn plasma(width: u32, height: u32) -> Result<Texture, &'static str> {
    if width == 0 || height == 0 {
        return Err("Texture dimensions must be positive");
    }

    let mut tex = Texture::new(width, height)?;
    let lut = PLASMA_LUT.get_or_init(|| {
        let mut local_lut = [0u32; 1024];
        for t in 0..1024 {
            let normalized = t as f32 / 1023.0;
            let r_sin = (normalized * std::f32::consts::PI).sin();
            let r = (r_sin.abs() * 255.0) as u32;

            let g_sin = ((normalized * std::f32::consts::PI) + 2.0).sin();
            let g = (g_sin.abs() * 255.0) as u32;

            let b_sin = ((normalized * std::f32::consts::PI) + 4.0).sin();
            let b = (b_sin.abs() * 255.0) as u32;

            local_lut[t] = 0xFF00_0000 | ((r & 0xFF) << 16) | ((g & 0xFF) << 8) | (b & 0xFF);
        }
        local_lut
    });

    let width_usize = width as usize;
    let mut x_sincos = Vec::with_capacity(width_usize);
    for x in 0..width {
        let u = x as f32;
        let (sin_u, cos_u) = fast_sin_cos(u * 0.1);
        let u_v1 = sin_u;
        x_sincos.push((u, u_v1, sin_u, cos_u));
    }

    for (y, row) in tex.pixels_mut().chunks_exact_mut(width_usize).enumerate() {
        let v = y as f32;
        let (sin_v, cos_v) = fast_sin_cos(v * 0.1);
        let v_v2 = sin_v;
        let v_sq = v * v;

        for (x, p) in row.iter_mut().enumerate() {
            let (u, u_v1, sin_u, cos_u) = unsafe { *x_sincos.get_unchecked(x) };

            // v3 = sin((u + v) * 0.1) = sin_u*cos_v + cos_u*sin_v
            let v3 = sin_u * cos_v + cos_u * sin_v;

            let (v4, _) = fast_sin_cos(u.mul_add(u, v_sq).sqrt() * 0.1);

            let val = (u_v1 + v_v2 + v3 + v4) * 0.25; // -1 to 1
            let normalized = (val + 1.0) * 0.5; // 0 to 1

            let lut_idx = (normalized * 1023.0) as usize;
            let lut_idx = lut_idx.min(1023);

            *p = unsafe { *lut.get_unchecked(lut_idx) };
        }
    }

    Ok(tex)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plasma_zero_dimensions() {
        assert!(plasma(0, 10).is_err());
        assert!(plasma(10, 0).is_err());
        assert!(plasma(0, 0).is_err());
    }

    #[test]
    fn test_grid_pattern() {
        let tex = grid_pattern(10, 10, 5, 0xFFFF_FFFF, 0xFF00_0000).unwrap();
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
