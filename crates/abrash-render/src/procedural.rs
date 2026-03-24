//! Procedural Texture Generation Module
//!
//! Provides functions to generate textures algorithmically.

use crate::texture::Texture;
use crate::utils::XorShift32;

/// Generates a classic XOR texture.
///
/// # Errors
/// Returns an error if the texture dimensions are invalid.
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
#[allow(clippy::imprecise_flops)]
pub fn plasma(width: u32, height: u32) -> Result<Texture, &'static str> {
    let mut tex = Texture::new(width, height)?;

    for y in 0..height {
        for x in 0..width {
            let u = x as f32;
            let v = y as f32;

            let v1 = crate::math::fast_sin(u * 0.1);
            let v2 = crate::math::fast_sin(v * 0.1);
            let v3 = crate::math::fast_sin((u + v) * 0.1);
            let v4 = crate::math::fast_sin((u * u + v * v).sqrt() * 0.1);

            let val = (v1 + v2 + v3 + v4) * 0.25; // -1 to 1
            let normalized = (val + 1.0) * 0.5; // 0 to 1

            // Map to a psychedelic palette
            let r = (crate::math::fast_sin(normalized * std::f32::consts::PI).abs() * 255.0) as u32;
            let g = (crate::math::fast_sin((normalized * std::f32::consts::PI) + 2.0).abs() * 255.0) as u32;
            let b = (crate::math::fast_sin((normalized * std::f32::consts::PI) + 4.0).abs() * 255.0) as u32;

            let color = 0xFF00_0000 | ((r & 0xFF) << 16) | ((g & 0xFF) << 8) | (b & 0xFF);
            tex.set_pixel(x, y, color);
        }
    }
    Ok(tex)
}
