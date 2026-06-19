//! Procedural Texture Generation Module
//!
//! Provides functions to generate textures algorithmically.

use crate::texture::Texture;
use crate::utils::XorShift32;
use abrash_core::math::fast_sin_cos;

const PLASMA_LUT: [u32; 256] = [
    4_278_249_408,
    4_278_445_763,
    4_278_642_117,
    4_278_838_215,
    4_279_034_568,
    4_279_230_666,
    4_279_427_020,
    4_279_623_118,
    4_279_885_008,
    4_280_081_106,
    4_280_277_460,
    4_280_473_557,
    4_280_669_655,
    4_280_865_753,
    4_281_062_106,
    4_281_258_204,
    4_281_454_301,
    4_281_715_935,
    4_281_912_032,
    4_282_108_130,
    4_282_304_227,
    4_282_500_581,
    4_282_696_422,
    4_282_892_519,
    4_283_088_617,
    4_283_284_714,
    4_283_480_811,
    4_283_676_908,
    4_283_873_005,
    4_284_069_103,
    4_284_264_944,
    4_284_461_041,
    4_284_591_602,
    4_284_787_699,
    4_284_983_540,
    4_285_179_636,
    4_285_375_477,
    4_285_571_574,
    4_285_767_671,
    4_285_897_976,
    4_286_094_072,
    4_286_289_913,
    4_286_486_010,
    4_286_616_314,
    4_286_812_155,
    4_287_008_251,
    4_287_138_556,
    4_287_334_652,
    4_287_530_493,
    4_287_660_797,
    4_287_856_637,
    4_287_987_198,
    4_288_183_038,
    4_288_313_342,
    4_288_509_182,
    4_288_639_742,
    4_288_835_582,
    4_288_965_886,
    4_289_161_726,
    4_289_292_030,
    4_289_422_334,
    4_289_618_174,
    4_289_748_734,
    4_289_879_038,
    4_290_009_342,
    4_290_205_182,
    4_290_335_485,
    4_290_465_789,
    4_290_596_092,
    4_290_726_396,
    4_290_856_700,
    4_290_987_003,
    4_291_117_307,
    4_291_247_610,
    4_291_377_913,
    4_291_508_217,
    4_291_638_264,
    4_291_768_567,
    4_291_898_871,
    4_291_963_638,
    4_292_093_941,
    4_292_224_244,
    4_292_289_011,
    4_292_419_314,
    4_292_549_617,
    4_292_614_384,
    4_292_744_431,
    4_292_809_198,
    4_292_939_501,
    4_293_004_268,
    4_293_134_571,
    4_293_199_337,
    4_293_264_104,
    4_293_394_919,
    4_293_461_222,
    4_293_527_524,
    4_293_659_363,
    4_293_725_665,
    4_293_791_968,
    4_293_858_270,
    4_293_924_829,
    4_293_991_131,
    4_294_057_434,
    4_294_123_736,
    4_294_190_038,
    4_294_256_341,
    4_294_322_643,
    4_294_323_409,
    4_294_389_711,
    4_294_456_013,
    4_294_522_572,
    4_294_523_338,
    4_294_589_640,
    4_294_590_406,
    4_294_656_708,
    4_294_657_474,
    4_294_723_776,
    4_294_724_542,
    4_294_790_844,
    4_294_791_609,
    4_294_792_375,
    4_294_858_677,
    4_294_859_443,
    4_294_860_209,
    4_294_860_974,
    4_294_861_484,
    4_294_862_250,
    4_294_863_015,
    4_294_863_781,
    4_294_864_547,
    4_294_865_312,
    4_294_866_078,
    4_294_866_587,
    4_294_867_353,
    4_294_868_118,
    4_294_803_348,
    4_294_803_857,
    4_294_804_623,
    4_294_739_852,
    4_294_740_361,
    4_294_675_591,
    4_294_676_356,
    4_294_611_329,
    4_294_612_095,
    4_294_547_068,
    4_294_547_833,
    4_294_482_806,
    4_294_418_035,
    4_294_353_009,
    4_294_353_774,
    4_294_288_747,
    4_294_223_720,
    4_294_158_949,
    4_294_093_922,
    4_294_028_895,
    4_293_964_125,
    4_293_899_098,
    4_293_834_071,
    4_293_769_044,
    4_293_704_017,
    4_293_573_710,
    4_293_508_683,
    4_293_443_656,
    4_293_313_093,
    4_293_248_066,
    4_293_183_039,
    4_293_052_476,
    4_292_987_449,
    4_292_856_886,
    4_292_791_602,
    4_292_661_039,
    4_292_596_012,
    4_292_465_449,
    4_292_334_886,
    4_292_269_603,
    4_292_139_040,
    4_292_008_477,
    4_291_943_194,
    4_291_812_630,
    4_291_681_811,
    4_291_551_248,
    4_291_420_429,
    4_291_289_866,
    4_291_159_047,
    4_291_028_484,
    4_290_897_665,
    4_290_766_850,
    4_290_636_293,
    4_290_505_480,
    4_290_374_667,
    4_290_243_854,
    4_290_047_505,
    4_289_916_692,
    4_289_785_880,
    4_289_655_067,
    4_289_458_718,
    4_289_327_905,
    4_289_197_092,
    4_289_000_743,
    4_288_869_930,
    4_288_673_581,
    4_288_542_512,
    4_288_346_164,
    4_288_215_351,
    4_288_018_746,
    4_287_887_933,
    4_287_691_584,
    4_287_560_515,
    4_287_364_166,
    4_287_167_561,
    4_287_036_748,
    4_286_840_143,
    4_286_643_538,
    4_286_512_725,
    4_286_316_120,
    4_286_119_515,
    4_285_922_910,
    4_285_791_841,
    4_285_595_235,
    4_285_398_630,
    4_285_202_025,
    4_285_005_420,
    4_284_808_815,
    4_284_612_210,
    4_284_481_140,
    4_284_284_535,
    4_284_087_930,
    4_283_891_325,
    4_283_694_463,
    4_283_497_858,
    4_283_301_253,
    4_283_104_392,
    4_282_907_786,
    4_282_710_925,
    4_282_514_319,
    4_282_317_458,
    4_282_120_853,
    4_281_923_991,
    4_281_727_130,
    4_281_464_988,
    4_281_268_127,
    4_281_071_265,
    4_280_874_403,
    4_280_677_542,
    4_280_480_936,
    4_280_284_075,
    4_280_087_213,
    4_279_890_351,
    4_279_627_953,
    4_279_431_092,
    4_279_233_974,
    4_279_037_112,
    4_278_840_250,
    4_278_643_388,
    4_278_446_526,
    4_278_249_408,
];

/// Generates a classic XOR texture.
///
/// # Errors
/// Returns an error if the texture dimensions are invalid.
///
/// # Examples
///
/// ```
/// use abrash_render::procedural::xor_pattern;
///
/// let tex = xor_pattern(32, 32).unwrap();
/// assert_eq!(tex.width(), 32);
/// assert_eq!(tex.height(), 32);
/// ```
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
/// # Errors
/// Returns an error if the texture dimensions are invalid or `cell_size` is 0.
///
/// # Examples
///
/// ```
/// use abrash_render::procedural::grid_pattern;
///
/// let tex = grid_pattern(32, 32, 8, 0xFFFF_FFFF, 0xFF00_0000).unwrap();
/// assert_eq!(tex.width(), 32);
/// assert_eq!(tex.height(), 32);
/// ```
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
/// # Errors
/// Returns an error if the texture dimensions are invalid.
///
/// # Examples
///
/// ```
/// use abrash_render::procedural::white_noise;
///
/// let tex = white_noise(32, 32, 12345).unwrap();
/// assert_eq!(tex.width(), 32);
/// assert_eq!(tex.height(), 32);
/// ```
pub fn white_noise(width: u32, height: u32, seed: u32) -> Result<Texture, &'static str> {
    let mut tex = Texture::new(width, height)?;
    let mut rng = XorShift32::new(seed);

    for y in 0..height {
        for x in 0..width {
            let v = (rng.next_u32() & 0xFF) as u8;
            let color = 0xFF00_0000 | (u32::from(v) << 16) | (u32::from(v) << 8) | u32::from(v);
            tex.set_pixel(x, y, color);
        }
    }
    Ok(tex)
}

/// Generates a plasma effect.
///
/// # Errors
/// Returns an error if the texture dimensions are invalid.
///
/// # Examples
///
/// ```
/// use abrash_render::procedural::plasma;
///
/// let tex = plasma(32, 32).unwrap();
/// assert_eq!(tex.width(), 32);
/// assert_eq!(tex.height(), 32);
/// ```
pub fn plasma(width: u32, height: u32) -> Result<Texture, &'static str> {
    let mut tex = Texture::new(width, height)?;

    // We can elide the bounds checks by mapping directly to the texture's inner buffer
    // if we want to be even faster, but simple sequential index math is already much better.
    let w = width as usize;
    let h = height as usize;

    // Precompute sine values for horizontal and vertical components
    let mut u_sin = std::vec::Vec::with_capacity(w);
    let mut u_sin_2 = std::vec::Vec::with_capacity(w);

    for x in 0..width {
        let u = x as f32;
        let (s1, _) = fast_sin_cos(u * 0.1);
        u_sin.push(s1);
        u_sin_2.push(u * u);
    }

    let mut v_sin = std::vec::Vec::with_capacity(h);
    let mut v_sin_2 = std::vec::Vec::with_capacity(h);
    for y in 0..height {
        let v = y as f32;
        let (s1, _) = fast_sin_cos(v * 0.1);
        v_sin.push(s1);
        v_sin_2.push(v * v);
    }

    let mut u_add_v_sin = std::vec::Vec::with_capacity(w + h);
    for i in 0..(width + height) {
        let (s1, _) = fast_sin_cos((i as f32) * 0.1);
        u_add_v_sin.push(s1);
    }

    // Precompute sqrt(u^2 + v^2)
    // We only need to compute it once for each unique pair. Since we compute for all x and y,
    // we can precalculate the entire 2D array and map to a 1D vector.
    let mut dist_sin = std::vec::Vec::with_capacity(w * h);
    for y in 0..h {
        let v2 = v_sin_2[y];
        for x in 0..w {
            let u2 = u_sin_2[x];
            let (s1, _) = fast_sin_cos((u2 + v2).sqrt() * 0.1);
            dist_sin.push(s1);
        }
    }

    // Direct memory access is safe here because we iterate exactly w * h times
    // which exactly matches the texture size.
    let pixels = tex.pixels.as_mut_slice();

    // Avoid explicit manual chunk tracking if we can use get_unchecked safely.
    for y in 0..h {
        let v_val = v_sin[y];
        let y_offset = y * w;

        let row_start = y * w;
        let row = &mut pixels[row_start..row_start + w];

        for x in 0..w {
            // Because w, h bounds are exactly the array bounds, get_unchecked on row slices elides completely.
            let u_val = unsafe { *u_sin.get_unchecked(x) };

            let v3 = unsafe { *u_add_v_sin.get_unchecked(x + y) };
            let v4 = unsafe { *dist_sin.get_unchecked(y_offset + x) };

            let val = u_val + v_val + v3 + v4; // -4 to 4

            // Map directly to 0-255 range
            // (val * 0.25 + 1.0) * 127.5
            // = val * 31.875 + 127.5
            let float_idx = val.mul_add(31.875, 127.5);

            // Because float to int inherently handles bounds on safe casts, we can use unsafe float to int
            // or fast clamping
            let idx = float_idx as usize;

            // Since idx bounds are bounded by math clamp before cast, we can uncheck safely
            let lut_val = unsafe { *PLASMA_LUT.get_unchecked(idx.min(255)) };
            unsafe { *row.get_unchecked_mut(x) = lut_val };
        }
    }
    Ok(tex)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_grid_pattern() {
        let tex = grid_pattern(10, 10, 5, 0xFFFFFFFF, 0xFF000000).unwrap();
        assert_eq!(tex.width(), 10);
        assert_eq!(tex.height(), 10);
    }

    #[test]
    fn test_white_noise() {
        let tex = white_noise(10, 10, 12345).unwrap();
        assert_eq!(tex.width(), 10);
        assert_eq!(tex.height(), 10);
    }

    #[test]
    fn test_plasma() {
        let tex = plasma(10, 10).unwrap();
        assert_eq!(tex.width(), 10);
        assert_eq!(tex.height(), 10);
    }
}
