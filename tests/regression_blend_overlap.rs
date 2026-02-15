
use abrash::texture::blend_four_way;

#[test]
fn test_blend_four_way_corruption() {
    // R varies, B is 0.
    // P0: Red=0, Blue=0.
    // P1: Red=255, Blue=0.
    // Weights: 0.5 (128), 0.5 (128).
    // Effective weights w0=32768, w1=32768.
    // sum(R*W) = 0 + 255*32768 = 8355840 = 0x7F8000.
    // Expected Red: 0x7F (127).
    // Fractional Red: 0x8000.
    // Expected Blue: 0.

    // In buggy impl:
    // acc_rb = sum(B*W) + (sum(R*W) << 16).
    // acc_rb = 0 + (0x7F8000 << 16) = 0x7F80000000.
    // result = acc_rb >> 16 = 0x7F8000.
    // Extract Blue: (result) & 0xFF = 0x00.
    // Blue is 0. Correct.

    // Wait, my manual trace suggested corruption.
    // Corruption happens if fractional part lands in Blue's byte.
    // result = Blue_Final + sum(R*W).
    // result = 0 + 0x7F8000 = 0x7F8000.
    // Blue byte is 0x00.
    // It seems "Fractional Red" is higher up than I thought?
    // sum(R*W) is R_accum.
    // R_accum has "integer part" (upper 8 bits) and "fractional part" (lower 16 bits).
    // 0x7F is integer. 0x8000 is fractional.
    // Blue is at bits 0-7 of the result.
    // Fractional red is bits 0-15 of sum(R*W).
    // So `0x8000` corresponds to bit 15 set.
    // Bits 0-7 are `0x00`.
    // So Blue is NOT corrupted in this specific case (0x8000).

    // We need fractional red to have bits 0-7 set.
    // e.g. sum(R*W) ends in ...01.
    // We need odd sum.
    // 255 * 129?
    // w0 = 129*129? No, weights sum to 65536.
    // But individual w_i can be anything.
    // Let's use `wx=1`, `wy=1`.
    // w11 = 1*1 = 1.
    // w00 = 255*255 = 65025.
    // P0 (w00) has R=0.
    // P3 (w11) has R=255.
    // sum(R*W) = 0 + 255*1 = 255 = 0xFF.
    // acc_rb >> 16 will contain 255 (0xFF).
    // This 0xFF is in the Blue slot!
    // So Blue becomes 255!
    // Correct Blue should be 0.

    let black = 0xFF000000;
    let red = 0xFFFF0000;

    // Top-Left (weight ~1.0): Black
    // Bottom-Right (weight ~0.0): Red
    // wx=1, wy=1.
    // w00 = 255*255 = 65025.
    // w11 = 1.
    // Contribution from Red pixel is small but non-zero.
    // R contribution = 255 * 1 = 255.
    // This 255 sits in the lower bits of R-accumulator.
    // Because of overlap, it appears as 255 in Blue channel of result.

    let res = blend_four_way(black, black, black, red, 1, 1);

    // Expected: Red ~ 0. Blue = 0.
    let b = res & 0xFF;
    assert_eq!(b, 0, "Blue channel corrupted! Got {}", b);
}
