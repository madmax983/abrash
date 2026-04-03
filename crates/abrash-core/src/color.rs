//! Linear-space RGBA color type with sRGB, HSV, and HSL conversion.
//!
//! All math (lerp, blend) operates in **linear** light space. Conversion to/from
//! the perceptual sRGB encoding used by monitors and image files is explicit via
//! `from_srgb_*` / `to_srgb_*`. Mixing colors in sRGB produces the well-known
//! "too-dark midtones" artifact; always blend in linear space.
//!
//! The framebuffer uses packed `0xAARRGGBB` u32 values in sRGB. Use
//! [`Color::from_argb_u32`] / [`Color::to_argb_u32`] to bridge the gap.
//!
//! # Examples
//!
//! ```
//! use abrash_core::color::Color;
//!
//! // Mid-grey in sRGB (#808080) is ~21.6% energy, not 50%
//! let grey = Color::from_srgb_hex(0x808080FF);
//! assert!((grey.r - 0.2159).abs() < 0.001);
//!
//! // Porter-Duff "over" composite
//! let red  = Color::rgb(1.0, 0.0, 0.0);
//! let blue = Color::rgb(0.0, 0.0, 1.0);
//! let half_red = Color::new(1.0, 0.0, 0.0, 0.5);
//! let blended = Color::blend_over(half_red, blue);
//! assert!((blended.r - 0.5).abs() < 1e-5);
//! ```

use crate::math::{lerp, saturate};

/// An RGBA color in **linear** (physical) light space, components in `[0.0, 1.0]`.
///
/// Values outside `[0.0, 1.0]` are valid for HDR workflows; call [`Color::saturate`]
/// before converting to 8-bit output.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Color {
    /// Red channel (linear light).
    pub r: f32,
    /// Green channel (linear light).
    pub g: f32,
    /// Blue channel (linear light).
    pub b: f32,
    /// Alpha channel (linear; 1.0 = fully opaque).
    pub a: f32,
}

impl Color {
    /// Fully opaque black.
    pub const BLACK: Self = Self::rgb(0.0, 0.0, 0.0);
    /// Fully opaque white.
    pub const WHITE: Self = Self::rgb(1.0, 1.0, 1.0);
    /// Fully opaque red.
    pub const RED: Self = Self::rgb(1.0, 0.0, 0.0);
    /// Fully opaque green.
    pub const GREEN: Self = Self::rgb(0.0, 1.0, 0.0);
    /// Fully opaque blue.
    pub const BLUE: Self = Self::rgb(0.0, 0.0, 1.0);
    /// Fully opaque yellow.
    pub const YELLOW: Self = Self::rgb(1.0, 1.0, 0.0);
    /// Fully opaque cyan.
    pub const CYAN: Self = Self::rgb(0.0, 1.0, 1.0);
    /// Fully opaque magenta.
    pub const MAGENTA: Self = Self::rgb(1.0, 0.0, 1.0);
    /// Fully transparent (zero energy).
    pub const TRANSPARENT: Self = Self::new(0.0, 0.0, 0.0, 0.0);

    /// Construct from explicit components (linear light space).
    #[must_use]
    #[inline]
    pub const fn new(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { r, g, b, a }
    }

    /// Opaque color from RGB components (linear light space).
    #[must_use]
    #[inline]
    pub const fn rgb(r: f32, g: f32, b: f32) -> Self {
        Self::new(r, g, b, 1.0)
    }

    /// Grey with given luminance in linear space.
    #[must_use]
    #[inline]
    pub const fn grey(v: f32) -> Self {
        Self::rgb(v, v, v)
    }

    // ── sRGB conversions ──────────────────────────────────────────────────────

    /// Construct from sRGB u8 components (0–255) + linear alpha (0–255).
    ///
    /// RGB components are gamma-expanded (sRGB → linear light).
    /// Alpha is treated as linear (no gamma).
    #[must_use]
    pub fn from_srgb_u8(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self {
            r: srgb_to_linear(f32::from(r) * (1.0 / 255.0)),
            g: srgb_to_linear(f32::from(g) * (1.0 / 255.0)),
            b: srgb_to_linear(f32::from(b) * (1.0 / 255.0)),
            a: f32::from(a) * (1.0 / 255.0),
        }
    }

    /// Construct from a packed `0xRRGGBBAA` hex value (sRGB with linear alpha).
    ///
    /// Useful for CSS-style color literals: `Color::from_srgb_hex(0xFF8800FF)`.
    #[must_use]
    pub fn from_srgb_hex(rgba: u32) -> Self {
        let r = ((rgba >> 24) & 0xFF) as u8;
        let g = ((rgba >> 16) & 0xFF) as u8;
        let b = ((rgba >> 8) & 0xFF) as u8;
        let a = (rgba & 0xFF) as u8;
        Self::from_srgb_u8(r, g, b, a)
    }

    /// Construct from a packed framebuffer `0xAARRGGBB` value (sRGB).
    #[must_use]
    pub fn from_argb_u32(argb: u32) -> Self {
        let a = ((argb >> 24) & 0xFF) as u8;
        let r = ((argb >> 16) & 0xFF) as u8;
        let g = ((argb >> 8) & 0xFF) as u8;
        let b = (argb & 0xFF) as u8;
        Self::from_srgb_u8(r, g, b, a)
    }

    /// Convert to packed framebuffer `0xAARRGGBB` u32 (sRGB).
    ///
    /// Components are clamped to `[0.0, 1.0]` before encoding.
    #[must_use]
    pub fn to_argb_u32(self) -> u32 {
        let r = (linear_to_srgb(self.r.clamp(0.0, 1.0)) * 255.0 + 0.5) as u32;
        let g = (linear_to_srgb(self.g.clamp(0.0, 1.0)) * 255.0 + 0.5) as u32;
        let b = (linear_to_srgb(self.b.clamp(0.0, 1.0)) * 255.0 + 0.5) as u32;
        let a = (self.a.clamp(0.0, 1.0) * 255.0 + 0.5) as u32;
        (a << 24) | (r << 16) | (g << 8) | b
    }

    /// Convert to sRGB f32 tuple `(r, g, b, a)` in `[0.0, 1.0]`.
    #[must_use]
    pub fn to_srgb_f32(self) -> (f32, f32, f32, f32) {
        (
            linear_to_srgb(self.r.clamp(0.0, 1.0)),
            linear_to_srgb(self.g.clamp(0.0, 1.0)),
            linear_to_srgb(self.b.clamp(0.0, 1.0)),
            self.a.clamp(0.0, 1.0),
        )
    }

    // ── HSV / HSL conversions ─────────────────────────────────────────────────

    /// Construct from HSV (hue 0–360°, saturation 0–1, value 0–1) in linear space.
    ///
    /// Hue wraps: values outside `[0, 360)` are taken modulo 360.
    #[must_use]
    pub fn from_hsv(h: f32, s: f32, v: f32) -> Self {
        let s = s.clamp(0.0, 1.0);
        let v = v.clamp(0.0, 1.0);
        if s == 0.0 {
            return Self::rgb(v, v, v);
        }
        let h = h.rem_euclid(360.0) / 60.0;
        let i = h as u32;
        let f = h - i as f32;
        let p = v * (1.0 - s);
        let q = v * (1.0 - s * f);
        let t = v * (1.0 - s * (1.0 - f));
        let (r, g, b) = match i {
            0 => (v, t, p),
            1 => (q, v, p),
            2 => (p, v, t),
            3 => (p, q, v),
            4 => (t, p, v),
            _ => (v, p, q),
        };
        Self::rgb(r, g, b)
    }

    /// Convert this color to HSV `(hue 0–360°, saturation 0–1, value 0–1)`.
    ///
    /// Components are clamped to `[0.0, 1.0]` before conversion.
    #[must_use]
    pub fn to_hsv(self) -> (f32, f32, f32) {
        let r = self.r.clamp(0.0, 1.0);
        let g = self.g.clamp(0.0, 1.0);
        let b = self.b.clamp(0.0, 1.0);
        let cmax = r.max(g).max(b);
        let cmin = r.min(g).min(b);
        let delta = cmax - cmin;

        let v = cmax;
        let s = if cmax < 1e-6 { 0.0 } else { delta / cmax };
        let h = if delta < 1e-6 {
            0.0
        } else if r >= g && r >= b {
            60.0 * (((g - b) / delta).rem_euclid(6.0))
        } else if g >= b {
            60.0 * ((b - r) / delta + 2.0)
        } else {
            60.0 * ((r - g) / delta + 4.0)
        };
        (h, s, v)
    }

    /// Construct from HSL (hue 0–360°, saturation 0–1, lightness 0–1) in linear space.
    #[must_use]
    pub fn from_hsl(h: f32, s: f32, l: f32) -> Self {
        let s = s.clamp(0.0, 1.0);
        let l = l.clamp(0.0, 1.0);
        if s == 0.0 {
            return Self::rgb(l, l, l);
        }
        let q = if l < 0.5 {
            l * (1.0 + s)
        } else {
            l + s - l * s
        };
        let p = 2.0 * l - q;
        let h = h / 360.0;
        Self::rgb(
            hue_to_rgb(p, q, h + 1.0 / 3.0),
            hue_to_rgb(p, q, h),
            hue_to_rgb(p, q, h - 1.0 / 3.0),
        )
    }

    /// Convert this color to HSL `(hue 0–360°, saturation 0–1, lightness 0–1)`.
    #[must_use]
    pub fn to_hsl(self) -> (f32, f32, f32) {
        let r = self.r.clamp(0.0, 1.0);
        let g = self.g.clamp(0.0, 1.0);
        let b = self.b.clamp(0.0, 1.0);
        let cmax = r.max(g).max(b);
        let cmin = r.min(g).min(b);
        let delta = cmax - cmin;
        let l = (cmax + cmin) * 0.5;
        let s = if delta < 1e-6 {
            0.0
        } else {
            delta / (1.0 - (2.0 * l - 1.0).abs())
        };
        let h = if delta < 1e-6 {
            0.0
        } else if r >= g && r >= b {
            60.0 * (((g - b) / delta).rem_euclid(6.0))
        } else if g >= b {
            60.0 * ((b - r) / delta + 2.0)
        } else {
            60.0 * ((r - g) / delta + 4.0)
        };
        (h, s, l)
    }

    // ── Color operations ──────────────────────────────────────────────────────

    /// Linear interpolation between two colors (in linear space).
    #[must_use]
    #[inline]
    pub fn lerp(self, other: Self, t: f32) -> Self {
        Self {
            r: lerp(self.r, other.r, t),
            g: lerp(self.g, other.g, t),
            b: lerp(self.b, other.b, t),
            a: lerp(self.a, other.a, t),
        }
    }

    /// Clamp all components to `[0.0, 1.0]`.
    #[must_use]
    #[inline]
    pub const fn saturate(self) -> Self {
        Self {
            r: saturate(self.r),
            g: saturate(self.g),
            b: saturate(self.b),
            a: saturate(self.a),
        }
    }

    /// Convert to premultiplied alpha: `(r*a, g*a, b*a, a)`.
    ///
    /// Premultiplied alpha simplifies compositing math and avoids fringe
    /// artifacts when texture-filtering semi-transparent edges.
    #[must_use]
    #[inline]
    pub fn premultiply(self) -> Self {
        Self {
            r: self.r * self.a,
            g: self.g * self.a,
            b: self.b * self.a,
            a: self.a,
        }
    }

    /// Porter-Duff "over" composite: `src` on top of `dst`.
    ///
    /// Both colors should be in **straight** (non-premultiplied) alpha.
    /// `src.a = 1.0` fully replaces `dst`.
    #[must_use]
    #[inline]
    pub fn blend_over(src: Self, dst: Self) -> Self {
        let inv = 1.0 - src.a;
        Self {
            r: src.r * src.a + dst.r * inv,
            g: src.g * src.a + dst.g * inv,
            b: src.b * src.a + dst.b * inv,
            a: src.a + dst.a * inv,
        }
    }

    /// Additive blend: `src + dst` (clamped to `[0.0, 1.0]` if desired — use
    /// [`Color::saturate`]).
    #[must_use]
    #[inline]
    pub fn blend_add(self, other: Self) -> Self {
        Self {
            r: self.r + other.r,
            g: self.g + other.g,
            b: self.b + other.b,
            a: (self.a + other.a).min(1.0),
        }
    }

    /// Multiply blend: component-wise product (darkens).
    #[must_use]
    #[inline]
    pub fn blend_multiply(self, other: Self) -> Self {
        Self {
            r: self.r * other.r,
            g: self.g * other.g,
            b: self.b * other.b,
            a: self.a * other.a,
        }
    }

    /// Screen blend: `1 - (1-a)(1-b)` per channel (brightens).
    #[must_use]
    #[inline]
    pub fn blend_screen(self, other: Self) -> Self {
        Self {
            r: 1.0 - (1.0 - self.r) * (1.0 - other.r),
            g: 1.0 - (1.0 - self.g) * (1.0 - other.g),
            b: 1.0 - (1.0 - self.b) * (1.0 - other.b),
            a: self.a,
        }
    }

    /// Perceptual luminance: `0.2126·R + 0.7152·G + 0.0722·B` (ITU-R BT.709).
    ///
    /// Returns a value in `[0.0, 1.0]` for colors in `[0.0, 1.0]`.
    #[must_use]
    #[inline]
    pub fn luminance(self) -> f32 {
        self.r * 0.2126 + self.g * 0.7152 + self.b * 0.0722
    }

    /// Multiply RGB channels by a scalar (scale brightness), preserving alpha.
    #[must_use]
    #[inline]
    pub fn scale_rgb(self, s: f32) -> Self {
        Self {
            r: self.r * s,
            g: self.g * s,
            b: self.b * s,
            a: self.a,
        }
    }

    /// Return this color with a new alpha value.
    #[must_use]
    #[inline]
    pub const fn with_alpha(self, a: f32) -> Self {
        Self {
            r: self.r,
            g: self.g,
            b: self.b,
            a,
        }
    }

    /// Convert to greyscale using the luminance formula.
    #[must_use]
    #[inline]
    pub fn to_greyscale(self) -> Self {
        let l = self.luminance();
        Self::new(l, l, l, self.a)
    }

    /// Convert this linear-light RGB color to **Oklab** `(L, a, b)`.
    ///
    /// Oklab is a perceptually uniform color space designed by Björn Ottosson (2020).
    /// Equal Euclidean distances in Oklab correspond to equal perceived color differences,
    /// making it far superior to HSL/HSV for:
    /// - Gradient generation without muddy midpoints
    /// - Palette interpolation
    /// - Color-aware desaturation
    ///
    /// `L ∈ [0, 1]` is perceived lightness; `a` and `b` are opponent-color axes.
    ///
    /// # Examples
    ///
    /// ```
    /// use abrash_core::color::Color;
    ///
    /// let white = Color::WHITE;
    /// let (l, a, b) = white.to_oklab();
    /// assert!((l - 1.0).abs() < 0.01);
    /// assert!(a.abs() < 0.01);
    /// assert!(b.abs() < 0.01);
    /// ```
    #[must_use]
    pub fn to_oklab(self) -> (f32, f32, f32) {
        // Step 1: linear sRGB → XYZ-like intermediate (Oklab M1 matrix)
        let l = 0.412_221_5 * self.r + 0.536_332_6 * self.g + 0.051_445_9 * self.b;
        let m = 0.211_903_5 * self.r + 0.680_699_5 * self.g + 0.107_396_9 * self.b;
        let s = 0.088_302_5 * self.r + 0.281_718_8 * self.g + 0.629_978_8 * self.b;

        // Step 2: cube-root non-linearity
        let l_ = l.cbrt();
        let m_ = m.cbrt();
        let s_ = s.cbrt();

        // Step 3: Oklab M2 matrix → (L, a, b)
        let ok_l = 0.210_454_26 * l_ + 0.793_617_8 * m_ - 0.004_072_047 * s_;
        let ok_a = 1.977_998_5 * l_ - 2.428_592_2 * m_ + 0.450_593_7 * s_;
        let ok_b = 0.025_904_04 * l_ + 0.782_771_77 * m_ - 0.808_675_77 * s_;

        (ok_l, ok_a, ok_b)
    }

    /// Construct a `Color` from **Oklab** `(L, a, b)` coordinates.
    ///
    /// This is the inverse of [`Color::to_oklab`].
    /// The alpha channel defaults to `1.0` (fully opaque).
    ///
    /// # Examples
    ///
    /// ```
    /// use abrash_core::color::Color;
    ///
    /// let (l, a, b) = Color::RED.to_oklab();
    /// let back = Color::from_oklab(l, a, b);
    /// assert!((back.r - 1.0).abs() < 0.01);
    /// assert!(back.g.abs() < 0.01);
    /// ```
    #[must_use]
    pub fn from_oklab(ok_l: f32, ok_a: f32, ok_b: f32) -> Self {
        // Inverse M2
        let l_ = ok_l + 0.396_337_78 * ok_a + 0.215_803_76 * ok_b;
        let m_ = ok_l - 0.105_561_346 * ok_a - 0.063_854_17 * ok_b;
        let s_ = ok_l - 0.089_484_18 * ok_a - 1.291_485_5 * ok_b;

        // Cube
        let l = l_ * l_ * l_;
        let m = m_ * m_ * m_;
        let s = s_ * s_ * s_;

        // Inverse M1
        let r = 4.076_741_7 * l - 3.307_711_6 * m + 0.230_969_94 * s;
        let g = -1.268_438 * l + 2.609_757_4 * m - 0.341_319_38 * s;
        let b = -0.004_196_086 * l - 0.703_418_6 * m + 1.707_614_7 * s;

        Self::new(r.clamp(0.0, 1.0), g.clamp(0.0, 1.0), b.clamp(0.0, 1.0), 1.0)
    }

    /// Lerp two colors in **Oklab** space for perceptually smooth gradients.
    ///
    /// Unlike RGB lerp (which can produce muddy or over-saturated midpoints),
    /// Oklab lerp maintains constant perceived lightness and hue progression.
    ///
    /// # Examples
    ///
    /// ```
    /// use abrash_core::color::Color;
    ///
    /// let mid = Color::RED.lerp_oklab(Color::BLUE, 0.5);
    /// // Midpoint should be a neutral purple — not overly dark
    /// assert!(mid.r > 0.0 && mid.b > 0.0);
    /// ```
    #[must_use]
    pub fn lerp_oklab(self, other: Self, t: f32) -> Self {
        let (l0, a0, b0) = self.to_oklab();
        let (l1, a1, b1) = other.to_oklab();
        let alpha = self.a + (other.a - self.a) * t;
        let mut c = Self::from_oklab(l0 + (l1 - l0) * t, a0 + (a1 - a0) * t, b0 + (b1 - b0) * t);
        c.a = alpha;
        c
    }
}

impl std::ops::Add for Color {
    type Output = Self;
    #[inline]
    fn add(self, rhs: Self) -> Self {
        Self::new(
            self.r + rhs.r,
            self.g + rhs.g,
            self.b + rhs.b,
            self.a + rhs.a,
        )
    }
}

impl std::ops::Mul<f32> for Color {
    type Output = Self;
    #[inline]
    fn mul(self, s: f32) -> Self {
        Self::new(self.r * s, self.g * s, self.b * s, self.a * s)
    }
}

impl std::ops::Mul<Color> for f32 {
    type Output = Color;
    #[inline]
    fn mul(self, c: Color) -> Color {
        c * self
    }
}

// ── Internal gamma helpers ────────────────────────────────────────────────────

/// Convert sRGB encoded value (0–1) to linear light.
///
/// Uses the precise IEC 61966-2-1 piecewise formula.
#[inline]
fn srgb_to_linear(c: f32) -> f32 {
    if c <= 0.040_45 {
        c * (1.0 / 12.92)
    } else {
        ((c + 0.055) * (1.0 / 1.055)).powf(2.4)
    }
}

/// Convert linear light value (0–1) to sRGB encoding.
#[inline]
fn linear_to_srgb(c: f32) -> f32 {
    if c <= 0.003_130_8 {
        c * 12.92
    } else {
        1.055 * c.powf(1.0 / 2.4) - 0.055
    }
}

/// HSL hue-to-RGB helper.
#[inline]
fn hue_to_rgb(p: f32, q: f32, mut t: f32) -> f32 {
    if t < 0.0 {
        t += 1.0;
    }
    if t > 1.0 {
        t -= 1.0;
    }
    if t < 1.0 / 6.0 {
        return p + (q - p) * 6.0 * t;
    }
    if t < 1.0 / 2.0 {
        return q;
    }
    if t < 2.0 / 3.0 {
        return p + (q - p) * (2.0 / 3.0 - t) * 6.0;
    }
    p
}

#[cfg(test)]
mod tests {
    use super::*;

    const TOL: f32 = 0.001;

    #[test]
    fn srgb_midgrey_encodes_correctly() {
        // #808080 ≈ 0.2158 linear
        let c = Color::from_srgb_u8(128, 128, 128, 255);
        assert!((c.r - 0.2158).abs() < TOL, "r={}", c.r);
        assert!((c.g - c.r).abs() < 1e-6);
    }

    #[test]
    fn roundtrip_argb_u32() {
        let original: u32 = 0xFF_88_44_22;
        let c = Color::from_argb_u32(original);
        let back = c.to_argb_u32();
        // Allow ±1 per channel due to rounding
        for i in 0..4 {
            let shift = i * 8;
            let a = (original >> shift) & 0xFF;
            let b = (back >> shift) & 0xFF;
            assert!(a.abs_diff(b) <= 1, "channel {i}: {a} vs {b}");
        }
    }

    #[test]
    fn black_and_white_round_trip() {
        assert_eq!(Color::BLACK.to_argb_u32(), 0xFF_00_00_00);
        assert_eq!(Color::WHITE.to_argb_u32(), 0xFF_FF_FF_FF);
    }

    #[test]
    fn hsv_red_roundtrip() {
        let c = Color::from_hsv(0.0, 1.0, 1.0);
        assert!((c.r - 1.0).abs() < TOL);
        assert!(c.g.abs() < TOL);
        assert!(c.b.abs() < TOL);
        let (h, s, v) = Color::RED.to_hsv();
        assert!(h.abs() < TOL || (h - 360.0).abs() < TOL);
        assert!((s - 1.0).abs() < TOL);
        assert!((v - 1.0).abs() < TOL);
    }

    #[test]
    fn hsv_cyan_roundtrip() {
        let c = Color::from_hsv(180.0, 1.0, 1.0);
        assert!(c.r.abs() < TOL);
        assert!((c.g - 1.0).abs() < TOL);
        assert!((c.b - 1.0).abs() < TOL);
        let (h, s, v) = c.to_hsv();
        assert!((h - 180.0).abs() < TOL);
        assert!((s - 1.0).abs() < TOL);
        assert!((v - 1.0).abs() < TOL);
    }

    #[test]
    fn hsl_roundtrip() {
        let c = Color::from_hsl(240.0, 1.0, 0.5); // Pure blue in HSL
        assert!(c.r.abs() < TOL, "r={}", c.r);
        assert!(c.g.abs() < TOL, "g={}", c.g);
        assert!((c.b - 1.0).abs() < TOL, "b={}", c.b);
        let (h, s, l) = c.to_hsl();
        assert!((h - 240.0).abs() < TOL);
        assert!((s - 1.0).abs() < TOL);
        assert!((l - 0.5).abs() < TOL);
    }

    #[test]
    fn lerp_midpoint() {
        let c = Color::BLACK.lerp(Color::WHITE, 0.5);
        assert!((c.r - 0.5).abs() < TOL);
        assert!((c.a - 1.0).abs() < TOL);
    }

    #[test]
    fn blend_over_opaque_src_covers_dst() {
        let blended = Color::blend_over(Color::RED, Color::BLUE);
        assert!((blended.r - 1.0).abs() < TOL);
        assert!(blended.b.abs() < TOL);
    }

    #[test]
    fn blend_over_transparent_src_is_dst() {
        let src = Color::RED.with_alpha(0.0);
        let blended = Color::blend_over(src, Color::BLUE);
        assert!(blended.r.abs() < TOL);
        assert!((blended.b - 1.0).abs() < TOL);
    }

    #[test]
    fn blend_over_half_alpha() {
        let src = Color::new(1.0, 0.0, 0.0, 0.5);
        let dst = Color::new(0.0, 0.0, 1.0, 1.0);
        let out = Color::blend_over(src, dst);
        assert!((out.r - 0.5).abs() < TOL, "r={}", out.r);
        assert!((out.b - 0.5).abs() < TOL, "b={}", out.b);
    }

    #[test]
    fn luminance_white_is_one() {
        assert!((Color::WHITE.luminance() - 1.0).abs() < TOL);
        assert!(Color::BLACK.luminance().abs() < TOL);
    }

    #[test]
    fn premultiply_scales_rgb() {
        let c = Color::new(1.0, 0.5, 0.0, 0.5).premultiply();
        assert!((c.r - 0.5).abs() < TOL);
        assert!((c.g - 0.25).abs() < TOL);
        assert!((c.a - 0.5).abs() < TOL);
    }

    #[test]
    fn screen_blend_brightens() {
        let grey = Color::grey(0.5);
        let result = grey.blend_screen(grey);
        // 1 - (1-0.5)^2 = 0.75
        assert!((result.r - 0.75).abs() < TOL, "r={}", result.r);
    }

    #[test]
    fn greyscale_matches_luminance() {
        let c = Color::rgb(0.4, 0.3, 0.2);
        let grey = c.to_greyscale();
        let lum = c.luminance();
        assert!((grey.r - lum).abs() < TOL);
        assert!((grey.g - lum).abs() < TOL);
        assert!((grey.b - lum).abs() < TOL);
    }
}
