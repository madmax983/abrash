import re

with open('crates/abrash-core/src/color.rs', 'r') as f:
    content = f.read()

new_fn = """
    /// Blends two raw ARGB `u32` colors using SWAR (SIMD Within A Register) integer math.
    ///
    /// This is significantly faster than converting to `Color` floats, performing
    /// float math, and converting back to `u32`. It avoids unpacking all 4 channels
    /// by computing Red and Blue simultaneously in a single 32-bit register.
    ///
    /// Both colors should be in **straight** (non-premultiplied) alpha.
    #[must_use]
    #[inline]
    pub fn blend_over_u32_swar(src: u32, dst: u32) -> u32 {
        let alpha = src >> 24;
        if alpha == 0 {
            return dst;
        }
        if alpha == 255 {
            return src;
        }

        let inv_alpha = 255 - alpha;

        // Extract G and Alpha from src and dst
        let src_ag = (src >> 8) & 0x00FF_00FF;
        let dst_ag = (dst >> 8) & 0x00FF_00FF;

        // Extract R and B from src and dst
        let src_rb = src & 0x00FF_00FF;
        let dst_rb = dst & 0x00FF_00FF;

        // Blend R and B together (SWAR)
        let out_rb = ((src_rb * alpha + dst_rb * inv_alpha) >> 8) & 0x00FF_00FF;

        // Blend Alpha and G together
        let out_ag = ((src_ag * alpha + dst_ag * inv_alpha) >> 8) & 0x00FF_00FF;

        (out_ag << 8) | out_rb
    }
"""

content = content.replace("    pub fn blend_over(src: Self, dst: Self) -> Self {", new_fn + "\n    pub fn blend_over(src: Self, dst: Self) -> Self {")

test_fn = """
    #[test]
    fn test_blend_over_swar() {
        let src_u32 = 0x80FF_0000; // Semi-transparent Red
        let dst_u32 = 0xFF00_FF00; // Opaque Green

        let src = Color::from_argb_u32(src_u32);
        let dst = Color::from_argb_u32(dst_u32);

        let float_res = Color::blend_over(src, dst).to_argb_u32();
        let swar_res = Color::blend_over_u32_swar(src_u32, dst_u32);

        let r_f = (float_res >> 16) & 0xFF;
        let r_s = (swar_res >> 16) & 0xFF;
        assert!(r_f.abs_diff(r_s) <= 1, "R mismatch: float {}, swar {}", r_f, r_s);

        let g_f = (float_res >> 8) & 0xFF;
        let g_s = (swar_res >> 8) & 0xFF;
        assert!(g_f.abs_diff(g_s) <= 1, "G mismatch: float {}, swar {}", g_f, g_s);

        let b_f = float_res & 0xFF;
        let b_s = swar_res & 0xFF;
        assert!(b_f.abs_diff(b_s) <= 1, "B mismatch: float {}, swar {}", b_f, b_s);
    }
"""
content = content.replace("fn test_blend_over() {", test_fn + "\n    #[test]\n    fn test_blend_over() {")

with open('crates/abrash-core/src/color.rs', 'w') as f:
    f.write(content)
