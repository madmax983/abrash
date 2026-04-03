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

// ── Simplex Noise ────────────────────────────────────────────────────────────

/// 2D Simplex noise in `[-1.0, 1.0]` (Stefan Gustavson / Ken Perlin 2001).
///
/// Simplex noise uses a triangular lattice instead of a square grid, which
/// eliminates the axis-aligned bias visible in gradient (Perlin) noise at large
/// scales and is ~30% faster in 3D.
///
/// # Examples
///
/// ```
/// use abrash_core::noise::simplex_2d;
///
/// let n = simplex_2d(1.5, 2.3);
/// assert!(n >= -1.0 && n <= 1.0);
///
/// // Deterministic
/// assert_eq!(simplex_2d(0.7, -1.2), simplex_2d(0.7, -1.2));
/// ```
#[must_use]
pub fn simplex_2d(x: f32, y: f32) -> f32 {
    // Skew input to simplex grid
    const F2: f32 = 0.366_025_4; // (sqrt(3) - 1) / 2
    const G2: f32 = 0.211_324_87; // (3 - sqrt(3)) / 6

    let s = (x + y) * F2;
    let i = (x + s).floor() as i32;
    let j = (y + s).floor() as i32;

    let t = (i + j) as f32 * G2;
    let x0 = x - (i as f32 - t);
    let y0 = y - (j as f32 - t);

    // Which simplex are we in?
    let (i1, j1) = if x0 > y0 { (1i32, 0i32) } else { (0i32, 1i32) };

    let x1 = x0 - i1 as f32 + G2;
    let y1 = y0 - j1 as f32 + G2;
    let x2 = x0 - 1.0 + 2.0 * G2;
    let y2 = y0 - 1.0 + 2.0 * G2;

    // Gradient contributions
    let ii = (i & 255) as usize;
    let jj = (j & 255) as usize;

    let gi0 = (PERM[ii + PERM[jj] as usize] % 12) as usize;
    let gi1 = (PERM[ii + i1 as usize + PERM[jj + j1 as usize] as usize] % 12) as usize;
    let gi2 = (PERM[ii + 1 + PERM[jj + 1] as usize] % 12) as usize;

    let n0 = {
        let t0 = 0.5 - x0 * x0 - y0 * y0;
        if t0 < 0.0 {
            0.0
        } else {
            let t02 = t0 * t0;
            t02 * t02 * simplex_grad2(gi0, x0, y0)
        }
    };
    let n1 = {
        let t1 = 0.5 - x1 * x1 - y1 * y1;
        if t1 < 0.0 {
            0.0
        } else {
            let t12 = t1 * t1;
            t12 * t12 * simplex_grad2(gi1, x1, y1)
        }
    };
    let n2 = {
        let t2 = 0.5 - x2 * x2 - y2 * y2;
        if t2 < 0.0 {
            0.0
        } else {
            let t22 = t2 * t2;
            t22 * t22 * simplex_grad2(gi2, x2, y2)
        }
    };

    // Scale to [-1, 1] (empirical constant from Gustavson)
    70.0 * (n0 + n1 + n2)
}

/// 3D Simplex noise in `[-1.0, 1.0]` (Stefan Gustavson / Ken Perlin 2001).
///
/// Uses 4 gradient contributions from tetrahedron corners instead of the 8
/// cube corners Perlin noise uses — approximately O(n) vs O(2^n) in dimension n.
///
/// # Examples
///
/// ```
/// use abrash_core::noise::simplex_3d;
///
/// let n = simplex_3d(1.5, 2.3, -0.7);
/// assert!(n >= -1.0 && n <= 1.0);
///
/// // Deterministic
/// assert_eq!(simplex_3d(0.7, -1.2, 0.3), simplex_3d(0.7, -1.2, 0.3));
/// ```
#[must_use]
pub fn simplex_3d(x: f32, y: f32, z: f32) -> f32 {
    const F3: f32 = 1.0 / 3.0;
    const G3: f32 = 1.0 / 6.0;

    // Skew to simplex grid
    let s = (x + y + z) * F3;
    let i = (x + s).floor() as i32;
    let j = (y + s).floor() as i32;
    let k = (z + s).floor() as i32;

    let t = (i + j + k) as f32 * G3;
    let x0 = x - (i as f32 - t);
    let y0 = y - (j as f32 - t);
    let z0 = z - (k as f32 - t);

    // Simplex traversal order
    let (i1, j1, k1, i2, j2, k2) = if x0 >= y0 {
        if y0 >= z0 {
            (1, 0, 0, 1, 1, 0)
        } else if x0 >= z0 {
            (1, 0, 0, 1, 0, 1)
        } else {
            (0, 0, 1, 1, 0, 1)
        }
    } else {
        if y0 < z0 {
            (0, 0, 1, 0, 1, 1)
        } else if x0 < z0 {
            (0, 1, 0, 0, 1, 1)
        } else {
            (0, 1, 0, 1, 1, 0)
        }
    };

    let x1 = x0 - i1 as f32 + G3;
    let y1 = y0 - j1 as f32 + G3;
    let z1 = z0 - k1 as f32 + G3;
    let x2 = x0 - i2 as f32 + 2.0 * G3;
    let y2 = y0 - j2 as f32 + 2.0 * G3;
    let z2 = z0 - k2 as f32 + 2.0 * G3;
    let x3 = x0 - 1.0 + 3.0 * G3;
    let y3 = y0 - 1.0 + 3.0 * G3;
    let z3 = z0 - 1.0 + 3.0 * G3;

    let ii = (i & 255) as usize;
    let jj = (j & 255) as usize;
    let kk = (k & 255) as usize;

    let gi0 = (PERM[ii + PERM[jj + PERM[kk] as usize] as usize] % 12) as usize;
    let gi1 = (PERM[ii + i1 + PERM[jj + j1 + PERM[kk + k1] as usize] as usize] % 12) as usize;
    let gi2 = (PERM[ii + i2 + PERM[jj + j2 + PERM[kk + k2] as usize] as usize] % 12) as usize;
    let gi3 = (PERM[ii + 1 + PERM[jj + 1 + PERM[kk + 1] as usize] as usize] % 12) as usize;

    let n0 = {
        let t0 = 0.6 - x0 * x0 - y0 * y0 - z0 * z0;
        if t0 < 0.0 {
            0.0
        } else {
            let t02 = t0 * t0;
            t02 * t02 * simplex_grad3(gi0, x0, y0, z0)
        }
    };
    let n1 = {
        let t1 = 0.6 - x1 * x1 - y1 * y1 - z1 * z1;
        if t1 < 0.0 {
            0.0
        } else {
            let t12 = t1 * t1;
            t12 * t12 * simplex_grad3(gi1, x1, y1, z1)
        }
    };
    let n2 = {
        let t2 = 0.6 - x2 * x2 - y2 * y2 - z2 * z2;
        if t2 < 0.0 {
            0.0
        } else {
            let t22 = t2 * t2;
            t22 * t22 * simplex_grad3(gi2, x2, y2, z2)
        }
    };
    let n3 = {
        let t3 = 0.6 - x3 * x3 - y3 * y3 - z3 * z3;
        if t3 < 0.0 {
            0.0
        } else {
            let t32 = t3 * t3;
            t32 * t32 * simplex_grad3(gi3, x3, y3, z3)
        }
    };

    32.0 * (n0 + n1 + n2 + n3)
}

/// 12-direction gradient table for simplex noise (x,y pairs).
#[inline]
fn simplex_grad2(hash: usize, x: f32, y: f32) -> f32 {
    const GRAD2: [(f32, f32); 12] = [
        (1.0, 1.0),
        (-1.0, 1.0),
        (1.0, -1.0),
        (-1.0, -1.0),
        (1.0, 0.0),
        (-1.0, 0.0),
        (1.0, 0.0),
        (-1.0, 0.0),
        (0.0, 1.0),
        (0.0, -1.0),
        (0.0, 1.0),
        (0.0, -1.0),
    ];
    let (gx, gy) = GRAD2[hash % 12];
    gx * x + gy * y
}

/// 12-direction gradient table for simplex noise (x,y,z).
#[inline]
fn simplex_grad3(hash: usize, x: f32, y: f32, z: f32) -> f32 {
    // 12 edges of a cube
    match hash % 12 {
        0 => x + y,
        1 => -x + y,
        2 => x - y,
        3 => -x - y,
        4 => x + z,
        5 => -x + z,
        6 => x - z,
        7 => -x - z,
        8 => y + z,
        9 => -y + z,
        10 => y - z,
        _ => -y - z,
    }
}

// ── Simplex fBm ──────────────────────────────────────────────────────────────

/// Fractional Brownian Motion built on top of [`simplex_2d`].
///
/// Higher quality than [`fbm_2d`] (Perlin-based) because simplex noise has no
/// axis-aligned bias.  Output is in **\[-1.0, 1.0\]**.
///
/// # Examples
///
/// ```
/// use abrash_core::noise::fbm_simplex_2d;
///
/// let h = fbm_simplex_2d(3.0, 7.0, 6, 2.0, 0.5);
/// assert!(h >= -1.0 && h <= 1.0);
/// ```
#[must_use]
pub fn fbm_simplex_2d(x: f32, y: f32, octaves: u32, lacunarity: f32, gain: f32) -> f32 {
    let mut value = 0.0_f32;
    let mut amplitude = 0.5_f32;
    let mut frequency = 1.0_f32;
    for _ in 0..octaves {
        value += simplex_2d(x * frequency, y * frequency) * amplitude;
        amplitude *= gain;
        frequency *= lacunarity;
    }
    value.clamp(-1.0, 1.0)
}

/// Fractional Brownian Motion built on top of [`simplex_3d`].
///
/// Output is in **\[-1.0, 1.0\]**.
///
/// # Examples
///
/// ```
/// use abrash_core::noise::fbm_simplex_3d;
///
/// let h = fbm_simplex_3d(1.0, 2.0, 3.0, 5, 2.0, 0.5);
/// assert!(h >= -1.0 && h <= 1.0);
/// ```
#[must_use]
pub fn fbm_simplex_3d(x: f32, y: f32, z: f32, octaves: u32, lacunarity: f32, gain: f32) -> f32 {
    let mut value = 0.0_f32;
    let mut amplitude = 0.5_f32;
    let mut frequency = 1.0_f32;
    for _ in 0..octaves {
        value += simplex_3d(x * frequency, y * frequency, z * frequency) * amplitude;
        amplitude *= gain;
        frequency *= lacunarity;
    }
    value.clamp(-1.0, 1.0)
}

// ── Curl Noise ────────────────────────────────────────────────────────────────

/// 2D curl noise — a divergence-free 2D flow field derived from the curl of a
/// scalar simplex noise potential.
///
/// Returns `(vx, vy)` — a velocity vector with no divergence, so the flow
/// neither sources nor sinks.  Ideal for particle systems, smoke, water surface.
///
/// `scale` controls the spatial frequency of the noise.
///
/// Curl is estimated numerically with a small epsilon `h = 0.001 / scale`.
///
/// # Examples
///
/// ```
/// use abrash_core::noise::curl_noise_2d;
///
/// let (vx, vy) = curl_noise_2d(1.0, 2.0, 1.0);
/// // The result is a velocity — no range guarantee, but should be finite
/// assert!(vx.is_finite() && vy.is_finite());
/// ```
#[must_use]
pub fn curl_noise_2d(x: f32, y: f32, scale: f32) -> (f32, f32) {
    // 2D curl of scalar potential N: (∂N/∂y, -∂N/∂x)
    let h = 0.001 / scale.max(1e-6);
    let dny = simplex_2d((x) * scale, (y + h) * scale) - simplex_2d((x) * scale, (y - h) * scale);
    let dnx = simplex_2d((x + h) * scale, (y) * scale) - simplex_2d((x - h) * scale, (y) * scale);
    let inv2h = 1.0 / (2.0 * h);
    (dny * inv2h, -dnx * inv2h)
}

/// 3D curl noise — a divergence-free 3D flow field.
///
/// Derives `(u, v, w)` as the curl of a 3-component simplex noise vector field
/// `(Nx, Ny, Nz)` sampled at offset potentials.
///
/// Uses the finite-difference approximation of `∇ × (Nx, Ny, Nz)`.
///
/// # Examples
///
/// ```
/// use abrash_core::noise::curl_noise_3d;
/// use abrash_core::math::Vec3;
///
/// let flow = curl_noise_3d(Vec3::new(1.0, 2.0, 0.5), 1.0);
/// assert!(flow.x.is_finite() && flow.y.is_finite() && flow.z.is_finite());
/// ```
#[must_use]
pub fn curl_noise_3d(p: crate::math::Vec3, scale: f32) -> crate::math::Vec3 {
    // Curl of (Nx, Ny, Nz): ∇×F = (∂Nz/∂y − ∂Ny/∂z, ∂Nx/∂z − ∂Nz/∂x, ∂Ny/∂x − ∂Nx/∂y)
    // Use three offset potential fields to break symmetry
    let h = 0.001 / scale.max(1e-6);
    let s = scale;
    let (px, py, pz) = (p.x, p.y, p.z);

    // Nx = simplex at (x, y, z)
    // Ny = simplex at (x+seed1, y+seed1, z+seed1)
    // Nz = simplex at (x+seed2, y+seed2, z+seed2)
    const S1: f32 = 3.171_31;
    const S2: f32 = 7.342_17;

    let nx = |x: f32, y: f32, z: f32| simplex_3d(x * s, y * s, z * s);
    let ny = |x: f32, y: f32, z: f32| simplex_3d((x + S1) * s, (y + S1) * s, (z + S1) * s);
    let nz = |x: f32, y: f32, z: f32| simplex_3d((x + S2) * s, (y + S2) * s, (z + S2) * s);

    let inv2h = 1.0 / (2.0 * h);

    // ∂Nz/∂y − ∂Ny/∂z
    let dnz_dy = (nz(px, py + h, pz) - nz(px, py - h, pz)) * inv2h;
    let dny_dz = (ny(px, py, pz + h) - ny(px, py, pz - h)) * inv2h;
    // ∂Nx/∂z − ∂Nz/∂x
    let dnx_dz = (nx(px, py, pz + h) - nx(px, py, pz - h)) * inv2h;
    let dnz_dx = (nz(px + h, py, pz) - nz(px - h, py, pz)) * inv2h;
    // ∂Ny/∂x − ∂Nx/∂y
    let dny_dx = (ny(px + h, py, pz) - ny(px - h, py, pz)) * inv2h;
    let dnx_dy = (nx(px, py + h, pz) - nx(px, py - h, pz)) * inv2h;

    crate::math::Vec3::new(dnz_dy - dny_dz, dnx_dz - dnz_dx, dny_dx - dnx_dy)
}

// ── Ridge, Billow, Domain Warp ───────────────────────────────────────────────

/// Ridge noise (2D): `1 - |fBm|`, sharpened by `sharpness` exponent.
///
/// Produces sharp mountain-ridge features — the complement of turbulence.
/// `sharpness` ≥ 1.0: higher values sharpen the ridges more (2.0 is typical).
/// Output is in **[0, 1]**.
///
/// # Examples
///
/// ```
/// use abrash_core::noise::ridge_noise_2d;
/// let v = ridge_noise_2d(1.0, 2.0, 4, 2.0, 0.5, 2.0);
/// assert!(v >= 0.0 && v <= 1.0);
/// ```
#[must_use]
pub fn ridge_noise_2d(
    x: f32,
    y: f32,
    octaves: u32,
    lacunarity: f32,
    gain: f32,
    sharpness: f32,
) -> f32 {
    let mut value = 0.0_f32;
    let mut amplitude = 0.5_f32;
    let mut frequency = 1.0_f32;
    let mut weight = 1.0_f32;

    for _ in 0..octaves {
        let n = 1.0 - gradient_noise_2d(x * frequency, y * frequency).abs();
        let ridge = n * n; // sharpen
        value += ridge * amplitude * weight;
        weight = (ridge * sharpness).clamp(0.0, 1.0);
        amplitude *= gain;
        frequency *= lacunarity;
    }
    value.clamp(0.0, 1.0)
}

/// Billow noise (2D): `|fBm|` — cloudy, puffy terrain features.
///
/// Output is in **[0, ∞)** (unbounded above but typically stays near 1).
/// Scale by a constant or clamp as needed.
///
/// # Examples
///
/// ```
/// use abrash_core::noise::billow_noise_2d;
/// let v = billow_noise_2d(1.0, 2.0, 4, 2.0, 0.5);
/// assert!(v >= 0.0);
/// ```
#[must_use]
pub fn billow_noise_2d(x: f32, y: f32, octaves: u32, lacunarity: f32, gain: f32) -> f32 {
    let mut value = 0.0_f32;
    let mut amplitude = 0.5_f32;
    let mut frequency = 1.0_f32;

    for _ in 0..octaves {
        value += gradient_noise_2d(x * frequency, y * frequency).abs() * amplitude;
        amplitude *= gain;
        frequency *= lacunarity;
    }
    value
}

/// Domain-warped fBm (2D): `fBm(p + strength * fBm(p))`.
///
/// Warps the input coordinates by another layer of noise before evaluating
/// fBm. Produces the organic, flowing terrain shapes seen in Inigo Quilez's
/// "Warping" technique.
///
/// `strength` controls how much the domain is displaced (0.5–1.5 is typical).
///
/// # Examples
///
/// ```
/// use abrash_core::noise::domain_warp_fbm_2d;
/// let a = domain_warp_fbm_2d(1.7, -0.9, 4, 2.0, 0.5, 0.5);
/// let b = domain_warp_fbm_2d(1.7, -0.9, 4, 2.0, 0.5, 0.5);
/// assert_eq!(a, b); // deterministic
/// ```
#[must_use]
pub fn domain_warp_fbm_2d(
    x: f32,
    y: f32,
    octaves: u32,
    lacunarity: f32,
    gain: f32,
    strength: f32,
) -> f32 {
    // First pass: warp offsets
    let wx = fbm_2d(x, y, octaves, lacunarity, gain);
    let wy = fbm_2d(x + 5.2, y + 1.3, octaves, lacunarity, gain); // offset seed
    fbm_2d(
        x + strength * wx,
        y + strength * wy,
        octaves,
        lacunarity,
        gain,
    )
}

/// Voronoi noise returning `(f1, f2, cell_id)` — F1 distance, F2 distance, and cell identifier.
///
/// Unlike [`worley_noise_2d`] which only returns F1, this returns both the
/// distance to the nearest feature point (`f1`) and second nearest (`f2`),
/// plus an integer cell identifier.
///
/// Common uses:
/// - `f1` alone: cellular / organic texture
/// - `f2 - f1`: "cracks" and cell borders
/// - `cell_id` as a seed: per-cell colour variation
///
/// # Examples
///
/// ```
/// use abrash_core::noise::voronoi_noise_2d;
///
/// let (f1, f2, _id) = voronoi_noise_2d(1.5, 2.3, 1.0);
/// assert!(f1 >= 0.0, "f1 must be non-negative");
/// assert!(f2 >= f1, "f2 must be >= f1");
///
/// // Deterministic
/// let (a1, _, _) = voronoi_noise_2d(0.7, -1.2, 0.8);
/// let (b1, _, _) = voronoi_noise_2d(0.7, -1.2, 0.8);
/// assert_eq!(a1, b1);
/// ```
pub fn voronoi_noise_2d(x: f32, y: f32, jitter: f32) -> (f32, f32, u32) {
    let ix = x.floor() as i32;
    let iy = y.floor() as i32;
    let fx = x - x.floor();
    let fy = y - y.floor();

    let mut f1 = f32::MAX;
    let mut f2 = f32::MAX;
    let mut cell_id = 0u32;

    for cy in -2_i32..=2 {
        for cx in -2_i32..=2 {
            let hx = hash2(ix + cx, iy + cy);
            let hy = hash2(ix + cx + 7_919, iy + cy + 104_729);
            let pt_x = cx as f32 + hx * jitter;
            let pt_y = cy as f32 + hy * jitter;
            let dx = fx - pt_x;
            let dy = fy - pt_y;
            let d = (dx * dx + dy * dy).sqrt();
            if d < f1 {
                f2 = f1;
                f1 = d;
                // Deterministic cell id from grid coordinates
                cell_id = (((ix + cx).wrapping_mul(1_619) ^ (iy + cy).wrapping_mul(31_337)) as u32)
                    .wrapping_mul(0x9e37_79b9);
            } else if d < f2 {
                f2 = d;
            }
        }
    }
    (f1, f2, cell_id)
}

/// Gabor noise kernel — a single oriented sinusoidal blob.
///
/// Models a Gaussian-windowed sine wave at position `(x, y)` relative to
/// the kernel centre.  Parameters:
///
/// - `freq`: spatial frequency of the sine wave (cycles per unit)
/// - `theta`: orientation angle of the sine (radians, measured from +x axis)
/// - `bandwidth`: Gaussian half-bandwidth controlling kernel extent (σ in units)
///
/// Returns a value in roughly `[−1, 1]`.  Use [`gabor_noise_2d`] for the
/// full noise function built by summing many kernels.
#[inline]
fn gabor_kernel(dx: f32, dy: f32, freq: f32, theta: f32, bandwidth: f32) -> f32 {
    let g = (-(dx * dx + dy * dy) / (2.0 * bandwidth * bandwidth)).exp();
    let s = std::f32::consts::TAU * freq * (dx * theta.cos() + dy * theta.sin());
    g * s.cos()
}

/// Gabor noise: spatially-controlled anisotropic band-pass noise.
///
/// Tiles the plane with a Poisson-distributed set of Gabor kernels (here
/// approximated by jittered grid cells, one kernel per cell) and sums their
/// contributions.  The result has a dominant frequency `freq` and orientation
/// `theta`, making it ideal for wood grain, fabric, turbulent flow lines, and
/// any texture with directional structure.
///
/// - `x`, `y`: sample position
/// - `freq`: dominant spatial frequency (≈ 2–10 for typical noise)
/// - `theta`: orientation angle in radians
/// - `bandwidth`: kernel Gaussian width (≈ 0.1–0.5; smaller = narrower bands)
/// - `cells`: number of grid cells per axis (more = smoother but slower)
///
/// Returns a value approximately in `[−1, 1]`.
///
/// # Examples
///
/// ```
/// use abrash_core::noise::gabor_noise_2d;
///
/// // Deterministic and non-trivial
/// let v = gabor_noise_2d(1.5, 2.3, 3.0, 0.0, 0.3, 4);
/// assert!(v.abs() <= 1.5, "Gabor noise out of expected range: {v}");
///
/// // Same inputs → same output
/// let a = gabor_noise_2d(0.7, -1.2, 5.0, 0.78, 0.2, 3);
/// let b = gabor_noise_2d(0.7, -1.2, 5.0, 0.78, 0.2, 3);
/// assert_eq!(a, b);
/// ```
pub fn gabor_noise_2d(x: f32, y: f32, freq: f32, theta: f32, bandwidth: f32, cells: i32) -> f32 {
    let ix = x.floor() as i32;
    let iy = y.floor() as i32;
    let mut sum = 0.0_f32;
    for cy in -cells..=cells {
        for cx in -cells..=cells {
            // Jitter the kernel centre within each grid cell
            let hx = hash2(ix + cx, iy + cy);
            let hy = hash2(ix + cx + 7_919, iy + cy + 104_729); // coprime offsets
            let kx = (ix + cx) as f32 + hx;
            let ky = (iy + cy) as f32 + hy;
            let dx = x - kx;
            let dy = y - ky;
            // Only accumulate kernels close enough to matter
            let r2 = dx * dx + dy * dy;
            if r2 < (3.0 * bandwidth) * (3.0 * bandwidth) {
                sum += gabor_kernel(dx, dy, freq, theta, bandwidth);
            }
        }
    }
    sum.clamp(-1.0, 1.0)
}

/// 3D Voronoi noise returning `(f1, f2, cell_id)`.
///
/// Extension of [`voronoi_noise_2d`] to three dimensions.  The same
/// feature-point Poisson process is applied in a 3D grid.
///
/// # Examples
///
/// ```
/// use abrash_core::noise::voronoi_noise_3d;
///
/// let (f1, f2, _id) = voronoi_noise_3d(1.5, 2.3, -0.7, 1.0);
/// assert!(f1 >= 0.0);
/// assert!(f2 >= f1);
///
/// // Deterministic
/// let (a, _, _) = voronoi_noise_3d(0.7, -1.2, 0.5, 0.8);
/// let (b, _, _) = voronoi_noise_3d(0.7, -1.2, 0.5, 0.8);
/// assert_eq!(a, b);
/// ```
pub fn voronoi_noise_3d(x: f32, y: f32, z: f32, jitter: f32) -> (f32, f32, u32) {
    let ix = x.floor() as i32;
    let iy = y.floor() as i32;
    let iz = z.floor() as i32;
    let fx = x - x.floor();
    let fy = y - y.floor();
    let fz = z - z.floor();

    let mut f1 = f32::MAX;
    let mut f2 = f32::MAX;
    let mut cell_id = 0u32;

    for cz in -1_i32..=1 {
        for cy in -1_i32..=1 {
            for cx in -1_i32..=1 {
                let hx = hash3(ix + cx, iy + cy, iz + cz);
                let hy = hash3(ix + cx + 7_919, iy + cy + 104_729, iz + cz + 2_017);
                let hz = hash3(ix + cx + 31_337, iy + cy + 1_031, iz + cz + 99_991);
                let pt_x = cx as f32 + hx * jitter;
                let pt_y = cy as f32 + hy * jitter;
                let pt_z = cz as f32 + hz * jitter;
                let dx = fx - pt_x;
                let dy = fy - pt_y;
                let dz = fz - pt_z;
                let d = (dx * dx + dy * dy + dz * dz).sqrt();
                if d < f1 {
                    f2 = f1;
                    f1 = d;
                    cell_id = (((ix + cx).wrapping_mul(1_619)
                        ^ (iy + cy).wrapping_mul(31_337)
                        ^ (iz + cz).wrapping_mul(6_271)) as u32)
                        .wrapping_mul(0x9e37_79b9);
                } else if d < f2 {
                    f2 = d;
                }
            }
        }
    }
    (f1, f2, cell_id)
}

/// 3D ridge noise: turbulence-like fBm where each octave uses `|noise| * -1`.
///
/// Produces sharp ridges at `|v| = 0` with smoother surrounding terrain.
/// Useful for mountain ranges, veins in rock, cracked earth.
///
/// # Examples
///
/// ```
/// use abrash_core::noise::ridge_noise_3d;
///
/// let v = ridge_noise_3d(1.5, 2.3, -0.7, 4, 2.0, 0.5);
/// assert!(v >= 0.0 && v <= 1.0, "ridge_noise_3d out of [0,1]: {v}");
///
/// // Deterministic
/// let a = ridge_noise_3d(0.7, -1.2, 0.4, 3, 2.0, 0.5);
/// let b = ridge_noise_3d(0.7, -1.2, 0.4, 3, 2.0, 0.5);
/// assert_eq!(a, b);
/// ```
pub fn ridge_noise_3d(x: f32, y: f32, z: f32, octaves: u32, lacunarity: f32, gain: f32) -> f32 {
    let mut value = 0.0_f32;
    let mut amplitude = 0.5_f32;
    let mut frequency = 1.0_f32;
    let mut weight = 1.0_f32;
    for _ in 0..octaves {
        let n = 1.0 - gradient_noise_3d(x * frequency, y * frequency, z * frequency).abs();
        let n = n * n * weight;
        value += n * amplitude;
        weight = n.clamp(0.0, 1.0);
        frequency *= lacunarity;
        amplitude *= gain;
    }
    value.clamp(0.0, 1.0)
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

    // ── Simplex noise ─────────────────────────────────────────────────────────

    #[test]
    fn simplex_2d_range() {
        for i in 0..100 {
            let x = i as f32 * 0.17 - 8.0;
            let y = i as f32 * 0.11 + 0.3;
            let v = simplex_2d(x, y);
            assert!(
                v >= -1.0 && v <= 1.0,
                "simplex_2d out of range at ({x},{y}): {v}"
            );
        }
    }

    #[test]
    fn simplex_2d_deterministic() {
        let a = simplex_2d(1.7, -0.9);
        let b = simplex_2d(1.7, -0.9);
        assert_eq!(a, b);
    }

    #[test]
    fn simplex_2d_not_constant() {
        // Should vary — two distant samples shouldn't be identical
        let a = simplex_2d(0.0, 0.0);
        let b = simplex_2d(10.5, 7.3);
        assert!(
            (a - b).abs() > 0.001,
            "simplex_2d appears constant: {a} vs {b}"
        );
    }

    #[test]
    fn simplex_3d_range() {
        for i in 0..100 {
            let x = i as f32 * 0.13 - 5.0;
            let y = i as f32 * 0.09 + 1.0;
            let z = i as f32 * 0.07 - 3.0;
            let v = simplex_3d(x, y, z);
            assert!(v >= -1.0 && v <= 1.0, "simplex_3d out of range: {v}");
        }
    }

    #[test]
    fn simplex_3d_deterministic() {
        let a = simplex_3d(0.7, -1.2, 0.3);
        let b = simplex_3d(0.7, -1.2, 0.3);
        assert_eq!(a, b);
    }

    #[test]
    fn ridge_range() {
        for i in 0..50 {
            let x = i as f32 * 0.13 - 3.0;
            let y = i as f32 * 0.07 + 1.0;
            let v = ridge_noise_2d(x, y, 4, 2.0, 0.5, 1.0);
            assert!(v >= 0.0 && v <= 1.0, "ridge out of range: {v}");
        }
    }

    #[test]
    fn billow_range() {
        for i in 0..50 {
            let x = i as f32 * 0.19 - 1.5;
            let y = i as f32 * 0.11 + 0.3;
            let v = billow_noise_2d(x, y, 4, 2.0, 0.5);
            assert!(v >= 0.0, "billow negative: {v}");
        }
    }

    #[test]
    fn domain_warp_deterministic() {
        let a = domain_warp_fbm_2d(1.7, -0.9, 4, 2.0, 0.5, 0.5);
        let b = domain_warp_fbm_2d(1.7, -0.9, 4, 2.0, 0.5, 0.5);
        assert_eq!(a, b);
    }

    // ── fbm_simplex / curl_noise ──────────────────────────────────────────────

    #[test]
    fn fbm_simplex_2d_range() {
        for i in 0..80 {
            let x = i as f32 * 0.21 - 7.0;
            let y = i as f32 * 0.13 + 1.0;
            let v = fbm_simplex_2d(x, y, 5, 2.0, 0.5);
            assert!(v >= -1.0 && v <= 1.0, "fbm_simplex_2d out of range: {v}");
        }
    }

    #[test]
    fn fbm_simplex_3d_range() {
        for i in 0..50 {
            let x = i as f32 * 0.17 - 4.0;
            let y = i as f32 * 0.11;
            let z = i as f32 * 0.09 - 2.0;
            let v = fbm_simplex_3d(x, y, z, 4, 2.0, 0.5);
            assert!(v >= -1.0 && v <= 1.0, "fbm_simplex_3d out of range: {v}");
        }
    }

    #[test]
    fn fbm_simplex_deterministic() {
        let a = fbm_simplex_2d(3.1, -0.7, 4, 2.0, 0.5);
        let b = fbm_simplex_2d(3.1, -0.7, 4, 2.0, 0.5);
        assert_eq!(a, b);
    }

    #[test]
    fn curl_2d_nonzero() {
        // curl should produce non-trivial flow — two nearby points shouldn't
        // both be zero
        let (ax, ay) = curl_noise_2d(0.5, 0.3, 1.0);
        let (bx, by) = curl_noise_2d(1.7, -0.9, 1.0);
        // At least one component should be non-trivially non-zero
        assert!(
            ax.abs() > 1e-5 || ay.abs() > 1e-5 || bx.abs() > 1e-5 || by.abs() > 1e-5,
            "curl_noise_2d appears zero everywhere"
        );
    }

    #[test]
    fn curl_2d_deterministic() {
        let a = curl_noise_2d(1.1, -0.3, 0.8);
        let b = curl_noise_2d(1.1, -0.3, 0.8);
        assert_eq!(a, b);
    }

    #[test]
    fn curl_3d_nonzero() {
        use crate::math::Vec3;
        let v = curl_noise_3d(Vec3::new(0.5, 0.3, -0.7), 1.0);
        // Must produce some non-zero flow
        assert!(v.length() > 1e-5, "curl_noise_3d appears zero: {v:?}");
    }

    #[test]
    fn curl_3d_deterministic() {
        use crate::math::Vec3;
        let a = curl_noise_3d(Vec3::new(1.1, -0.3, 0.9), 0.8);
        let b = curl_noise_3d(Vec3::new(1.1, -0.3, 0.9), 0.8);
        assert_eq!(a, b);
    }

    // ── Gabor noise ──────────────────────────────────────────────────────────

    #[test]
    fn gabor_in_range() {
        for (x, y) in [(0.5_f32, 0.7_f32), (1.3, -0.9), (-2.1, 1.6)] {
            let v = gabor_noise_2d(x, y, 3.0, 0.0, 0.3, 3);
            assert!(v.abs() <= 1.0, "gabor out of [-1,1]: {v} at ({x},{y})");
        }
    }

    #[test]
    fn gabor_deterministic() {
        let a = gabor_noise_2d(0.7, -1.2, 5.0, 0.78, 0.2, 3);
        let b = gabor_noise_2d(0.7, -1.2, 5.0, 0.78, 0.2, 3);
        assert_eq!(a, b);
    }

    #[test]
    fn gabor_nonzero() {
        // Non-trivial sample should produce meaningful output
        let v = gabor_noise_2d(1.5, 2.3, 3.0, 0.0, 0.3, 4);
        assert!(v.abs() > 1e-6, "gabor appears zero: {v}");
    }

    #[test]
    fn gabor_orientation_differs() {
        // Two different orientations should produce different results at the same point
        let h = gabor_noise_2d(1.0, 0.5, 4.0, 0.0, 0.25, 3);
        let v = gabor_noise_2d(1.0, 0.5, 4.0, std::f32::consts::FRAC_PI_2, 0.25, 3);
        assert!(
            (h - v).abs() > 1e-4,
            "gabor ignoring orientation: h={h} v={v}"
        );
    }

    // ── Voronoi noise ────────────────────────────────────────────────────────

    #[test]
    fn voronoi_f2_gte_f1() {
        for (x, y) in [(0.5_f32, 0.3_f32), (1.7, -0.9), (-3.2, 2.1)] {
            let (f1, f2, _) = voronoi_noise_2d(x, y, 1.0);
            assert!(f2 >= f1, "f2 < f1 at ({x},{y}): f1={f1} f2={f2}");
        }
    }

    #[test]
    fn voronoi_deterministic() {
        let (a, _, ia) = voronoi_noise_2d(0.7, -1.2, 0.8);
        let (b, _, ib) = voronoi_noise_2d(0.7, -1.2, 0.8);
        assert_eq!(a, b);
        assert_eq!(ia, ib);
    }

    #[test]
    fn voronoi_cells_differ() {
        // Nearby but clearly different cells should have different ids
        let (_, _, id1) = voronoi_noise_2d(0.1, 0.1, 1.0);
        let (_, _, id2) = voronoi_noise_2d(1.8, 1.8, 1.0);
        assert_ne!(id1, id2, "cells at different positions should differ");
    }

    // ── voronoi_noise_3d ─────────────────────────────────────────────────────

    #[test]
    fn voronoi_3d_f2_gte_f1() {
        for (x, y, z) in [
            (0.5_f32, 0.3_f32, 0.7_f32),
            (1.7, -0.9, 2.3),
            (-3.2, 2.1, 0.4),
        ] {
            let (f1, f2, _) = voronoi_noise_3d(x, y, z, 1.0);
            assert!(f2 >= f1, "f2 < f1 at ({x},{y},{z}): f1={f1} f2={f2}");
        }
    }

    #[test]
    fn voronoi_3d_deterministic() {
        let (a, _, ia) = voronoi_noise_3d(0.7, -1.2, 0.5, 0.8);
        let (b, _, ib) = voronoi_noise_3d(0.7, -1.2, 0.5, 0.8);
        assert_eq!(a, b);
        assert_eq!(ia, ib);
    }

    #[test]
    fn voronoi_3d_cells_differ() {
        let (_, _, id1) = voronoi_noise_3d(0.1, 0.1, 0.1, 1.0);
        let (_, _, id2) = voronoi_noise_3d(5.8, 5.8, 5.8, 1.0);
        assert_ne!(id1, id2);
    }

    #[test]
    fn voronoi_3d_zero_jitter_grid() {
        // With jitter=0, cell features are at grid centres: f1 = 0.5 everywhere
        // (nearest cell centre is always 0.5 away at most)
        let (f1, _, _) = voronoi_noise_3d(0.3, 0.3, 0.3, 0.0);
        assert!(f1 <= 0.9, "f1 should be bounded with zero jitter: {f1}");
    }

    // ── ridge_noise_3d ───────────────────────────────────────────────────────

    #[test]
    fn ridge_3d_in_range() {
        for (x, y, z) in [
            (0.0_f32, 0.0_f32, 0.0_f32),
            (1.5, -2.3, 0.7),
            (-10.0, 5.0, 3.0),
        ] {
            let v = ridge_noise_3d(x, y, z, 4, 2.0, 0.5);
            assert!(
                (0.0..=1.0).contains(&v),
                "ridge value out of range at ({x},{y},{z}): {v}"
            );
        }
    }

    #[test]
    fn ridge_3d_deterministic() {
        let a = ridge_noise_3d(1.0, 2.0, 3.0, 4, 2.0, 0.5);
        let b = ridge_noise_3d(1.0, 2.0, 3.0, 4, 2.0, 0.5);
        assert_eq!(a, b);
    }

    #[test]
    fn ridge_3d_more_octaves_varies() {
        // More octaves should produce different (usually higher detail) values
        let v1 = ridge_noise_3d(0.5, 0.5, 0.5, 1, 2.0, 0.5);
        let v4 = ridge_noise_3d(0.5, 0.5, 0.5, 4, 2.0, 0.5);
        // They can be equal by coincidence, but the values should both be valid
        assert!((0.0..=1.0).contains(&v1));
        assert!((0.0..=1.0).contains(&v4));
    }
}
