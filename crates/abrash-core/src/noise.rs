//! Procedural noise functions for textures, terrain, and animation.
//!
//! Implements three complementary noise types:
//!
//! * **[`value_noise_2d`] / [`value_noise_3d`]** — Hash-based lattice noise. Fast,
//!   smooth, no gradient artifacts. Good for cloud-like patterns.
//! * **[`gradient_noise_2d`] / [`gradient_noise_3d`]** — Classic Perlin gradient
//!   noise (Ken Perlin's improved 2002 permutation table). Higher quality,
//!   no directional bias, slight performance cost.
//! * **[`fbm_2d`] / [`fbm_3d`]** — Fractional Brownian Motion layering of either
//!   noise type. Produces natural-looking fractal detail (mountains, clouds).
//!
//! All functions return values in **\[-1.0, 1.0\]** for gradient noise and
//! **\[0.0, 1.0\]** for value noise. FBM output is normalized to \[-1.0, 1.0\].
//!
//! # Examples
//!
//! ```
//! use abrash_core::noise::{gradient_noise_2d, fbm_2d};
//!
//! // Single octave — smooth hills
//! let n = gradient_noise_2d(1.5, 2.3);
//! assert!(n >= -1.0 && n <= 1.0);
//!
//! // 6 octaves of fBm — rocky terrain
//! let height = fbm_2d(3.0, 7.0, 6, 2.0, 0.5);
//! assert!(height >= -1.0 && height <= 1.0);
//! ```

// ── Permutation table (Perlin improved noise, 2002) ───────────────────────────

/// Perlin's improved permutation table, doubled to avoid modulo in indexing.
const PERM: [u8; 512] = {
    const P: [u8; 256] = [
        151, 160, 137, 91, 90, 15, 131, 13, 201, 95, 96, 53, 194, 233, 7, 225, 140, 36, 103, 30,
        69, 142, 8, 99, 37, 240, 21, 10, 23, 190, 6, 148, 247, 120, 234, 75, 0, 26, 197, 62, 94,
        252, 219, 203, 117, 35, 11, 32, 57, 177, 33, 88, 237, 149, 56, 87, 174, 20, 125, 136, 171,
        168, 68, 175, 74, 165, 71, 134, 139, 48, 27, 166, 77, 146, 158, 231, 83, 111, 229, 122, 60,
        211, 133, 230, 220, 105, 92, 41, 55, 46, 245, 40, 244, 102, 143, 54, 65, 25, 63, 161, 1,
        216, 80, 73, 209, 76, 132, 187, 208, 89, 18, 169, 200, 196, 135, 130, 116, 188, 159, 86,
        164, 100, 109, 198, 173, 186, 3, 64, 52, 217, 226, 250, 124, 123, 5, 202, 38, 147, 118,
        126, 255, 82, 85, 212, 207, 206, 59, 227, 47, 16, 58, 17, 182, 189, 28, 42, 223, 183, 170,
        213, 119, 248, 152, 2, 44, 154, 163, 70, 221, 153, 101, 155, 167, 43, 172, 9, 129, 22, 39,
        253, 19, 98, 108, 110, 79, 113, 224, 232, 178, 185, 112, 104, 218, 246, 97, 228, 251, 34,
        242, 193, 238, 210, 144, 12, 191, 179, 162, 241, 81, 51, 145, 235, 249, 14, 239, 107, 49,
        192, 214, 31, 181, 199, 106, 157, 184, 84, 204, 176, 115, 121, 50, 45, 127, 4, 150, 254,
        138, 236, 205, 93, 222, 114, 67, 29, 24, 72, 243, 141, 128, 195, 78, 66, 215, 61, 156, 180,
    ];
    let mut out = [0u8; 512];
    let mut i = 0;
    while i < 256 {
        out[i] = P[i];
        out[i + 256] = P[i];
        i += 1;
    }
    out
};

// ── Value noise ───────────────────────────────────────────────────────────────

/// 2D value noise in `[0.0, 1.0]`.
///
/// Hash-based lattice noise with quintic interpolation. Fast and smooth.
#[must_use]
pub fn value_noise_2d(x: f32, y: f32) -> f32 {
    let xi = x.floor() as i32;
    let yi = y.floor() as i32;
    let xf = x - xi as f32;
    let yf = y - yi as f32;

    let u = fade(xf);
    let v = fade(yf);

    let a = hash2(xi, yi);
    let b = hash2(xi + 1, yi);
    let c = hash2(xi, yi + 1);
    let d = hash2(xi + 1, yi + 1);

    let ab = lerp_f(a, b, u);
    let cd = lerp_f(c, d, u);
    lerp_f(ab, cd, v)
}

/// 3D value noise in `[0.0, 1.0]`.
#[must_use]
pub fn value_noise_3d(x: f32, y: f32, z: f32) -> f32 {
    let xi = x.floor() as i32;
    let yi = y.floor() as i32;
    let zi = z.floor() as i32;
    let xf = x - xi as f32;
    let yf = y - yi as f32;
    let zf = z - zi as f32;

    let u = fade(xf);
    let v = fade(yf);
    let w = fade(zf);

    let v000 = hash3(xi, yi, zi);
    let v100 = hash3(xi + 1, yi, zi);
    let v010 = hash3(xi, yi + 1, zi);
    let v110 = hash3(xi + 1, yi + 1, zi);
    let v001 = hash3(xi, yi, zi + 1);
    let v101 = hash3(xi + 1, yi, zi + 1);
    let v011 = hash3(xi, yi + 1, zi + 1);
    let v111 = hash3(xi + 1, yi + 1, zi + 1);

    let x0 = lerp_f(v000, v100, u);
    let x1 = lerp_f(v010, v110, u);
    let x2 = lerp_f(v001, v101, u);
    let x3 = lerp_f(v011, v111, u);
    let y0 = lerp_f(x0, x1, v);
    let y1 = lerp_f(x2, x3, v);
    lerp_f(y0, y1, w)
}

// ── Gradient noise (Perlin) ───────────────────────────────────────────────────

/// 2D Perlin gradient noise in `[-1.0, 1.0]`.
///
/// Uses Ken Perlin's improved 2002 permutation table and gradient vectors.
/// Free of the directional bias present in the original 1985 implementation.
#[must_use]
pub fn gradient_noise_2d(x: f32, y: f32) -> f32 {
    let xi = x.floor() as i32;
    let yi = y.floor() as i32;
    let xf = x - xi as f32;
    let yf = y - yi as f32;

    let u = fade(xf);
    let v = fade(yf);

    let aa = perm2(xi, yi);
    let ab = perm2(xi, yi + 1);
    let ba = perm2(xi + 1, yi);
    let bb = perm2(xi + 1, yi + 1);

    let g00 = grad2(aa, xf, yf);
    let g10 = grad2(ba, xf - 1.0, yf);
    let g01 = grad2(ab, xf, yf - 1.0);
    let g11 = grad2(bb, xf - 1.0, yf - 1.0);

    let x0 = lerp_f(g00, g10, u);
    let x1 = lerp_f(g01, g11, u);
    lerp_f(x0, x1, v)
}

/// 3D Perlin gradient noise in `[-1.0, 1.0]`.
#[must_use]
pub fn gradient_noise_3d(x: f32, y: f32, z: f32) -> f32 {
    let xi = x.floor() as i32;
    let yi = y.floor() as i32;
    let zi = z.floor() as i32;
    let xf = x - xi as f32;
    let yf = y - yi as f32;
    let zf = z - zi as f32;

    let u = fade(xf);
    let v = fade(yf);
    let w = fade(zf);

    let a = perm1(xi);
    let b = perm1(xi + 1);

    let aa = perm2_raw(a, yi);
    let ab = perm2_raw(a, yi + 1);
    let ba = perm2_raw(b, yi);
    let bb = perm2_raw(b, yi + 1);

    let aaa = perm3_raw(aa, zi) as usize;
    let aab = perm3_raw(aa, zi + 1) as usize;
    let aba = perm3_raw(ab, zi) as usize;
    let abb = perm3_raw(ab, zi + 1) as usize;
    let baa = perm3_raw(ba, zi) as usize;
    let bab = perm3_raw(ba, zi + 1) as usize;
    let bba = perm3_raw(bb, zi) as usize;
    let bbb = perm3_raw(bb, zi + 1) as usize;

    let g000 = grad3(aaa, xf, yf, zf);
    let g100 = grad3(baa, xf - 1.0, yf, zf);
    let g010 = grad3(aba, xf, yf - 1.0, zf);
    let g110 = grad3(bba, xf - 1.0, yf - 1.0, zf);
    let g001 = grad3(aab, xf, yf, zf - 1.0);
    let g101 = grad3(bab, xf - 1.0, yf, zf - 1.0);
    let g011 = grad3(abb, xf, yf - 1.0, zf - 1.0);
    let g111 = grad3(bbb, xf - 1.0, yf - 1.0, zf - 1.0);

    let x00 = lerp_f(g000, g100, u);
    let x10 = lerp_f(g010, g110, u);
    let x01 = lerp_f(g001, g101, u);
    let x11 = lerp_f(g011, g111, u);
    let y0 = lerp_f(x00, x10, v);
    let y1 = lerp_f(x01, x11, v);
    lerp_f(y0, y1, w)
}

// ── Fractal Brownian Motion ───────────────────────────────────────────────────

/// 2D fractal Brownian motion using Perlin gradient noise.
///
/// Sums `octaves` layers of gradient noise, each scaled by `lacunarity` in
/// frequency and `gain` in amplitude. Output is in `[-1.0, 1.0]`.
///
/// Typical values: `lacunarity = 2.0`, `gain = 0.5` (decreasing amplitude).
///
/// # Arguments
///
/// * `x`, `y` — Input coordinates.
/// * `octaves` — Number of noise layers (2–8 typical).
/// * `lacunarity` — Frequency multiplier per octave (default 2.0).
/// * `gain` — Amplitude multiplier per octave (default 0.5).
#[must_use]
pub fn fbm_2d(x: f32, y: f32, octaves: u32, lacunarity: f32, gain: f32) -> f32 {
    let mut value = 0.0f32;
    let mut amplitude = 1.0f32;
    let mut frequency = 1.0f32;
    let mut max_amplitude = 0.0f32;

    for _ in 0..octaves {
        value += gradient_noise_2d(x * frequency, y * frequency) * amplitude;
        max_amplitude += amplitude;
        amplitude *= gain;
        frequency *= lacunarity;
    }

    value / max_amplitude
}

/// 3D fractal Brownian motion using Perlin gradient noise.
///
/// See [`fbm_2d`] for parameter documentation.
#[must_use]
pub fn fbm_3d(x: f32, y: f32, z: f32, octaves: u32, lacunarity: f32, gain: f32) -> f32 {
    let mut value = 0.0f32;
    let mut amplitude = 1.0f32;
    let mut frequency = 1.0f32;
    let mut max_amplitude = 0.0f32;

    for _ in 0..octaves {
        value += gradient_noise_3d(x * frequency, y * frequency, z * frequency) * amplitude;
        max_amplitude += amplitude;
        amplitude *= gain;
        frequency *= lacunarity;
    }

    value / max_amplitude
}

/// 2D fractal Brownian motion using value noise (faster than gradient fBm).
#[must_use]
pub fn fbm_value_2d(x: f32, y: f32, octaves: u32, lacunarity: f32, gain: f32) -> f32 {
    let mut value = 0.0f32;
    let mut amplitude = 1.0f32;
    let mut frequency = 1.0f32;
    let mut max_amplitude = 0.0f32;

    for _ in 0..octaves {
        // Remap value noise [0,1] to [-1,1] before layering
        let n = value_noise_2d(x * frequency, y * frequency) * 2.0 - 1.0;
        value += n * amplitude;
        max_amplitude += amplitude;
        amplitude *= gain;
        frequency *= lacunarity;
    }

    value / max_amplitude
}

// ── Turbulence (absolute-value fBm) ──────────────────────────────────────────

/// 2D turbulence: sum of |octaves| — produces sharp ridges. Output in `[0.0, 1.0]`.
#[must_use]
pub fn turbulence_2d(x: f32, y: f32, octaves: u32, lacunarity: f32, gain: f32) -> f32 {
    let mut value = 0.0f32;
    let mut amplitude = 1.0f32;
    let mut frequency = 1.0f32;
    let mut max_amplitude = 0.0f32;

    for _ in 0..octaves {
        value += gradient_noise_2d(x * frequency, y * frequency).abs() * amplitude;
        max_amplitude += amplitude;
        amplitude *= gain;
        frequency *= lacunarity;
    }

    value / max_amplitude
}

// ── Worley (cellular) noise ───────────────────────────────────────────────────

/// 2D Worley (cellular / Voronoi) noise.
///
/// Returns **F1**: the Euclidean distance to the nearest cell feature point.
/// Output is in roughly `[0.0, 0.7]` for `jitter = 1.0`.
///
/// `jitter ∈ [0.0, 1.0]` controls how randomly the feature point is displaced
/// from its cell center.  `0.0` → perfectly regular grid; `1.0` → full random.
///
/// # Examples
///
/// ```
/// use abrash_core::noise::worley_noise_2d;
///
/// let d = worley_noise_2d(1.5, 2.3, 1.0);
/// assert!(d >= 0.0 && d <= 1.5);
/// ```
#[must_use]
pub fn worley_noise_2d(x: f32, y: f32, jitter: f32) -> f32 {
    let xi = x.floor() as i32;
    let yi = y.floor() as i32;
    let xf = x - xi as f32;
    let yf = y - yi as f32;

    let mut min_dist_sq = f32::MAX;

    for dy in -1_i32..=1 {
        for dx in -1_i32..=1 {
            let cx = xi + dx;
            let cy = yi + dy;

            // Two independent hash values for x and y jitter, each in [0.0, 1.0].
            let rx = hash2(cx, cy);
            let ry = hash2(cx ^ 0x1abe_cd5, cy ^ 0x9e37_79b9u32 as i32);

            // Feature point at cell center + jitter offset.
            let fp_x = dx as f32 + 0.5 + (rx - 0.5) * jitter;
            let fp_y = dy as f32 + 0.5 + (ry - 0.5) * jitter;

            let ddx = fp_x - xf;
            let ddy = fp_y - yf;
            let d_sq = ddx * ddx + ddy * ddy;
            if d_sq < min_dist_sq {
                min_dist_sq = d_sq;
            }
        }
    }

    min_dist_sq.sqrt()
}

/// 3D Worley (cellular / Voronoi) noise.
///
/// Returns **F1**: the Euclidean distance to the nearest cell feature point.
/// Output is in roughly `[0.0, 0.9]` for `jitter = 1.0`.
///
/// See [`worley_noise_2d`] for the `jitter` parameter description.
///
/// # Examples
///
/// ```
/// use abrash_core::noise::worley_noise_3d;
///
/// let d = worley_noise_3d(1.0, 2.0, 3.0, 1.0);
/// assert!(d >= 0.0 && d <= 2.0);
/// ```
#[must_use]
pub fn worley_noise_3d(x: f32, y: f32, z: f32, jitter: f32) -> f32 {
    let xi = x.floor() as i32;
    let yi = y.floor() as i32;
    let zi = z.floor() as i32;
    let xf = x - xi as f32;
    let yf = y - yi as f32;
    let zf = z - zi as f32;

    let mut min_dist_sq = f32::MAX;

    for dz in -1_i32..=1 {
        for dy in -1_i32..=1 {
            for dx in -1_i32..=1 {
                let cx = xi + dx;
                let cy = yi + dy;
                let cz = zi + dz;

                let rx = hash3(cx, cy, cz);
                let ry = hash3(
                    cx ^ 0x1abe_cd5,
                    cy ^ 0x9e37_79b9u32 as i32,
                    cz ^ 0x6c62_272e,
                );
                let rz = hash3(
                    cx ^ 0x517c_c1b7,
                    cy ^ 0x27d4_eb2fu32 as i32,
                    cz ^ 0xb492_2022u32 as i32,
                );

                let fp_x = dx as f32 + 0.5 + (rx - 0.5) * jitter;
                let fp_y = dy as f32 + 0.5 + (ry - 0.5) * jitter;
                let fp_z = dz as f32 + 0.5 + (rz - 0.5) * jitter;

                let ddx = fp_x - xf;
                let ddy = fp_y - yf;
                let ddz = fp_z - zf;
                let d_sq = ddx * ddx + ddy * ddy + ddz * ddz;
                if d_sq < min_dist_sq {
                    min_dist_sq = d_sq;
                }
            }
        }
    }

    min_dist_sq.sqrt()
}

// ── Internal helpers ──────────────────────────────────────────────────────────

/// Quintic fade: `6t⁵ - 15t⁴ + 10t³` — zero first and second derivatives at 0 and 1.
#[inline]
fn fade(t: f32) -> f32 {
    t * t * t * (t * (t * 6.0 - 15.0) + 10.0)
}

#[inline]
fn lerp_f(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

/// 32-bit integer hash with good avalanche properties.
#[inline]
const fn hash_u32(mut x: u32) -> u32 {
    x = x.wrapping_add(0x9e37_79b9);
    x = (x ^ (x >> 16)).wrapping_mul(0x45d9_f3b);
    x = (x ^ (x >> 16)).wrapping_mul(0x45d9_f3b);
    x ^ (x >> 16)
}

/// Hash 2D integer coordinates to [0.0, 1.0].
#[inline]
fn hash2(x: i32, y: i32) -> f32 {
    let h = hash_u32(hash_u32(x as u32).wrapping_add(y as u32));
    (h as f32) * (1.0 / u32::MAX as f32)
}

/// Hash 3D integer coordinates to [0.0, 1.0].
#[inline]
fn hash3(x: i32, y: i32, z: i32) -> f32 {
    let h = hash_u32(hash_u32(hash_u32(x as u32).wrapping_add(y as u32)).wrapping_add(z as u32));
    (h as f32) * (1.0 / u32::MAX as f32)
}

/// Permutation table lookup for a single index (wraps at 256).
#[inline]
const fn perm1(x: i32) -> usize {
    PERM[(x & 255) as usize] as usize
}

#[inline]
const fn perm2_raw(a: usize, y: i32) -> usize {
    PERM[(a + (y & 255) as usize) & 511] as usize
}

#[inline]
const fn perm3_raw(a: usize, z: i32) -> usize {
    PERM[(a + (z & 255) as usize) & 511] as usize
}

/// Permutation lookup for 2D (x,y) → hash byte.
#[inline]
const fn perm2(x: i32, y: i32) -> usize {
    let xi = (x & 255) as usize;
    let yi = (y & 255) as usize;
    PERM[(PERM[xi] as usize + yi) & 511] as usize
}

/// 2D gradient dot product from hash.
#[inline]
fn grad2(hash: usize, x: f32, y: f32) -> f32 {
    // 8 gradient directions in 2D: (±1, 0), (0, ±1), (±√½, ±√½)
    match hash & 7 {
        0 => x,
        1 => -x,
        2 => y,
        3 => -y,
        4 => x + y,
        5 => -x + y,
        6 => x - y,
        _ => -x - y,
    }
}

/// 3D gradient dot product from hash (12 Perlin gradients).
#[inline]
fn grad3(hash: usize, x: f32, y: f32, z: f32) -> f32 {
    // 12 gradients from Perlin's improved noise (2002)
    // arms 0/12 and 9/13 are intentional duplicates for even distribution
    match hash & 15 {
        0 | 12 => x + y,
        1 => -x + y,
        2 => x - y,
        3 => -x - y,
        4 => x + z,
        5 => -x + z,
        6 => x - z,
        7 => -x - z,
        8 => y + z,
        9 | 13 => -y + z,
        10 => y - z,
        14 => y - x,
        _ => -y - z, // covers 11, 15
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn value_noise_range_2d() {
        for i in 0..100 {
            let x = i as f32 * 0.37;
            let y = i as f32 * 0.13;
            let n = value_noise_2d(x, y);
            assert!(n >= 0.0 && n <= 1.0, "value_noise_2d out of range: {n}");
        }
    }

    #[test]
    fn value_noise_range_3d() {
        for i in 0..100 {
            let x = i as f32 * 0.23;
            let y = i as f32 * 0.17;
            let z = i as f32 * 0.41;
            let n = value_noise_3d(x, y, z);
            assert!(n >= 0.0 && n <= 1.0, "value_noise_3d out of range: {n}");
        }
    }

    #[test]
    fn gradient_noise_range_2d() {
        let mut min = f32::MAX;
        let mut max = f32::MIN;
        for i in 0..200 {
            let x = i as f32 * 0.19;
            let y = i as f32 * 0.31;
            let n = gradient_noise_2d(x, y);
            assert!(n >= -1.0 && n <= 1.0, "gradient_noise_2d out of range: {n}");
            min = min.min(n);
            max = max.max(n);
        }
        // Should explore a reasonable range (not all zeros)
        assert!(max > 0.1, "gradient_noise_2d max too low: {max}");
        assert!(min < -0.1, "gradient_noise_2d min too high: {min}");
    }

    #[test]
    fn gradient_noise_range_3d() {
        for i in 0..100 {
            let x = i as f32 * 0.29;
            let y = i as f32 * 0.37;
            let z = i as f32 * 0.53;
            let n = gradient_noise_3d(x, y, z);
            assert!(n >= -1.0 && n <= 1.0, "gradient_noise_3d out of range: {n}");
        }
    }

    #[test]
    fn fbm_range_2d() {
        for i in 0..100 {
            let x = i as f32 * 0.41;
            let y = i as f32 * 0.23;
            let n = fbm_2d(x, y, 6, 2.0, 0.5);
            assert!(n >= -1.0 && n <= 1.0, "fbm_2d out of range: {n}");
        }
    }

    #[test]
    fn fbm_range_3d() {
        for i in 0..50 {
            let x = i as f32 * 0.31;
            let y = i as f32 * 0.17;
            let z = i as f32 * 0.59;
            let n = fbm_3d(x, y, z, 4, 2.0, 0.5);
            assert!(n >= -1.0 && n <= 1.0, "fbm_3d out of range: {n}");
        }
    }

    #[test]
    fn turbulence_range_2d() {
        for i in 0..100 {
            let x = i as f32 * 0.27;
            let y = i as f32 * 0.43;
            let n = turbulence_2d(x, y, 4, 2.0, 0.5);
            assert!(n >= 0.0 && n <= 1.0, "turbulence out of range: {n}");
        }
    }

    #[test]
    fn noise_is_deterministic() {
        let a = gradient_noise_2d(3.14, 2.71);
        let b = gradient_noise_2d(3.14, 2.71);
        assert_eq!(a, b);
    }

    #[test]
    fn value_noise_continuity() {
        // Noise should be C1 continuous — nearby points should be close in value
        let base = value_noise_2d(1.0, 1.0);
        let nearby = value_noise_2d(1.001, 1.001);
        assert!(
            (base - nearby).abs() < 0.01,
            "large discontinuity: {base} vs {nearby}"
        );
    }

    #[test]
    fn gradient_noise_lattice_points_near_zero() {
        // Gradient noise tends toward 0 at integer lattice points
        // (not exactly 0 due to gradient dot product, but small)
        let n = gradient_noise_2d(0.0, 0.0);
        assert!(n.abs() < 0.1, "gradient at origin: {n}");
    }

    #[test]
    fn worley_2d_non_negative() {
        for i in 0..200 {
            let x = i as f32 * 0.37 - 3.7;
            let y = i as f32 * 0.13 + 1.2;
            let d = worley_noise_2d(x, y, 1.0);
            assert!(d >= 0.0, "worley_noise_2d returned negative: {d}");
        }
    }

    #[test]
    fn worley_2d_jitter_zero_is_regular_grid() {
        // With jitter=0, every cell center is equidistant from the four
        // surrounding feature points (which sit at cell centers).
        // The cell center (0.5, 0.5) in any cell should yield distance ≈ 0.
        let d = worley_noise_2d(0.5, 0.5, 0.0);
        assert!(d < 0.01, "distance at own cell center should be ~0: {d}");
    }

    #[test]
    fn worley_2d_deterministic() {
        let a = worley_noise_2d(1.23, 4.56, 0.8);
        let b = worley_noise_2d(1.23, 4.56, 0.8);
        assert_eq!(a, b);
    }

    #[test]
    fn worley_3d_non_negative() {
        for i in 0..200 {
            let x = i as f32 * 0.41 - 2.1;
            let y = i as f32 * 0.17 + 0.5;
            let z = i as f32 * 0.23 - 1.3;
            let d = worley_noise_3d(x, y, z, 1.0);
            assert!(d >= 0.0, "worley_noise_3d returned negative: {d}");
        }
    }

    #[test]
    fn worley_3d_jitter_zero_is_regular_grid() {
        let d = worley_noise_3d(0.5, 0.5, 0.5, 0.0);
        assert!(d < 0.01, "distance at own cell center should be ~0: {d}");
    }

    #[test]
    fn worley_3d_deterministic() {
        let a = worley_noise_3d(7.1, -2.3, 0.9, 0.7);
        let b = worley_noise_3d(7.1, -2.3, 0.9, 0.7);
        assert_eq!(a, b);
    }
}
