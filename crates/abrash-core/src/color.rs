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

    /// Overlay blend: multiply for darks, screen for lights.
    ///
    /// Uses `self` as the base layer and `other` as the blend layer.
    /// Where `self < 0.5`, darkens; where `self >= 0.5`, lightens.
    ///
    /// # Examples
    ///
    /// ```
    /// use abrash_core::color::Color;
    ///
    /// // Overlaying grey over grey should return grey
    /// let grey = Color::grey(0.5);
    /// let out = grey.blend_overlay(grey);
    /// assert!((out.r - 0.5).abs() < 1e-4, "r={}", out.r);
    /// ```
    #[must_use]
    #[inline]
    pub fn blend_overlay(self, other: Self) -> Self {
        let overlay = |base: f32, blend: f32| -> f32 {
            if base < 0.5 {
                2.0 * base * blend
            } else {
                1.0 - 2.0 * (1.0 - base) * (1.0 - blend)
            }
        };
        Self {
            r: overlay(self.r, other.r),
            g: overlay(self.g, other.g),
            b: overlay(self.b, other.b),
            a: self.a,
        }
    }

    /// Hard light blend: like overlay but with blend/base roles swapped.
    ///
    /// # Examples
    ///
    /// ```
    /// use abrash_core::color::Color;
    ///
    /// // Hard light of grey over grey equals grey
    /// let grey = Color::grey(0.5);
    /// let out = grey.blend_hard_light(grey);
    /// assert!((out.r - 0.5).abs() < 1e-4, "r={}", out.r);
    /// ```
    #[must_use]
    #[inline]
    pub fn blend_hard_light(self, other: Self) -> Self {
        // Hard light = overlay with base and blend swapped
        other.blend_overlay(self)
    }

    /// Soft light blend: gentle dodge/burn based on blend layer.
    ///
    /// Uses the W3C / Photoshop soft-light formula.  Results in a softer
    /// contrast adjustment than hard light.
    ///
    /// # Examples
    ///
    /// ```
    /// use abrash_core::color::Color;
    ///
    /// // Soft light of grey over grey equals grey
    /// let grey = Color::grey(0.5);
    /// let out = grey.blend_soft_light(grey);
    /// assert!((out.r - 0.5).abs() < 0.01, "r={}", out.r);
    /// ```
    #[must_use]
    pub fn blend_soft_light(self, other: Self) -> Self {
        let soft = |base: f32, blend: f32| -> f32 {
            if blend <= 0.5 {
                base - (1.0 - 2.0 * blend) * base * (1.0 - base)
            } else {
                let d = if base <= 0.25 {
                    ((16.0 * base - 12.0) * base + 4.0) * base
                } else {
                    base.sqrt()
                };
                base + (2.0 * blend - 1.0) * (d - base)
            }
        };
        Self {
            r: soft(self.r, other.r),
            g: soft(self.g, other.g),
            b: soft(self.b, other.b),
            a: self.a,
        }
    }

    /// Adjust brightness by adding `amount` to each RGB channel.
    ///
    /// Positive values brighten, negative values darken.  Result is clamped to
    /// `[0, 1]`.
    ///
    /// # Examples
    ///
    /// ```
    /// use abrash_core::color::Color;
    ///
    /// let c = Color::grey(0.5).adjust_brightness(0.2);
    /// assert!((c.r - 0.7).abs() < 1e-5);
    /// ```
    #[must_use]
    #[inline]
    pub fn adjust_brightness(self, amount: f32) -> Self {
        Self {
            r: (self.r + amount).clamp(0.0, 1.0),
            g: (self.g + amount).clamp(0.0, 1.0),
            b: (self.b + amount).clamp(0.0, 1.0),
            a: self.a,
        }
    }

    /// Adjust contrast around the midpoint (0.5).
    ///
    /// `factor > 1` increases contrast (pushes towards extremes); `factor < 1`
    /// decreases contrast (pushes towards grey).  `factor = 0` produces flat
    /// 50% grey.
    ///
    /// # Examples
    ///
    /// ```
    /// use abrash_core::color::Color;
    ///
    /// // Grey is unaffected by contrast adjustment
    /// let grey = Color::grey(0.5);
    /// let c = grey.adjust_contrast(2.0);
    /// assert!((c.r - 0.5).abs() < 1e-5);
    ///
    /// // Dark colour gets darker with more contrast
    /// let dark = Color::grey(0.3);
    /// let boosted = dark.adjust_contrast(2.0);
    /// assert!(boosted.r < dark.r);
    /// ```
    #[must_use]
    #[inline]
    pub fn adjust_contrast(self, factor: f32) -> Self {
        let adj = |v: f32| ((v - 0.5) * factor + 0.5).clamp(0.0, 1.0);
        Self {
            r: adj(self.r),
            g: adj(self.g),
            b: adj(self.b),
            a: self.a,
        }
    }

    /// Adjust HSV saturation by multiplying the S component.
    ///
    /// `factor = 0` produces greyscale; `factor = 1` is unchanged; `factor > 1`
    /// boosts saturation (clamped to `[0, 1]`).
    ///
    /// # Examples
    ///
    /// ```
    /// use abrash_core::color::Color;
    ///
    /// // Desaturate to zero → greyscale
    /// let red = Color::RED;
    /// let grey = red.adjust_saturation(0.0);
    /// assert!((grey.r - grey.g).abs() < 0.01, "should be grey");
    ///
    /// // Saturation of 1 is a no-op
    /// let same = red.adjust_saturation(1.0);
    /// assert!((same.r - red.r).abs() < 0.01);
    /// ```
    #[must_use]
    pub fn adjust_saturation(self, factor: f32) -> Self {
        let (h, s, v) = self.to_hsv();
        Self::from_hsv(h, (s * factor).clamp(0.0, 1.0), v).with_alpha(self.a)
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

    /// Approximate blackbody radiation color for a given color temperature in Kelvin.
    ///
    /// Uses Tanner Helland's curve-fit approximation (2012), valid for 1000K–40000K.
    /// Returns a fully-opaque linear-light color.
    ///
    /// Typical values:
    /// - 1850K — candle flame (deep orange)
    /// - 3200K — tungsten bulb (warm white)
    /// - 5500K — midday sunlight (neutral white)
    /// - 6500K — overcast sky (cool white)
    /// - 9000K — blue sky
    ///
    /// # Examples
    ///
    /// ```
    /// use abrash_core::color::Color;
    ///
    /// let candle = Color::from_temperature(1850.0);
    /// // Candle light is orange-red: r > g > b
    /// assert!(candle.r > candle.g && candle.g > candle.b);
    ///
    /// let daylight = Color::from_temperature(6500.0);
    /// // Daylight is roughly white
    /// assert!((daylight.r - daylight.g).abs() < 0.15);
    /// ```
    #[must_use]
    pub fn from_temperature(kelvin: f32) -> Self {
        let t = kelvin.clamp(1000.0, 40_000.0) / 100.0;

        let r = if t <= 66.0 {
            1.0_f32
        } else {
            let x = t - 60.0;
            (329.698_727_44 * x.powf(-0.133_204_759_2) / 255.0).clamp(0.0, 1.0)
        };

        let g = if t <= 66.0 {
            (99.470_802_59 * t.ln() - 161.119_568_26) / 255.0
        } else {
            let x = t - 60.0;
            (288.122_169_52 * x.powf(-0.075_514_849_2) / 255.0)
        }
        .clamp(0.0, 1.0);

        let b = if t >= 66.0 {
            1.0_f32
        } else if t <= 19.0 {
            0.0
        } else {
            let x = t - 10.0;
            ((138.517_731_2 * x.ln() - 305.044_792_6) / 255.0).clamp(0.0, 1.0)
        };

        // Convert from sRGB (the approximation is in perceptual space) to linear
        let srgb_to_lin = |v: f32| -> f32 {
            if v <= 0.040_45 {
                v / 12.92
            } else {
                ((v + 0.055) / 1.055).powf(2.4)
            }
        };
        Self::new(srgb_to_lin(r), srgb_to_lin(g), srgb_to_lin(b), 1.0)
    }

    /// Sample a multi-stop color gradient at `t ∈ [0, 1]`.
    ///
    /// `stops` is a slice of `(position, color)` pairs sorted in ascending
    /// `position` order.  The first stop is clamped at `t = 0`, the last at
    /// `t = 1`.
    ///
    /// Colors are interpolated in **linear** light space (same as [`Color::lerp`]).
    /// For perceptually smooth gradients use Oklab; for that, lerp the stops
    /// with [`Color::lerp_oklab`] manually.
    ///
    /// Returns `Color::BLACK` if `stops` is empty.
    ///
    /// # Examples
    ///
    /// ```
    /// use abrash_core::color::Color;
    ///
    /// let stops = [
    ///     (0.0, Color::BLACK),
    ///     (0.5, Color::rgb(1.0, 0.0, 0.0)),
    ///     (1.0, Color::WHITE),
    /// ];
    /// let mid = Color::gradient(0.5, &stops);
    /// assert!((mid.r - 1.0).abs() < 1e-5, "should be red at midpoint");
    ///
    /// let quarter = Color::gradient(0.25, &stops);
    /// assert!(quarter.r > 0.0 && quarter.r < 1.0);
    /// ```
    #[must_use]
    pub fn gradient(t: f32, stops: &[(f32, Self)]) -> Self {
        if stops.is_empty() {
            return Self::BLACK;
        }
        if stops.len() == 1 {
            return stops[0].1;
        }
        let t = t.clamp(stops[0].0, stops[stops.len() - 1].0);
        // Find the segment
        for i in 0..stops.len() - 1 {
            let (t0, c0) = stops[i];
            let (t1, c1) = stops[i + 1];
            if t <= t1 {
                let span = t1 - t0;
                if span < 1e-8 {
                    return c1;
                }
                return c0.lerp(c1, (t - t0) / span);
            }
        }
        stops[stops.len() - 1].1
    }

    /// Color dodge: brightens `self` by the inverse of `other`.
    ///
    /// Result = clamp(self / (1 − other)).  Produces values brighter than
    /// either input — the photographic "dodge" operation.
    ///
    /// # Examples
    ///
    /// ```
    /// use abrash_core::color::Color;
    ///
    /// let grey = Color::rgb(0.5, 0.5, 0.5);
    /// let dark = Color::rgb(0.25, 0.25, 0.25);
    /// let out = grey.blend_dodge(dark);
    /// // dodge always brightens — result > grey
    /// assert!(out.r > grey.r);
    /// ```
    #[must_use]
    pub fn blend_dodge(self, other: Self) -> Self {
        let f = |a: f32, b: f32| -> f32 {
            if b >= 1.0 {
                1.0
            } else {
                (a / (1.0 - b)).clamp(0.0, 1.0)
            }
        };
        Self::new(
            f(self.r, other.r),
            f(self.g, other.g),
            f(self.b, other.b),
            self.a,
        )
    }

    /// Color burn: darkens `self` toward `other`.
    ///
    /// Result = 1 − clamp((1 − self) / other).  The complement of dodge.
    ///
    /// # Examples
    ///
    /// ```
    /// use abrash_core::color::Color;
    ///
    /// let grey = Color::rgb(0.5, 0.5, 0.5);
    /// let light = Color::rgb(0.75, 0.75, 0.75);
    /// let out = grey.blend_burn(light);
    /// // burn always darkens — result < grey
    /// assert!(out.r < grey.r);
    /// ```
    #[must_use]
    pub fn blend_burn(self, other: Self) -> Self {
        let f = |a: f32, b: f32| -> f32 {
            if b <= 0.0 {
                0.0
            } else {
                (1.0 - (1.0 - a) / b).clamp(0.0, 1.0)
            }
        };
        Self::new(
            f(self.r, other.r),
            f(self.g, other.g),
            f(self.b, other.b),
            self.a,
        )
    }

    /// Difference blend: absolute difference of each channel.
    ///
    /// Identical pixels produce black; complementary pairs produce white.
    /// Useful for comparing two layers or creating psychedelic effects.
    ///
    /// # Examples
    ///
    /// ```
    /// use abrash_core::color::Color;
    ///
    /// let a = Color::rgb(0.8, 0.3, 0.5);
    /// let b = Color::rgb(0.3, 0.3, 0.5);
    /// let out = a.blend_difference(b);
    /// assert!((out.r - 0.5).abs() < 1e-5);
    /// assert!(out.g < 1e-5); // identical channels → 0
    /// ```
    #[must_use]
    pub fn blend_difference(self, other: Self) -> Self {
        Self::new(
            (self.r - other.r).abs(),
            (self.g - other.g).abs(),
            (self.b - other.b).abs(),
            self.a,
        )
    }

    /// Exclusion blend: softer version of difference, avoids extreme contrast.
    ///
    /// Formula: `a + b − 2·a·b`.  Identical pixels produce mid-grey (0.5),
    /// not black.
    ///
    /// # Examples
    ///
    /// ```
    /// use abrash_core::color::Color;
    ///
    /// let white = Color::WHITE;
    /// let black = Color::BLACK;
    /// // white exclusion black = white (max contrast)
    /// assert!((white.blend_exclusion(black).r - 1.0).abs() < 1e-5);
    /// // identical mid-grey → 0.5
    /// let grey = Color::rgb(0.5, 0.5, 0.5);
    /// assert!((grey.blend_exclusion(grey).r - 0.5).abs() < 1e-5);
    /// ```
    #[must_use]
    pub fn blend_exclusion(self, other: Self) -> Self {
        let f = |a: f32, b: f32| a + b - 2.0 * a * b;
        Self::new(
            f(self.r, other.r),
            f(self.g, other.g),
            f(self.b, other.b),
            self.a,
        )
    }

    /// Convert from linear RGB to CIE XYZ (D65 illuminant).
    ///
    /// The alpha channel is preserved unchanged.  Uses the IEC 61966-2-1
    /// sRGB primaries matrix.
    ///
    /// # Examples
    ///
    /// ```
    /// use abrash_core::color::Color;
    ///
    /// let (x, y, z) = Color::WHITE.to_xyz();
    /// // D65 white point: (0.9505, 1.0, 1.089)
    /// assert!((x - 0.9505).abs() < 0.002);
    /// assert!((y - 1.0).abs() < 0.002);
    /// assert!((z - 1.0890).abs() < 0.002);
    /// ```
    #[must_use]
    pub fn to_xyz(self) -> (f32, f32, f32) {
        let r = self.r;
        let g = self.g;
        let b = self.b;
        let x = 0.412_456_4 * r + 0.357_576_1 * g + 0.180_437_5 * b;
        let y = 0.212_672_9 * r + 0.715_152_2 * g + 0.072_174_9 * b;
        let z = 0.019_333_9 * r + 0.119_192_0 * g + 0.950_304_1 * b;
        (x, y, z)
    }

    /// Construct a [`Color`] from CIE XYZ (D65 illuminant).
    ///
    /// The result is in linear light space.  Out-of-gamut values are clamped.
    ///
    /// # Examples
    ///
    /// ```
    /// use abrash_core::color::Color;
    ///
    /// let c = Color::from_xyz(0.9505, 1.0, 1.0890);
    /// assert!((c.r - 1.0).abs() < 0.01);
    /// assert!((c.g - 1.0).abs() < 0.01);
    /// assert!((c.b - 1.0).abs() < 0.01);
    /// ```
    #[must_use]
    pub fn from_xyz(x: f32, y: f32, z: f32) -> Self {
        let r = 3.240_454_2 * x - 1.537_138_5 * y - 0.498_531_4 * z;
        let g = -0.969_266_0 * x + 1.876_010_8 * y + 0.041_556_0 * z;
        let b = 0.055_643_4 * x - 0.204_025_9 * y + 1.057_225_2 * z;
        Self::new(r.clamp(0.0, 1.0), g.clamp(0.0, 1.0), b.clamp(0.0, 1.0), 1.0)
    }

    /// Convert to Oklch (L, Chroma, Hue) — the cylindrical form of Oklab.
    ///
    /// - `L` ∈ [0, 1]: perceptual lightness
    /// - `C` ≥ 0: chroma (saturation magnitude)
    /// - `H` ∈ [0, 2π]: hue angle in radians
    ///
    /// # Examples
    ///
    /// ```
    /// use abrash_core::color::Color;
    ///
    /// let grey = Color::rgb(0.5, 0.5, 0.5);
    /// let (l, c, _h) = grey.to_oklch();
    /// assert!(c < 0.01, "grey has near-zero chroma: {c}");
    /// assert!(l > 0.0 && l < 1.0);
    /// ```
    #[must_use]
    pub fn to_oklch(self) -> (f32, f32, f32) {
        let (l, a, b) = self.to_oklab();
        let c = (a * a + b * b).sqrt();
        let h = b.atan2(a).rem_euclid(std::f32::consts::TAU);
        (l, c, h)
    }

    /// Construct a [`Color`] from Oklch (L, Chroma, Hue).
    ///
    /// `H` is in radians.  Converts via Oklab → linear RGB.
    ///
    /// # Examples
    ///
    /// ```
    /// use abrash_core::color::Color;
    ///
    /// // Round-trip: linear red → Oklch → back
    /// let red = Color::rgb(1.0, 0.0, 0.0);
    /// let (l, c, h) = red.to_oklch();
    /// let back = Color::from_oklch(l, c, h);
    /// assert!((back.r - red.r).abs() < 1e-4);
    /// assert!((back.g - red.g).abs() < 1e-4);
    /// assert!((back.b - red.b).abs() < 1e-4);
    /// ```
    #[must_use]
    pub fn from_oklch(l: f32, c: f32, h: f32) -> Self {
        let a = c * h.cos();
        let b = c * h.sin();
        Self::from_oklab(l, a, b)
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

    #[test]
    fn temperature_candle_is_warm() {
        // ~1850K candle — strong red, weak blue
        let c = Color::from_temperature(1850.0);
        assert!(
            c.r > c.b,
            "candle should be red-biased, r={} b={}",
            c.r,
            c.b
        );
        assert!(c.a == 1.0);
    }

    #[test]
    fn temperature_daylight_is_neutral() {
        // ~6500K daylight — roughly balanced
        let c = Color::from_temperature(6500.0);
        assert!(c.r > 0.0 && c.g > 0.0 && c.b > 0.0, "should be non-zero");
        assert!(c.a == 1.0);
    }

    #[test]
    fn temperature_clamps_extremes() {
        let low = Color::from_temperature(0.0);
        let high = Color::from_temperature(100_000.0);
        // Both should equal clamped-range results, not panic
        assert_eq!(low, Color::from_temperature(1000.0));
        assert_eq!(high, Color::from_temperature(40_000.0));
    }

    #[test]
    fn gradient_endpoints() {
        let stops = [(0.0, Color::BLACK), (1.0, Color::WHITE)];
        let start = Color::gradient(0.0, &stops);
        let end = Color::gradient(1.0, &stops);
        assert!((start.r).abs() < TOL);
        assert!((end.r - 1.0).abs() < TOL);
    }

    #[test]
    fn gradient_midpoint() {
        let stops = [(0.0, Color::BLACK), (1.0, Color::WHITE)];
        let mid = Color::gradient(0.5, &stops);
        assert!((mid.r - 0.5).abs() < TOL, "r={}", mid.r);
    }

    #[test]
    fn gradient_multi_stop() {
        let stops = [(0.0, Color::BLACK), (0.5, Color::RED), (1.0, Color::WHITE)];
        // At 0.25 we're halfway between BLACK and RED
        let c = Color::gradient(0.25, &stops);
        assert!((c.r - 0.5).abs() < TOL, "r={}", c.r);
        assert!(c.g.abs() < TOL);
    }

    #[test]
    fn gradient_empty_returns_black() {
        let c = Color::gradient(0.5, &[]);
        assert_eq!(c, Color::BLACK);
    }

    #[test]
    fn gradient_clamps_out_of_range() {
        let stops = [(0.2, Color::BLACK), (0.8, Color::WHITE)];
        let under = Color::gradient(0.0, &stops);
        let over = Color::gradient(1.0, &stops);
        assert!((under.r).abs() < TOL);
        assert!((over.r - 1.0).abs() < TOL);
    }

    // ── Blend modes ──────────────────────────────────────────────────────────

    #[test]
    fn overlay_grey_is_grey() {
        let grey = Color::grey(0.5);
        let out = grey.blend_overlay(grey);
        assert!((out.r - 0.5).abs() < 0.01, "r={}", out.r);
    }

    #[test]
    fn overlay_white_stays_white() {
        let out = Color::WHITE.blend_overlay(Color::WHITE);
        assert!((out.r - 1.0).abs() < TOL);
    }

    #[test]
    fn overlay_black_stays_black() {
        let out = Color::BLACK.blend_overlay(Color::BLACK);
        assert!(out.r.abs() < TOL);
    }

    #[test]
    fn hard_light_is_swapped_overlay() {
        let a = Color::rgb(0.3, 0.5, 0.7);
        let b = Color::rgb(0.6, 0.4, 0.2);
        let hl = a.blend_hard_light(b);
        let ol = b.blend_overlay(a);
        assert!((hl.r - ol.r).abs() < TOL, "r: hl={} ol={}", hl.r, ol.r);
    }

    #[test]
    fn soft_light_grey_is_grey() {
        let grey = Color::grey(0.5);
        let out = grey.blend_soft_light(grey);
        assert!((out.r - 0.5).abs() < 0.01, "r={}", out.r);
    }

    // ── Adjustments ─────────────────────────────────────────────────────────

    #[test]
    fn brightness_up() {
        let c = Color::grey(0.4).adjust_brightness(0.2);
        assert!((c.r - 0.6).abs() < TOL);
    }

    #[test]
    fn brightness_clamps() {
        let c = Color::WHITE.adjust_brightness(0.5);
        assert!((c.r - 1.0).abs() < TOL);
        let c2 = Color::BLACK.adjust_brightness(-0.5);
        assert!(c2.r.abs() < TOL);
    }

    #[test]
    fn contrast_midpoint_unchanged() {
        let grey = Color::grey(0.5);
        let c = grey.adjust_contrast(3.0);
        assert!((c.r - 0.5).abs() < TOL);
    }

    #[test]
    fn contrast_increases_spread() {
        let dark = Color::grey(0.3);
        let boosted = dark.adjust_contrast(2.0);
        assert!(
            boosted.r < dark.r,
            "contrast boost should darken r={}",
            boosted.r
        );
    }

    #[test]
    fn saturation_zero_is_greyscale() {
        let red = Color::RED;
        let grey = red.adjust_saturation(0.0);
        assert!((grey.r - grey.g).abs() < 0.01);
        assert!((grey.r - grey.b).abs() < 0.01);
    }

    #[test]
    fn saturation_one_is_noop() {
        let blue = Color::BLUE;
        let same = blue.adjust_saturation(1.0);
        assert!((same.r - blue.r).abs() < 0.01);
        assert!((same.b - blue.b).abs() < 0.01);
    }

    // ── blend_dodge / blend_burn ─────────────────────────────────────────────

    #[test]
    fn dodge_brightens() {
        let base = Color::rgb(0.4, 0.4, 0.4);
        let factor = Color::rgb(0.5, 0.5, 0.5);
        let out = base.blend_dodge(factor);
        assert!(out.r > base.r, "dodge must brighten");
    }

    #[test]
    fn burn_darkens() {
        let base = Color::rgb(0.6, 0.6, 0.6);
        let factor = Color::rgb(0.5, 0.5, 0.5);
        let out = base.blend_burn(factor);
        assert!(out.r < base.r, "burn must darken");
    }

    #[test]
    fn dodge_full_divisor_clamps() {
        let out = Color::rgb(0.5, 0.5, 0.5).blend_dodge(Color::WHITE);
        assert!((out.r - 1.0).abs() < 1e-5);
    }

    #[test]
    fn burn_zero_divisor_clamps() {
        let out = Color::rgb(0.5, 0.5, 0.5).blend_burn(Color::BLACK);
        assert!(out.r < 1e-5);
    }

    // ── blend_difference / blend_exclusion ───────────────────────────────────

    #[test]
    fn difference_identical_is_black() {
        let c = Color::rgb(0.7, 0.3, 0.5);
        let out = c.blend_difference(c);
        assert!(out.r < 1e-5);
        assert!(out.g < 1e-5);
        assert!(out.b < 1e-5);
    }

    #[test]
    fn difference_is_commutative() {
        let a = Color::rgb(0.8, 0.2, 0.6);
        let b = Color::rgb(0.3, 0.7, 0.1);
        let ab = a.blend_difference(b);
        let ba = b.blend_difference(a);
        assert!((ab.r - ba.r).abs() < 1e-5);
        assert!((ab.g - ba.g).abs() < 1e-5);
    }

    #[test]
    fn exclusion_grey_is_midgrey() {
        let grey = Color::rgb(0.5, 0.5, 0.5);
        let out = grey.blend_exclusion(grey);
        assert!(
            (out.r - 0.5).abs() < 1e-5,
            "exclusion of grey with itself = 0.5"
        );
    }

    // ── to_xyz / from_xyz ────────────────────────────────────────────────────

    #[test]
    fn xyz_white_point_d65() {
        let (x, y, z) = Color::WHITE.to_xyz();
        assert!((x - 0.9505).abs() < 0.003, "x={x}");
        assert!((y - 1.0).abs() < 0.003, "y={y}");
        assert!((z - 1.0890).abs() < 0.003, "z={z}");
    }

    #[test]
    fn xyz_roundtrip() {
        let c = Color::rgb(0.8, 0.3, 0.5);
        let (x, y, z) = c.to_xyz();
        let back = Color::from_xyz(x, y, z);
        assert!((back.r - c.r).abs() < 1e-4, "r");
        assert!((back.g - c.g).abs() < 1e-4, "g");
        assert!((back.b - c.b).abs() < 1e-4, "b");
    }

    #[test]
    fn xyz_black_is_zero() {
        let (x, y, z) = Color::BLACK.to_xyz();
        assert!(x < 1e-5 && y < 1e-5 && z < 1e-5);
    }

    // ── to_oklch / from_oklch ────────────────────────────────────────────────

    #[test]
    fn oklch_grey_zero_chroma() {
        let grey = Color::rgb(0.5, 0.5, 0.5);
        let (_l, c, _h) = grey.to_oklch();
        assert!(c < 0.01, "grey chroma should be near zero, got {c}");
    }

    #[test]
    fn oklch_roundtrip() {
        let col = Color::rgb(0.9, 0.2, 0.4);
        let (l, c, h) = col.to_oklch();
        let back = Color::from_oklch(l, c, h);
        assert!((back.r - col.r).abs() < 1e-4, "r");
        assert!((back.g - col.g).abs() < 1e-4, "g");
        assert!((back.b - col.b).abs() < 1e-4, "b");
    }

    #[test]
    fn oklch_hue_in_range() {
        use std::f32::consts::TAU;
        for col in [Color::RED, Color::GREEN, Color::BLUE] {
            let (_l, _c, h) = col.to_oklch();
            assert!(h >= 0.0 && h <= TAU, "hue out of [0, 2π]: {h}");
        }
    }
}
