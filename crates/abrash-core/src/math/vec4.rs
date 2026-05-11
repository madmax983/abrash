#[allow(clippy::wildcard_imports)]
use super::*;
use std::mem::MaybeUninit;
use std::ops::{Add, Mul, Sub};

/// A 4-component vector, often used for homogeneous coordinates or tangents.
///
/// In the rasterization pipeline, `Vec4` is used for:
/// *   Homogeneous coordinates (x, y, z, w) where w is the perspective term.
/// *   Tangent vectors in Normal Mapping, where w stores the handedness of the tangent basis.
///
/// # Examples
///
/// ```
/// use abrash_core::math::Vec4;
///
/// let v = Vec4::new(1.0, 2.0, 3.0, 1.0);
/// assert_eq!(v.x, 1.0);
/// ```
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Vec4 {
    /// The X component.
    pub x: f32,
    /// The Y component.
    pub y: f32,
    /// The Z component.
    pub z: f32,
    /// The W component.
    pub w: f32,
}

impl Vec4 {
    /// A vector with all components set to zero.
    pub const ZERO: Self = Self {
        x: 0.0,
        y: 0.0,
        z: 0.0,
        w: 0.0,
    };
    /// A vector with all components set to one.
    pub const ONE: Self = Self {
        x: 1.0,
        y: 1.0,
        z: 1.0,
        w: 1.0,
    };

    /// Creates a vector with all components set to `v`.
    #[must_use]
    #[inline]
    pub const fn splat(v: f32) -> Self {
        Self {
            x: v,
            y: v,
            z: v,
            w: v,
        }
    }

    /// Returns the xyz components as a `Vec3`.
    #[must_use]
    #[inline]
    pub const fn xyz(self) -> Vec3 {
        Vec3::new(self.x, self.y, self.z)
    }

    /// Linearly interpolate between this vector and another.
    ///
    /// The `t` factor dictates the blend: `0.0` returns `self`, `1.0` returns `other`.
    ///
    /// # Examples
    ///
    /// ```
    /// use abrash_core::math::Vec4;
    ///
    /// let start = Vec4::new(0.0, 0.0, 0.0, 0.0);
    /// let end = Vec4::new(10.0, 10.0, 10.0, 10.0);
    /// let mid = start.lerp(end, 0.5);
    ///
    /// assert_eq!(mid.x, 5.0);
    /// assert_eq!(mid.w, 5.0);
    /// ```
    #[must_use]
    #[inline(always)]
    pub fn lerp(self, other: Self, t: f32) -> Self {
        Self {
            x: self.x + (other.x - self.x) * t,
            y: self.y + (other.y - self.y) * t,
            z: self.z + (other.z - self.z) * t,
            w: self.w + (other.w - self.w) * t,
        }
    }

    #[must_use]
    #[inline(always)]

    /// Creates a new 4D vector.
    ///
    /// # Examples
    ///
    /// ```
    /// use abrash_core::math::Vec4;
    ///
    /// let v = Vec4::new(1.0, 2.0, 3.0, 1.0);
    /// assert_eq!(v.w, 1.0);
    /// ```

    pub const fn new(x: f32, y: f32, z: f32, w: f32) -> Self {
        Self { x, y, z, w }
    }

    /// Calculates dot product between two vectors.
    #[must_use]
    #[inline]
    pub fn dot(self, other: Self) -> f32 {
        self.x * other.x + self.y * other.y + self.z * other.z + self.w * other.w
    }

    /// Calculates squared length (magnitude²) of the vector.
    #[must_use]
    #[inline]
    pub fn length_sq(self) -> f32 {
        self.dot(self)
    }

    /// Calculates the Euclidean length (magnitude) of the vector.
    #[must_use]
    #[inline]
    pub fn length(self) -> f32 {
        self.length_sq().sqrt()
    }

    /// Returns a normalized unit vector (length of 1.0).
    ///
    /// For tiny vectors (length² <= `1e-8`), returns the original vector.
    #[must_use]
    #[inline]
    pub fn normalize(self) -> Self {
        let len_sq = self.length_sq();
        if len_sq > 0.000_000_01 {
            let inv_len = len_sq.sqrt().recip();
            Self {
                x: self.x * inv_len,
                y: self.y * inv_len,
                z: self.z * inv_len,
                w: self.w * inv_len,
            }
        } else {
            self
        }
    }

    /// Distance to another vector.
    #[must_use]
    #[inline]
    pub fn distance(self, other: Self) -> f32 {
        (self - other).length()
    }

    /// Squared distance to another vector.
    #[must_use]
    #[inline]
    pub fn distance_sq(self, other: Self) -> f32 {
        (self - other).length_sq()
    }

    /// Component-wise minimum.
    #[must_use]
    #[inline]
    pub const fn min(self, other: Self) -> Self {
        Self {
            x: self.x.min(other.x),
            y: self.y.min(other.y),
            z: self.z.min(other.z),
            w: self.w.min(other.w),
        }
    }

    /// Component-wise maximum.
    #[must_use]
    #[inline]
    pub const fn max(self, other: Self) -> Self {
        Self {
            x: self.x.max(other.x),
            y: self.y.max(other.y),
            z: self.z.max(other.z),
            w: self.w.max(other.w),
        }
    }

    /// Clamp each component between corresponding min/max components.
    #[must_use]
    #[inline]
    pub const fn clamp(self, min: Self, max: Self) -> Self {
        Self {
            x: self.x.clamp(min.x, max.x),
            y: self.y.clamp(min.y, max.y),
            z: self.z.clamp(min.z, max.z),
            w: self.w.clamp(min.w, max.w),
        }
    }

    /// Projects this vector onto another vector.
    ///
    /// Returns `Vec4::ZERO` when `onto` is near zero to avoid division by tiny values.
    #[must_use]
    #[inline]
    pub fn project_onto(self, onto: Self) -> Self {
        let denom = onto.length_sq();
        if denom <= 0.000_000_01 {
            return Self::ZERO;
        }
        onto * (self.dot(onto) / denom)
    }

    /// Reject this vector from another vector (component orthogonal to `onto`).
    #[must_use]
    #[inline]
    pub fn reject_from(self, onto: Self) -> Self {
        self - self.project_onto(onto)
    }

    /// Perspective divide: divide `x`, `y`, `z` by `w` and return as a [`Vec3`].
    ///
    /// Used after matrix-vector multiplication to convert homogeneous clip-space
    /// coordinates to NDC (Normalised Device Coordinates).  Returns `Vec3::ZERO`
    /// if `w` is zero.
    ///
    /// # Examples
    ///
    /// ```
    /// use abrash_core::math::Vec4;
    ///
    /// let h = Vec4::new(2.0, 4.0, 6.0, 2.0);
    /// let ndc = h.homogenize();
    /// assert!((ndc.x - 1.0).abs() < 1e-6);
    /// assert!((ndc.y - 2.0).abs() < 1e-6);
    /// assert!((ndc.z - 3.0).abs() < 1e-6);
    /// ```
    #[must_use]
    #[inline]
    pub fn homogenize(self) -> Vec3 {
        if self.w.abs() < 1e-10 {
            return Vec3::ZERO;
        }
        let inv_w = 1.0 / self.w;
        Vec3::new(self.x * inv_w, self.y * inv_w, self.z * inv_w)
    }

    /// Component-wise absolute value.
    #[must_use]
    #[inline]
    pub const fn abs(self) -> Self {
        Self {
            x: self.x.abs(),
            y: self.y.abs(),
            z: self.z.abs(),
            w: self.w.abs(),
        }
    }

    /// Component-wise sign: `−1.0`, `0.0`, or `+1.0`.
    #[must_use]
    #[inline]
    pub const fn sign(self) -> Self {
        Self {
            x: self.x.signum(),
            y: self.y.signum(),
            z: self.z.signum(),
            w: self.w.signum(),
        }
    }

    /// Component-wise floor (round toward negative infinity).
    #[must_use]
    #[inline]
    pub const fn floor(self) -> Self {
        Self {
            x: self.x.floor(),
            y: self.y.floor(),
            z: self.z.floor(),
            w: self.w.floor(),
        }
    }

    /// Component-wise ceiling (round toward positive infinity).
    #[must_use]
    #[inline]
    pub const fn ceil(self) -> Self {
        Self {
            x: self.x.ceil(),
            y: self.y.ceil(),
            z: self.z.ceil(),
            w: self.w.ceil(),
        }
    }

    /// Component-wise round (round to nearest, ties to even).
    #[must_use]
    #[inline]
    pub const fn round(self) -> Self {
        Self {
            x: self.x.round(),
            y: self.y.round(),
            z: self.z.round(),
            w: self.w.round(),
        }
    }

    /// Component-wise fractional part (`x - floor(x)`), matching GLSL semantics (result in [0, 1)).
    #[must_use]
    #[inline]
    pub fn fract(self) -> Self {
        Self {
            x: self.x - self.x.floor(),
            y: self.y - self.y.floor(),
            z: self.z - self.z.floor(),
            w: self.w - self.w.floor(),
        }
    }

    /// Smallest of the four components.
    #[must_use]
    #[inline]
    pub const fn min_component(self) -> f32 {
        self.x.min(self.y).min(self.z).min(self.w)
    }

    /// Largest of the four components.
    #[must_use]
    #[inline]
    pub const fn max_component(self) -> f32 {
        self.x.max(self.y).max(self.z).max(self.w)
    }

    /// Component-wise `x^exp`.
    #[must_use]
    #[inline]
    pub fn pow(self, exp: f32) -> Self {
        Self::new(
            self.x.powf(exp),
            self.y.powf(exp),
            self.z.powf(exp),
            self.w.powf(exp),
        )
    }

    /// Component-wise `e^x`.
    #[must_use]
    #[inline]
    pub fn exp(self) -> Self {
        Self::new(self.x.exp(), self.y.exp(), self.z.exp(), self.w.exp())
    }

    /// Component-wise natural log.
    #[must_use]
    #[inline]
    pub fn log(self) -> Self {
        Self::new(self.x.ln(), self.y.ln(), self.z.ln(), self.w.ln())
    }
}

/// Multiply vector by scalar.
impl std::ops::Mul<f32> for Vec4 {
    type Output = Self;
    #[inline]
    fn mul(self, scalar: f32) -> Self {
        Self {
            x: self.x * scalar,
            y: self.y * scalar,
            z: self.z * scalar,
            w: self.w * scalar,
        }
    }
}

/// Component-wise multiply.
impl std::ops::Mul for Vec4 {
    type Output = Self;
    #[inline]
    fn mul(self, other: Self) -> Self {
        Self {
            x: self.x * other.x,
            y: self.y * other.y,
            z: self.z * other.z,
            w: self.w * other.w,
        }
    }
}

/// Component-wise addition.
impl std::ops::Add for Vec4 {
    type Output = Self;
    #[inline]
    fn add(self, other: Self) -> Self {
        Self {
            x: self.x + other.x,
            y: self.y + other.y,
            z: self.z + other.z,
            w: self.w + other.w,
        }
    }
}

/// Component-wise subtraction.
impl std::ops::Sub for Vec4 {
    type Output = Self;
    #[inline]
    fn sub(self, other: Self) -> Self {
        Self {
            x: self.x - other.x,
            y: self.y - other.y,
            z: self.z - other.z,
            w: self.w - other.w,
        }
    }
}

impl std::ops::Div<f32> for Vec4 {
    type Output = Self;
    #[inline]
    fn div(self, scalar: f32) -> Self {
        let inv = 1.0 / scalar;
        Self {
            x: self.x * inv,
            y: self.y * inv,
            z: self.z * inv,
            w: self.w * inv,
        }
    }
}
