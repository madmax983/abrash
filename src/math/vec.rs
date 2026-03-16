#[cfg(target_arch = "x86_64")]
use std::arch::x86_64::*;
use std::ops::{Add, Div, Mul, Sub};
use super::fast_inv_sqrt;

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Vec2 {
    pub x: f32,
    pub y: f32,
}

impl Vec2 {
    #[must_use]
    #[inline(always)]
    pub fn lerp(self, other: Self, t: f32) -> Self {
        Self {
            x: self.x + (other.x - self.x) * t,
            y: self.y + (other.y - self.y) * t,
        }
    }

    /// Creates a new 2D vector.
    #[must_use]
    #[inline]
    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }

    /// Calculates the Euclidean length (magnitude) of the vector.
    #[must_use]
    #[inline]
    pub fn length(self) -> f32 {
        self.x.hypot(self.y)
    }
}

impl Add for Vec2 {
    type Output = Self;

    #[inline]
    fn add(self, other: Self) -> Self {
        Self {
            x: self.x + other.x,
            y: self.y + other.y,
        }
    }
}

impl Sub for Vec2 {
    type Output = Self;

    #[inline]
    fn sub(self, other: Self) -> Self {
        Self {
            x: self.x - other.x,
            y: self.y - other.y,
        }
    }
}

impl Mul<f32> for Vec2 {
    type Output = Self;

    #[inline]
    fn mul(self, scalar: f32) -> Self {
        Self {
            x: self.x * scalar,
            y: self.y * scalar,
        }
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Vec3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl Vec3 {
    /// Linearly interpolate between this vector and another.
    ///
    /// `t` is the interpolation factor (0.0 = self, 1.0 = other).
    #[must_use]
    #[inline(always)]
    pub fn lerp(self, other: Self, t: f32) -> Self {
        Self {
            x: self.x + (other.x - self.x) * t,
            y: self.y + (other.y - self.y) * t,
            z: self.z + (other.z - self.z) * t,
        }
    }

    pub const ZERO: Self = Self {
        x: 0.0,
        y: 0.0,
        z: 0.0,
    };
    pub const ONE: Self = Self {
        x: 1.0,
        y: 1.0,
        z: 1.0,
    };

    /// Creates a new vector.
    #[must_use]
    #[inline]
    pub const fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z }
    }

    /// Calculates the dot product with another vector.
    ///
    /// The dot product represents the projection of one vector onto another.
    /// *   Positive if pointing in similar direction.
    /// *   Zero if perpendicular.
    /// *   Negative if pointing in opposite directions.
    ///
    /// # Examples
    ///
    /// ```
    /// use abrash::math::Vec3;
    ///
    /// let a = Vec3::new(1.0, 0.0, 0.0);
    /// let b = Vec3::new(0.5, 0.0, 0.0);
    /// let c = Vec3::new(0.0, 1.0, 0.0);
    ///
    /// // Parallel vectors
    /// assert_eq!(a.dot(b), 0.5);
    ///
    /// // Perpendicular vectors
    /// assert_eq!(a.dot(c), 0.0);
    /// ```
    ///
    /// # Performance
    ///
    /// Passes `self` by value rather than reference to avoid pointer indirection
    /// and improve register allocation for small `Copy` types.
    #[must_use]
    #[inline]
    pub fn dot(self, other: Self) -> f32 {
        self.x * other.x + self.y * other.y + self.z * other.z
    }

    /// Calculates the cross product with another vector.
    ///
    /// Returns a vector perpendicular to both input vectors using the Right-Hand Rule.
    ///
    /// # Examples
    ///
    /// ```
    /// use abrash::math::Vec3;
    ///
    /// // X cross Y = Z (Right-Handed)
    /// let x = Vec3::new(1.0, 0.0, 0.0);
    /// let y = Vec3::new(0.0, 1.0, 0.0);
    /// let z = x.cross(y);
    ///
    /// assert_eq!(z, Vec3::new(0.0, 0.0, 1.0));
    /// ```
    ///
    /// # Performance
    ///
    /// Passes `self` by value rather than reference to avoid pointer indirection
    /// and improve register allocation for small `Copy` types.
    #[must_use]
    #[inline]
    pub fn cross(self, other: Self) -> Self {
        Self {
            x: self.y * other.z - self.z * other.y,
            y: self.z * other.x - self.x * other.z,
            z: self.x * other.y - self.y * other.x,
        }
    }

    /// Calculates the Euclidean length (magnitude) of the vector.
    ///
    /// # Performance
    ///
    /// Passes `self` by value rather than reference to avoid pointer indirection
    /// and improve register allocation for small `Copy` types.
    #[must_use]
    #[inline]
    pub fn length(self) -> f32 {
        (self.x * self.x + self.y * self.y + self.z * self.z).sqrt()
    }

    /// Returns a normalized unit vector (length of 1.0).
    ///
    /// # Behavior for Small Vectors
    ///
    /// If the vector's length is less than `0.0001`, this function returns
    /// the original vector unchanged to avoid division by zero or precision issues.
    ///
    /// # Examples
    ///
    /// ```
    /// use abrash::math::Vec3;
    ///
    /// let v = Vec3::new(0.0, 3.0, 4.0); // Length is 5
    /// let n = v.normalize();
    /// assert_eq!(n, Vec3::new(0.0, 0.6, 0.8));
    ///
    /// // Small vector behavior
    /// let tiny = Vec3::new(0.00001, 0.0, 0.0);
    /// assert_eq!(tiny.normalize(), tiny);
    /// ```
    ///
    /// # Performance
    ///
    /// Passes `self` by value rather than reference to avoid pointer indirection
    /// and improve register allocation for small `Copy` types.
    #[must_use]
    #[inline]
    pub fn normalize(self) -> Self {
        // Optimization: Use rsqrt instead of 1.0/sqrt.
        // We use len_sq to avoid sqrt if the vector is too small.
        // 0.0001^2 = 0.00000001
        let len_sq = self.x * self.x + self.y * self.y + self.z * self.z;
        if len_sq > 0.00000001 {
            let inv_len = fast_inv_sqrt(len_sq);
            Self {
                x: self.x * inv_len,
                y: self.y * inv_len,
                z: self.z * inv_len,
            }
        } else {
            self
        }
    }

    /// Returns a normalized unit vector using fast inverse square root approximation.
    ///
    /// This is faster than `normalize()` but slightly less accurate.
    /// Useful for lighting calculations where extreme precision is not required.
    ///
    /// # Performance
    ///
    /// Passes `self` by value rather than reference to avoid pointer indirection
    /// and improve register allocation for small `Copy` types.
    #[must_use]
    #[inline]
    pub fn fast_normalize(self) -> Self {
        let len_sq = self.x * self.x + self.y * self.y + self.z * self.z;
        if len_sq > 0.0001 {
            let inv_len = fast_inv_sqrt(len_sq);
            Self {
                x: self.x * inv_len,
                y: self.y * inv_len,
                z: self.z * inv_len,
            }
        } else {
            self
        }
    }

    /// Calculates the squared length (magnitude) of the vector.
    ///
    /// Faster than `length()` as it avoids a square root operation.
    /// Useful for comparing distances.
    ///
    /// # Performance
    ///
    /// Passes `self` by value rather than reference to avoid pointer indirection
    /// and improve register allocation for small `Copy` types.
    #[must_use]
    #[inline]
    pub fn length_sq(self) -> f32 {
        self.x * self.x + self.y * self.y + self.z * self.z
    }

    /// Reflects this vector around a given normal vector.
    ///
    /// The formula used is $v - 2 \cdot (v \cdot n) \cdot n$.
    ///
    /// # Examples
    ///
    /// ```
    /// use abrash::math::Vec3;
    ///
    /// let v = Vec3::new(1.0, -1.0, 0.0);
    /// let n = Vec3::new(0.0, 1.0, 0.0);
    /// let r = v.reflect(n);
    ///
    /// assert!((r.x - 1.0).abs() < 1e-6);
    /// assert!((r.y - 1.0).abs() < 1e-6);
    /// assert!(r.z.abs() < 1e-6);
    /// ```
    ///
    /// # Performance
    ///
    /// This implementation takes `self` by value rather than by reference to avoid pointer indirection
    /// for a small struct. It also manually unfolds scalar components to avoid intermediate struct
    /// allocations and improve scalar instruction pipelining.
    #[must_use]
    #[inline]
    pub fn reflect(self, normal: Self) -> Self {
        // Equivalent to `self - normal * (2.0 * self.dot(normal))`
        // but manually unfolded to avoid intermediate Vec3 allocations
        // and allow better scalar instruction pipelining.
        let dot2 = 2.0 * (self.x * normal.x + self.y * normal.y + self.z * normal.z);
        Self {
            x: self.x - normal.x * dot2,
            y: self.y - normal.y * dot2,
            z: self.z - normal.z * dot2,
        }
    }

    /// Linearly interpolate between this vector and another.
    ///
    /// `t` is the interpolation factor (0.0 = self, 1.0 = other).
    #[must_use]
    #[inline]
    pub const fn min(self, other: Self) -> Self {
        Self {
            x: self.x.min(other.x),
            y: self.y.min(other.y),
            z: self.z.min(other.z),
        }
    }

    /// Returns a new vector containing the maximum value for each component.
    #[must_use]
    #[inline]
    pub const fn max(&self, other: Self) -> Self {
        Self {
            x: self.x.max(other.x),
            y: self.y.max(other.y),
            z: self.z.max(other.z),
        }
    }
}

impl Add for Vec3 {
    type Output = Self;
    #[inline]
    fn add(self, other: Self) -> Self {
        Self {
            x: self.x + other.x,
            y: self.y + other.y,
            z: self.z + other.z,
        }
    }
}

impl Sub for Vec3 {
    type Output = Self;
    #[inline]
    fn sub(self, other: Self) -> Self {
        Self {
            x: self.x - other.x,
            y: self.y - other.y,
            z: self.z - other.z,
        }
    }
}

impl Mul<f32> for Vec3 {
    type Output = Self;
    #[inline]
    fn mul(self, scalar: f32) -> Self {
        Self {
            x: self.x * scalar,
            y: self.y * scalar,
            z: self.z * scalar,
        }
    }
}

impl Mul for Vec3 {
    type Output = Self;
    #[inline]
    fn mul(self, other: Self) -> Self {
        Self {
            x: self.x * other.x,
            y: self.y * other.y,
            z: self.z * other.z,
        }
    }
}

impl std::ops::Div<f32> for Vec3 {
    type Output = Self;
    #[inline]
    fn div(self, scalar: f32) -> Self {
        let inv = 1.0 / scalar;
        Self {
            x: self.x * inv,
            y: self.y * inv,
            z: self.z * inv,
        }
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Vec4 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub w: f32,
}

impl Vec4 {
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
    /// use abrash::math::Vec4;
    ///
    /// let v = Vec4::new(1.0, 2.0, 3.0, 1.0);
    /// assert_eq!(v.w, 1.0);
    /// ```

    pub const fn new(x: f32, y: f32, z: f32, w: f32) -> Self {
        Self { x, y, z, w }
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