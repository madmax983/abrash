//! Texture mapping support for software rendering.
//!
//! Provides texture storage, sampling, and loading from BMP files.

use crate::math::Vec2;
use std::fmt;

/// Errors that can occur when loading BMP files
#[derive(Debug, Clone, PartialEq)]
pub enum BmpError {
    InvalidFormat,
    UnsupportedBpp(u16),
}

impl fmt::Display for BmpError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BmpError::InvalidFormat => write!(f, "Invalid BMP format"),
            BmpError::UnsupportedBpp(bpp) => write!(f, "Unsupported bits per pixel: {}", bpp),
        }
    }
}

impl std::error::Error for BmpError {}

/// A 2D texture with ARGB8888 pixel format
#[derive(Debug, Clone)]
pub struct Texture {
    pixels: Vec<u32>,
    width: u32,
    height: u32,
}

impl Texture {
    /// Create a new texture filled with black
    pub fn new(width: u32, height: u32) -> Self {
        let pixels = vec![0xFF000000; (width * height) as usize]; // Opaque black
        Self {
            pixels,
            width,
            height,
        }
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    /// Get a pixel with wrapping coordinates (supports negative indices)
    pub fn get_pixel(&self, x: i32, y: i32) -> u32 {
        let x = x.rem_euclid(self.width as i32) as u32;
        let y = y.rem_euclid(self.height as i32) as u32;
        self.pixels[(y * self.width + x) as usize]
    }

    /// Set a pixel with wrapping coordinates
    pub fn set_pixel(&mut self, x: i32, y: i32, color: u32) {
        let x = x.rem_euclid(self.width as i32) as u32;
        let y = y.rem_euclid(self.height as i32) as u32;
        self.pixels[(y * self.width + x) as usize] = color;
    }

    /// Sample texture using nearest-neighbor filtering
    /// UV coordinates are in [0, 1] range and wrap
    pub fn sample_nearest(&self, u: f32, v: f32) -> u32 {
        let u = u.rem_euclid(1.0);
        let v = v.rem_euclid(1.0);
        let x = (u * self.width as f32) as i32;
        let y = (v * self.height as f32) as i32;
        self.get_pixel(x, y)
    }

    /// Sample texture using bilinear filtering
    /// UV coordinates are in [0, 1] range and wrap
    pub fn sample_bilinear(&self, u: f32, v: f32) -> u32 {
        let u = u.rem_euclid(1.0);
        let v = v.rem_euclid(1.0);

        // Convert to texel space
        let x = u * self.width as f32 - 0.5;
        let y = v * self.height as f32 - 0.5;

        // Get integer coordinates and fractional parts
        let x0 = x.floor() as i32;
        let y0 = y.floor() as i32;
        let x1 = x0 + 1;
        let y1 = y0 + 1;

        let fx = x - x0 as f32;
        let fy = y - y0 as f32;

        // Sample 4 texels
        let c00 = self.get_pixel(x0, y0);
        let c10 = self.get_pixel(x1, y0);
        let c01 = self.get_pixel(x0, y1);
        let c11 = self.get_pixel(x1, y1);

        // Bilinear interpolation
        let c0 = lerp_color(c00, c10, fx);
        let c1 = lerp_color(c01, c11, fx);
        lerp_color(c0, c1, fy)
    }

    /// Load a texture from BMP file data
    pub fn from_bmp(data: &[u8]) -> Result<Self, BmpError> {
        // Minimum size for BMP header (14) + DIB header (40)
        if data.len() < 54 {
            return Err(BmpError::InvalidFormat);
        }

        // Check magic bytes "BM"
        if data[0] != b'B' || data[1] != b'M' {
            return Err(BmpError::InvalidFormat);
        }

        // Read DIB header
        let dib_header_size = u32::from_le_bytes([data[14], data[15], data[16], data[17]]);
        if dib_header_size != 40 {
            return Err(BmpError::InvalidFormat); // Only support BITMAPINFOHEADER
        }

        let width = i32::from_le_bytes([data[18], data[19], data[20], data[21]]) as u32;
        let height = i32::from_le_bytes([data[22], data[23], data[24], data[25]]).unsigned_abs();
        let bpp = u16::from_le_bytes([data[28], data[29]]);
        let data_offset = u32::from_le_bytes([data[10], data[11], data[12], data[13]]) as usize;

        // Only support 24-bit and 32-bit
        if bpp != 24 && bpp != 32 {
            return Err(BmpError::UnsupportedBpp(bpp));
        }

        // Check if we have enough data
        if data.len() < data_offset {
            return Err(BmpError::InvalidFormat);
        }

        let bytes_per_pixel = (bpp / 8) as usize;
        let row_size = (bpp as usize * width as usize).div_ceil(32) * 4; // Row padding to 4 bytes

        let mut pixels = vec![0u32; (width * height) as usize];

        // Read pixel data (BMP is bottom-up)
        for y in 0..height {
            let row_start = data_offset + (y as usize * row_size);
            for x in 0..width {
                let pixel_offset = row_start + (x as usize * bytes_per_pixel);

                if pixel_offset + bytes_per_pixel > data.len() {
                    return Err(BmpError::InvalidFormat);
                }

                // BMP uses BGR(A) format
                let b = data[pixel_offset] as u32;
                let g = data[pixel_offset + 1] as u32;
                let r = data[pixel_offset + 2] as u32;
                let a = if bpp == 32 {
                    data[pixel_offset + 3] as u32
                } else {
                    0xFF
                };

                // Convert to ARGB
                let color = (a << 24) | (r << 16) | (g << 8) | b;

                // Flip vertically (BMP is bottom-up)
                let flipped_y = height - 1 - y;
                pixels[(flipped_y * width + x) as usize] = color;
            }
        }

        Ok(Self {
            pixels,
            width,
            height,
        })
    }
}

/// Linear interpolation between two colors (ARGB format)
fn lerp_color(c0: u32, c1: u32, t: f32) -> u32 {
    let a0 = ((c0 >> 24) & 0xFF) as f32;
    let r0 = ((c0 >> 16) & 0xFF) as f32;
    let g0 = ((c0 >> 8) & 0xFF) as f32;
    let b0 = (c0 & 0xFF) as f32;

    let a1 = ((c1 >> 24) & 0xFF) as f32;
    let r1 = ((c1 >> 16) & 0xFF) as f32;
    let g1 = ((c1 >> 8) & 0xFF) as f32;
    let b1 = (c1 & 0xFF) as f32;

    let a = (a0 + (a1 - a0) * t) as u32;
    let r = (r0 + (r1 - r0) * t) as u32;
    let g = (g0 + (g1 - g0) * t) as u32;
    let b = (b0 + (b1 - b0) * t) as u32;

    (a << 24) | (r << 16) | (g << 8) | b
}

/// Perspective-correct UV interpolation using the 1/w trick
///
/// This ensures textures don't warp on surfaces viewed at an angle.
/// When w values differ, linear interpolation in screen space produces incorrect results.
/// The correct approach is to interpolate uv/w and 1/w linearly, then recover UV.
pub fn interpolate_uv_perspective(uv0: Vec2, w0: f32, uv1: Vec2, w1: f32, t: f32) -> Vec2 {
    let inv_w0 = 1.0 / w0;
    let inv_w1 = 1.0 / w1;

    // Interpolate uv/w and 1/w in screen space
    let uv_over_w = Vec2::lerp(uv0 * inv_w0, uv1 * inv_w1, t);
    let inv_w = inv_w0 * (1.0 - t) + inv_w1 * t;

    // Recover perspective-correct UV
    uv_over_w / inv_w
}
