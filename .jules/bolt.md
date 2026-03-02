# Bolt's Journal

**[Performance Optimization: Unchecked Rasterizer Access]**
**Learning:** Removing bounds checks (`test_and_set` -> `test_and_set_unchecked`) in the single-pixel/empty-span path of the rasterizer yielded a ~20% performance improvement for small triangles. This path is hit frequently for sub-pixel or thin geometry.
**Action:** Look for other "checked by logic" hot paths where `unsafe` unchecked access can be justified by loop invariants.

**[Optimization Trap: Intrinsics vs Inlining]**
**Learning:** Attempting to optimize `Vec3::normalize` with `unsafe` SIMD intrinsics (`rsqrtss`) caused precision regressions in tests and violated safety policies. However, simply adding `#[inline]` to `Vec3::length` provided a ~50% speedup (11.3ns -> 5.7ns), matching the performance of the unsafe intrinsic version without the downsides.
**Action:** Always profile function call overhead and inlining (`#[inline]`) before resorting to `unsafe` intrinsics. Precision loss from fast-math approximations can break regression tests.

**[Performance Optimization: Pre-allocation in loops]**
**Learning:** In heavily looping setup functions like `Cloth::new`, using `Vec::new()` causes multiple reallocations which hurts performance and causes fragmentation. By mathematically calculating the exact capacity needed upfront, we can use `Vec::with_capacity()` to achieve zero-cost allocation on setup.
**Action:** Always calculate and use exact capacities for predictable, nested loops generating data.

**[Performance Optimization: Lens Distortion SIMD]**
**Learning:** Implemented an AVX2 vectorized version of a radial lens distortion effect. By processing 8 pixels concurrently and using gather instructions (`_mm256_i32gather_epi32`), performance significantly improved over the scalar loop.
**Action:** Vectorize independent per-pixel operations.
