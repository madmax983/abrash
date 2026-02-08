//! Procedural Texture Generation Module
//!
//! Provides functions to generate textures algorithmically.

use crate::rasterizer::Texture;

/// A simple Xorshift random number generator for deterministic noise.
struct XorShift32 {
    state: u32,
}

impl XorShift32 {
    const fn new(seed: u32) -> Self {
        Self {
            state: if seed == 0 { 0xDEAD_BEEF } else { seed },
        }
    }

    const fn next(&mut self) -> u32 {
        let mut x = self.state;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.state = x;
        x
    }
}

/// Generates a classic XOR texture.
///
/// This creates a bitwise XOR pattern often used in early computer graphics demos.
/// `color = (x ^ y) & 255`.
///
/// # Examples
///
/// ```
/// use abrash::experimental::procedural::xor_pattern;
///
/// let tex = xor_pattern(256, 256).unwrap();
/// assert_eq!(tex.width, 256);
/// ```
///
/// # Errors
/// Returns an error if the texture dimensions are invalid (e.g. 0 or too large).
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
/// Creates a grid of lines with a solid background color.
///
/// # Examples
///
/// ```
/// use abrash::experimental::procedural::grid_pattern;
///
/// let green = 0xFF00FF00;
/// let black = 0xFF000000;
/// let tex = grid_pattern(256, 256, 32, green, black).unwrap();
/// ```
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
/// Uses a simple PRNG to generate random grayscale noise.
///
/// # Examples
///
/// ```
/// use abrash::experimental::procedural::white_noise;
///
/// let tex = white_noise(256, 256, 12345).unwrap();
/// ```
///
/// # Errors
/// Returns an error if the texture dimensions are invalid.
pub fn white_noise(width: u32, height: u32, seed: u32) -> Result<Texture, &'static str> {
    let mut tex = Texture::new(width, height)?;
    let mut rng = XorShift32::new(seed);

    for y in 0..height {
        for x in 0..width {
            let v = (rng.next() & 0xFF) as u8;
            let color = 0xFF00_0000 | (u32::from(v) << 16) | (u32::from(v) << 8) | u32::from(v);
            tex.set_pixel(x, y, color);
        }
    }
    Ok(tex)
}

/// Generates a plasma effect.
///
/// Creates a smooth, psychedelic pattern using sine waves.
///
/// # Examples
///
/// ```
/// use abrash::experimental::procedural::plasma;
///
/// let tex = plasma(256, 256).unwrap();
/// ```
///
/// # Errors
/// Returns an error if the texture dimensions are invalid.
pub fn plasma(width: u32, height: u32) -> Result<Texture, &'static str> {
    let mut tex = Texture::new(width, height)?;

    for y in 0..height {
        for x in 0..width {
            let u = x as f32;
            let v = y as f32;

            let v1 = (u * 0.1).sin();
            let v2 = (v * 0.1).sin();
            let v3 = ((u + v) * 0.1).sin();
            let v4 = (u.hypot(v) * 0.1).sin();

            let val = (v1 + v2 + v3 + v4) * 0.25; // -1 to 1
            let normalized = (val + 1.0) * 0.5; // 0 to 1

            // Map to a psychedelic palette
            let r = ((normalized * std::f32::consts::PI).sin().abs() * 255.0) as u32;
            let g = (((normalized * std::f32::consts::PI) + 2.0).sin().abs() * 255.0) as u32;
            let b = (((normalized * std::f32::consts::PI) + 4.0).sin().abs() * 255.0) as u32;

            let color = 0xFF00_0000 | ((r & 0xFF) << 16) | ((g & 0xFF) << 8) | (b & 0xFF);
            tex.set_pixel(x, y, color);
        }
    }
    Ok(tex)
}
