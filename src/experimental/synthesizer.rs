#![allow(
    clippy::unreadable_literal,
    clippy::missing_const_for_fn,
    clippy::needless_pass_by_value,
    clippy::manual_range_contains,
    clippy::needless_range_loop
)]
//! Nova Procedural Texture Synthesizer 🌟
//!
//! An experimental feature to generate procedural textures entirely at runtime.
//! This allows for memory-free texture generation like noise, gradients, and
//! checkerboards which can be blended together using a functional pipeline.
//!
//! # Example
//! ```
//! use abrash::experimental::synthesizer::{Synthesizer, Solid, Checkerboard, Blend};
//! let bg = Solid::new(0xFF_FF_00_00); // Red
//! let fg = Solid::new(0xFF_00_FF_00); // Green
//! let pattern = Checkerboard::new(bg, fg, 16);
//! let color = pattern.sample(0.5, 0.5);
//! ```

use crate::texture::blend_swar;
use std::num::Wrapping;

/// The core trait for all procedural texture generators.
pub trait Synthesizer: Sync + Send {
    /// Samples the procedural texture at the given UV coordinates [0.0, 1.0].
    /// Returns a packed 32-bit ARGB color.
    #[must_use]
    fn sample(&self, u: f32, v: f32) -> u32;
}

/// A solid color generator.
pub struct Solid {
    color: u32,
}

impl Solid {
    #[must_use]
    pub fn new(color: u32) -> Self {
        Self { color }
    }
}

impl Synthesizer for Solid {
    #[inline(always)]
    fn sample(&self, _u: f32, _v: f32) -> u32 {
        self.color
    }
}

/// A linear gradient generator from Left (u=0) to Right (u=1).
pub struct LinearGradient {
    start_color: u32,
    end_color: u32,
}

impl LinearGradient {
    #[must_use]
    pub fn new(start_color: u32, end_color: u32) -> Self {
        Self {
            start_color,
            end_color,
        }
    }
}

impl Synthesizer for LinearGradient {
    #[inline]
    fn sample(&self, u: f32, _v: f32) -> u32 {
        let t = u.clamp(0.0, 1.0);
        let weight = (t * 256.0) as u32;
        // SWAR args: c0, c1, w, inv_w (c0 * w + c1 * inv_w) / 256.
        // End color is c0 and Start color is c1 based on how weight scales (t=1 -> weight=256 -> fully c0)
        blend_swar(self.end_color, self.start_color, weight, 256 - weight)
    }
}

/// A classic checkerboard pattern generator.
pub struct Checkerboard<A, B> {
    a: A,
    b: B,
    scale: f32,
}

impl<A: Synthesizer, B: Synthesizer> Checkerboard<A, B> {
    #[must_use]
    pub fn new(a: A, b: B, squares_per_axis: u32) -> Self {
        Self {
            a,
            b,
            scale: squares_per_axis as f32,
        }
    }
}

impl<A: Synthesizer, B: Synthesizer> Synthesizer for Checkerboard<A, B> {
    #[inline]
    fn sample(&self, u: f32, v: f32) -> u32 {
        let x = (u * self.scale).floor() as i32;
        let y = (v * self.scale).floor() as i32;

        if (x + y) % 2 == 0 {
            self.a.sample(u, v)
        } else {
            self.b.sample(u, v)
        }
    }
}

/// A basic Value Noise generator for organic textures (clouds, dirt).
pub struct ValueNoise {
    seed: u32,
    scale: f32,
    color1: u32,
    color2: u32,
}

impl ValueNoise {
    #[must_use]
    pub fn new(seed: u32, scale: f32, color1: u32, color2: u32) -> Self {
        Self {
            seed,
            scale,
            color1,
            color2,
        }
    }

    #[inline(always)]
    fn hash(&self, x: i32, y: i32) -> f32 {
        let mut n = Wrapping(x as u32)
            + Wrapping(y as u32) * Wrapping(57)
            + Wrapping(self.seed) * Wrapping(131);
        n = (n << 13) ^ n;

        // Manual wrapping calculation without relying on unstable `Div` for `Wrapping<f32>`
        let p1 = n.0.wrapping_mul(n.0).wrapping_mul(n.0).wrapping_mul(15731);
        let p2 = n.0.wrapping_mul(789221);
        let p3 = 1376312589;
        let res = 1.0 - ((p1.wrapping_add(p2).wrapping_add(p3)) as f32 / 1073741824.0);

        (res * 0.5) + 0.5 // Map to 0.0 - 1.0
    }

    #[inline]
    fn smooth_step(t: f32) -> f32 {
        t * t * (3.0 - 2.0 * t)
    }
}

impl Synthesizer for ValueNoise {
    fn sample(&self, u: f32, v: f32) -> u32 {
        let x = u * self.scale;
        let y = v * self.scale;

        let ix = x.floor() as i32;
        let iy = y.floor() as i32;

        let fx = x - ix as f32;
        let fy = y - iy as f32;

        let v1 = self.hash(ix, iy);
        let v2 = self.hash(ix + 1, iy);
        let v3 = self.hash(ix, iy + 1);
        let v4 = self.hash(ix + 1, iy + 1);

        let sx = Self::smooth_step(fx);
        let sy = Self::smooth_step(fy);

        let i1 = v1 + sx * (v2 - v1);
        let i2 = v3 + sx * (v4 - v3);
        let val = i1 + sy * (i2 - i1);

        let weight = (val.clamp(0.0, 1.0) * 256.0) as u32;
        blend_swar(self.color1, self.color2, weight, 256 - weight)
    }
}

/// A blend combinator that mixes two synthesizers based on an alpha value [0.0, 1.0].
pub struct Blend<A, B> {
    a: A,
    b: B,
    alpha: f32,
}

impl<A: Synthesizer, B: Synthesizer> Blend<A, B> {
    #[must_use]
    pub fn new(a: A, b: B, alpha: f32) -> Self {
        Self {
            a,
            b,
            alpha: alpha.clamp(0.0, 1.0),
        }
    }
}

impl<A: Synthesizer, B: Synthesizer> Synthesizer for Blend<A, B> {
    #[inline]
    fn sample(&self, u: f32, v: f32) -> u32 {
        let color_a = self.a.sample(u, v);
        let color_b = self.b.sample(u, v);
        let weight = (self.alpha * 256.0) as u32;
        blend_swar(color_a, color_b, weight, 256 - weight)
    }
}
