# Bolt's Journal

**[Performance Optimization: Unchecked Rasterizer Access]**
**Learning:** Removing bounds checks (`test_and_set` -> `test_and_set_unchecked`) in the single-pixel/empty-span path of the rasterizer yielded a ~20% performance improvement for small triangles. This path is hit frequently for sub-pixel or thin geometry.
**Action:** Look for other "checked by logic" hot paths where `unsafe` unchecked access can be justified by loop invariants.

**[Optimization Trap: Intrinsics vs Inlining]**
**Learning:** Attempting to optimize `Vec3::normalize` with `unsafe` SIMD intrinsics (`rsqrtss`) caused precision regressions in tests and violated safety policies. However, simply adding `#[inline]` to `Vec3::length` provided a ~50% speedup (11.3ns -> 5.7ns), matching the performance of the unsafe intrinsic version without the downsides.
**Action:** Always profile function call overhead and inlining (`#[inline]`) before resorting to `unsafe` intrinsics. Precision loss from fast-math approximations can break regression tests.
**[Performance Optimization: Rayon Collection]**\n**Learning:** Replacing  on Rayon iterators followed by  with a direct call to  eliminates intermediate heap allocations and memory copying entirely without losing thread safety.\n**Action:** Use  for all parallel vector aggregations.
**[Performance Optimization: Rayon Collection]**
**Learning:** Replacing `.collect::<Vec<_>>()` on Rayon iterators followed by `.extend()` with a direct call to `Vec::par_extend()` eliminates intermediate heap allocations and memory copying entirely without losing thread safety.
**Action:** Use `.par_extend()` for all parallel vector aggregations.
