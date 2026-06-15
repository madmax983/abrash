#[allow(clippy::wildcard_imports)]
use super::*;
use std::mem::MaybeUninit;
use std::ops::{Add, Mul, Sub};

/// A 3-component vector commonly used for positions, directions, and colors.
///
/// # Examples
///
/// ```
/// use abrash_core::math::Vec3;
///
/// let v = Vec3::new(1.0, 2.0, 3.0);
/// assert_eq!(v.x, 1.0);
/// ```
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Vec3 {
    /// The X component.
    pub x: f32,
    /// The Y component.
    pub y: f32,
    /// The Z component.
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

    /// Spherical interpolation between this vector and another.
    ///
    /// Inputs are treated as directions and normalized internally.
    /// Falls back to normalized linear interpolation when vectors are nearly parallel.
    #[must_use]
    #[inline]
    pub fn slerp(self, other: Self, t: f32) -> Self {
        let a = self.normalize_or_zero();
        let b = other.normalize_or_zero();
        let dot = a.dot(b).clamp(-1.0, 1.0);

        // For tiny angles, lerp is numerically more stable and faster.
        if dot > 0.999_5 {
            return a.lerp(b, t).normalize_or_zero();
        }

        let theta = dot.acos();
        let sin_theta = theta.sin();
        if sin_theta.abs() <= 1e-6 {
            return a;
        }

        let w0 = ((1.0 - t) * theta).sin() / sin_theta;
        let w1 = (t * theta).sin() / sin_theta;
        (a * w0) + (b * w1)
    }

    /// A vector with all components set to `0.0`.
    pub const ZERO: Self = Self {
        x: 0.0,
        y: 0.0,
        z: 0.0,
    };
    /// A vector with all components set to `1.0`.
    pub const ONE: Self = Self {
        x: 1.0,
        y: 1.0,
        z: 1.0,
    };
    /// Positive X axis.
    pub const X: Self = Self {
        x: 1.0,
        y: 0.0,
        z: 0.0,
    };
    /// Positive Y axis.
    pub const Y: Self = Self {
        x: 0.0,
        y: 1.0,
        z: 0.0,
    };
    /// Positive Z axis.
    pub const Z: Self = Self {
        x: 0.0,
        y: 0.0,
        z: 1.0,
    };
    /// World up direction (+Y).
    pub const UP: Self = Self {
        x: 0.0,
        y: 1.0,
        z: 0.0,
    };
    /// World right direction (+X).
    pub const RIGHT: Self = Self {
        x: 1.0,
        y: 0.0,
        z: 0.0,
    };
    /// World forward direction (-Z, right-handed camera convention).
    pub const FORWARD: Self = Self {
        x: 0.0,
        y: 0.0,
        z: -1.0,
    };

    /// Creates a vector with all components set to `v`.
    #[must_use]
    #[inline]
    pub const fn splat(v: f32) -> Self {
        Self { x: v, y: v, z: v }
    }

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
    /// use abrash_core::math::Vec3;
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
    /// use abrash_core::math::Vec3;
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
        self.x
            .mul_add(self.x, self.y.mul_add(self.y, self.z * self.z))
            .sqrt()
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
    /// use abrash_core::math::Vec3;
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
        let len_sq = self.x * self.x + self.y * self.y + self.z * self.z;
        if len_sq > 0.000_000_01 {
            let inv_len = len_sq.sqrt().recip();
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
        let len_sq = self.length_sq();
        if len_sq > 0.000_000_01 {
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

    /// Distance to another vector.
    #[must_use]
    #[inline]
    pub fn distance(self, other: Self) -> f32 {
        let dx = self.x - other.x;
        let dy = self.y - other.y;
        let dz = self.z - other.z;
        (dx * dx + dy * dy + dz * dz).sqrt()
    }

    /// Squared distance to another vector.
    #[must_use]
    #[inline]
    pub fn distance_sq(self, other: Self) -> f32 {
        let dx = self.x - other.x;
        let dy = self.y - other.y;
        let dz = self.z - other.z;
        dx * dx + dy * dy + dz * dz
    }

    /// Returns a normalized unit vector or zero for tiny inputs.
    ///
    /// Unlike `normalize`, this never returns denormal tiny vectors.
    #[must_use]
    #[inline]
    pub fn normalize_or_zero(self) -> Self {
        let len_sq = self.length_sq();
        if len_sq > 0.000_000_01 {
            let inv_len = len_sq.sqrt().recip();
            Self::new(self.x * inv_len, self.y * inv_len, self.z * inv_len)
        } else {
            Self::ZERO
        }
    }

    /// Moves this point toward `target` by at most `max_delta`.
    #[must_use]
    #[inline]
    pub fn move_towards(self, target: Self, max_delta: f32) -> Self {
        let to = target - self;
        let dist_sq = to.length_sq();
        if dist_sq <= max_delta * max_delta || dist_sq <= f32::EPSILON {
            target
        } else {
            let inv_dist = dist_sq.sqrt().recip();
            self + to * (max_delta * inv_dist)
        }
    }

    /// Reflects this vector around a given normal vector.
    ///
    /// The formula used is $v - 2 \cdot (v \cdot n) \cdot n$.
    ///
    /// # Examples
    ///
    /// ```
    /// use abrash_core::math::Vec3;
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

    /// Reflects this vector around a *unit-length* normal vector.
    ///
    /// This avoids any extra normalization/division and is ideal for hot shading paths
    /// where normals are already normalized.
    #[must_use]
    #[inline]
    pub fn reflect_normalized(self, unit_normal: Self) -> Self {
        let dot2 = 2.0 * self.dot(unit_normal);
        Self {
            x: self.x - unit_normal.x * dot2,
            y: self.y - unit_normal.y * dot2,
            z: self.z - unit_normal.z * dot2,
        }
    }

    /// Refracts this vector through a surface with the given normal.
    ///
    /// `eta` is the ratio of refractive indices (`n1 / n2`).
    /// Returns `Vec3::ZERO` when total internal reflection occurs.
    #[must_use]
    #[inline]
    pub fn refract(self, normal: Self, eta: f32) -> Self {
        let cos_i = (-self.dot(normal)).clamp(-1.0, 1.0);
        let k = 1.0 - eta * eta * (1.0 - cos_i * cos_i);
        if k < 0.0 {
            Self::ZERO
        } else {
            self * eta + normal * (eta * cos_i - k.sqrt())
        }
    }

    /// Returns this vector oriented to face away from a reference direction.
    ///
    /// Equivalent to GLSL `faceforward(n, i, nref)` when called as
    /// `n.face_forward(i, nref)`.
    #[must_use]
    #[inline]
    pub fn face_forward(self, incident: Self, reference_normal: Self) -> Self {
        if reference_normal.dot(incident) < 0.0 {
            self
        } else {
            self * -1.0
        }
    }

    /// Projects this vector onto another vector.
    ///
    /// Returns `Vec3::ZERO` when `onto` is near zero to avoid division by tiny values.
    #[must_use]
    #[inline]
    pub fn project_onto(self, onto: Self) -> Self {
        let denom = onto.length_sq();
        if denom <= 0.000_000_01 {
            return Self::ZERO;
        }
        onto * (self.dot(onto) / denom)
    }

    /// Projects this vector onto a *unit-length* direction.
    ///
    /// Returns `Vec3::ZERO` for degenerate inputs to avoid amplification of NaNs/Infs.
    #[must_use]
    #[inline]
    pub fn project_onto_normalized(self, onto_unit: Self) -> Self {
        let len_sq = onto_unit.length_sq();
        if len_sq <= 0.000_000_01 {
            return Self::ZERO;
        }
        onto_unit * self.dot(onto_unit)
    }

    /// Reject this vector from another vector (component orthogonal to `onto`).
    #[must_use]
    #[inline]
    pub fn reject_from(self, onto: Self) -> Self {
        self - self.project_onto(onto)
    }

    /// Projects this vector onto a plane defined by its unit normal.
    ///
    /// Removes the component along `normal`, leaving only the tangential part.
    /// `unit_normal` must be unit-length.
    ///
    /// # Examples
    ///
    /// ```
    /// use abrash_core::math::Vec3;
    ///
    /// // Project onto XZ plane (normal = +Y): Y component becomes zero
    /// let v = Vec3::new(1.0, 5.0, 2.0);
    /// let flat = v.project_onto_plane(Vec3::Y);
    /// assert!(flat.y.abs() < 1e-5);
    /// assert!((flat.x - 1.0).abs() < 1e-5);
    /// assert!((flat.z - 2.0).abs() < 1e-5);
    /// ```
    #[must_use]
    #[inline]
    pub fn project_onto_plane(self, unit_normal: Self) -> Self {
        self - unit_normal * self.dot(unit_normal)
    }

    /// Returns angle between vectors in radians.
    ///
    /// Returns 0 for near-zero length inputs.
    #[must_use]
    #[inline]
    pub fn angle_between(self, other: Self) -> f32 {
        let denom = self.length() * other.length();
        if denom <= 0.000_000_01 {
            return 0.0;
        }
        (self.dot(other) / denom).clamp(-1.0, 1.0).acos()
    }

    /// Builds an orthonormal basis from this direction.
    ///
    /// Returns two unit vectors `(tangent, bitangent)` that are perpendicular
    /// The smallest of the three components.
    ///
    /// # Examples
    /// ```
    /// use abrash_core::math::Vec3;
    /// assert_eq!(Vec3::new(3.0, 1.0, 2.0).min_component(), 1.0);
    /// ```
    #[must_use]
    #[inline]
    pub const fn min_component(self) -> f32 {
        self.x.min(self.y).min(self.z)
    }

    /// The largest of the three components.
    ///
    /// # Examples
    /// ```
    /// use abrash_core::math::Vec3;
    /// assert_eq!(Vec3::new(3.0, 1.0, 2.0).max_component(), 3.0);
    /// ```
    #[must_use]
    #[inline]
    pub const fn max_component(self) -> f32 {
        self.x.max(self.y).max(self.z)
    }

    /// Component-wise `x^exp`.
    ///
    /// # Examples
    /// ```
    /// use abrash_core::math::Vec3;
    /// let v = Vec3::new(2.0, 3.0, 4.0).pow(2.0);
    /// assert!((v.x - 4.0).abs() < 1e-5);
    /// assert!((v.y - 9.0).abs() < 1e-5);
    /// ```
    #[must_use]
    #[inline]
    pub fn pow(self, exp: f32) -> Self {
        Self::new(self.x.powf(exp), self.y.powf(exp), self.z.powf(exp))
    }

    /// Component-wise `e^x`.
    ///
    /// # Examples
    /// ```
    /// use abrash_core::math::Vec3;
    /// let v = Vec3::new(0.0, 1.0, 2.0).exp();
    /// assert!((v.x - 1.0).abs() < 1e-5);
    /// ```
    #[must_use]
    #[inline]
    pub fn exp(self) -> Self {
        Self::new(self.x.exp(), self.y.exp(), self.z.exp())
    }

    /// Component-wise natural log `ln(x)`.  Returns `-inf` for non-positive components.
    ///
    /// # Examples
    /// ```
    /// use abrash_core::math::Vec3;
    /// let v = Vec3::new(1.0, std::f32::consts::E, 1.0).log();
    /// assert!(v.x.abs() < 1e-5);
    /// assert!((v.y - 1.0).abs() < 1e-4);
    /// ```
    #[must_use]
    #[inline]
    pub fn log(self) -> Self {
        Self::new(self.x.ln(), self.y.ln(), self.z.ln())
    }

    /// to the (normalized) input and each other.
    #[must_use]
    #[inline]
    pub fn orthonormal_basis(self) -> (Self, Self) {
        let n = if self.length_sq() > 0.000_000_01 {
            self.normalize()
        } else {
            Self::new(0.0, 0.0, 1.0)
        };

        let helper = if n.z.abs() < 0.999 {
            Self::new(0.0, 0.0, 1.0)
        } else {
            Self::new(0.0, 1.0, 0.0)
        };

        let tangent = helper.cross(n).normalize();
        let bitangent = n.cross(tangent);
        (tangent, bitangent)
    }

    /// Computes barycentric coordinates of this point relative to triangle `(a, b, c)`.
    ///
    /// Returns `None` for degenerate triangles (near-zero area).
    /// The returned vector stores `(u, v, w)` such that:
    /// `self = a * u + b * v + c * w` and `u + v + w = 1`.
    #[must_use]
    #[inline]
    pub fn barycentric_coordinates(self, a: Self, b: Self, c: Self) -> Option<Self> {
        let v0 = b - a;
        let v1 = c - a;
        let v2 = self - a;

        let d00 = v0.dot(v0);
        let d01 = v0.dot(v1);
        let d11 = v1.dot(v1);
        let d20 = v2.dot(v0);
        let d21 = v2.dot(v1);

        #[allow(clippy::suspicious_operation_groupings)]
        let denom = d00 * d11 - d01 * d01;
        if denom.abs() <= 1e-8 {
            return None;
        }

        let inv_denom = 1.0 / denom;
        let v = (d11 * d20 - d01 * d21) * inv_denom;
        let w = (d00 * d21 - d01 * d20) * inv_denom;
        let u = 1.0 - v - w;
        Some(Self::new(u, v, w))
    }

    /// Reconstructs a point from barycentric coordinates over triangle `(a, b, c)`.
    ///
    /// `bary` stores `(u, v, w)` weights corresponding to vertices `(a, b, c)`.
    #[must_use]
    #[inline]
    pub fn from_barycentric(a: Self, b: Self, c: Self, bary: Self) -> Self {
        a * bary.x + b * bary.y + c * bary.z
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
    pub const fn max(self, other: Self) -> Self {
        Self {
            x: self.x.max(other.x),
            y: self.y.max(other.y),
            z: self.z.max(other.z),
        }
    }

    /// Component-wise absolute value.
    #[must_use]
    #[inline]
    pub const fn abs(self) -> Self {
        Self {
            x: self.x.abs(),
            y: self.y.abs(),
            z: self.z.abs(),
        }
    }

    /// Component-wise clamp.
    #[must_use]
    #[inline]
    pub const fn clamp(self, min: Self, max: Self) -> Self {
        Self {
            x: self.x.clamp(min.x, max.x),
            y: self.y.clamp(min.y, max.y),
            z: self.z.clamp(min.z, max.z),
        }
    }

    /// Returns true when all components are finite.
    #[must_use]
    #[inline]
    #[allow(clippy::missing_const_for_fn)]
    pub fn is_finite(self) -> bool {
        self.x.is_finite() && self.y.is_finite() && self.z.is_finite()
    }

    /// Clamps vector magnitude to at most `max_length`.
    ///
    /// Useful for velocity limiting and stable iterative solvers.
    #[must_use]
    #[inline]
    pub fn clamp_length(self, max_length: f32) -> Self {
        if max_length <= 0.0 {
            return Self::ZERO;
        }

        let len_sq = self.length_sq();
        let max_sq = max_length * max_length;
        if len_sq <= max_sq || len_sq <= 0.000_000_01 {
            self
        } else {
            let scale = max_length * len_sq.sqrt().recip();
            self * scale
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
        }
    }

    /// Component-wise step: returns `1.0` if `self >= edge`, else `0.0`.
    ///
    /// GLSL equivalent of `step(edge, x)`.
    #[must_use]
    #[inline]
    pub fn step(self, edge: Self) -> Self {
        Self {
            x: if self.x >= edge.x { 1.0 } else { 0.0 },
            y: if self.y >= edge.y { 1.0 } else { 0.0 },
            z: if self.z >= edge.z { 1.0 } else { 0.0 },
        }
    }

    /// Component-wise Hermite smoothstep between `edge0` and `edge1`.
    ///
    /// Matches GLSL `smoothstep(edge0, edge1, self)`.
    #[must_use]
    #[inline]
    pub fn smoothstep(self, edge0: Self, edge1: Self) -> Self {
        let tx = ((self.x - edge0.x) / (edge1.x - edge0.x)).clamp(0.0, 1.0);
        let ty = ((self.y - edge0.y) / (edge1.y - edge0.y)).clamp(0.0, 1.0);
        let tz = ((self.z - edge0.z) / (edge1.z - edge0.z)).clamp(0.0, 1.0);
        Self::new(
            tx * tx * (3.0 - 2.0 * tx),
            ty * ty * (3.0 - 2.0 * ty),
            tz * tz * (3.0 - 2.0 * tz),
        )
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
