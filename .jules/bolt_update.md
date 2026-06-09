**[Heat Vision Inline Integer Math Optimization]**
**Learning:** In the `apply_heat_vision` post-processing effect, using a pre-calculated 1024-entry LUT and `_mm256_i32gather_epi32` instructions requires memory indirection that limits throughput. By replacing the LUT entirely with pure SIMD scaled integer math and conditional blending (`_mm256_blendv_epi8`), memory loads are eliminated in favor of ALU operations.
**Action:** Replace `LUT` arrays and `gather` calls with inline conditional arithmetic using scaling and `blendv_epi8`, resulting in a ~10-17% speedup.
