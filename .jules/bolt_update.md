**[Heat Vision Infinity Check Optimization]**
**Learning:** In hot rendering scalar fallback loops, replacing floating-point positive infinity equality checks (`depth == f32::INFINITY`) with integer bitwise checks (`depth.to_bits() == 0x7F80_0000`) avoids slow floating-point comparisons and improves performance.
**Action:** Replace `f32::INFINITY` equality checks with `to_bits() == 0x7F80_0000` in per-pixel rendering loops and add explanatory comments to prevent magic numbers.
