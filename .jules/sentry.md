# Sentry's Journal

## [Framebuffer `clear_rect` Out-of-Bounds]
**Learning:** The `clear_rect` function in `Framebuffer` panicked when arguments were fully out of bounds or negatively out of bounds due to incorrect scaling of the start/end slices, as `y.saturating_add` on large heights could be smaller than expected. Also, `clamp` was useful to enforce proper boundaries.
**Action:** When working with grid buffers and arbitrary rectangular boundaries, use `clamp(0, max_size)` for correct region intersection, ensuring `start <= end` before looping over the rows.
## [Texture `generate_mipmaps` Unwrap Panic]
**Learning:** Using `.unwrap()` on dynamic collections like `mips.last()` within algorithms like `generate_mipmaps` poses a panic risk if the collection state diverges (e.g., cleared before generation). It's crucial to substitute risky fallback unwraps with safer alternatives.
**Action:** Replace risky fallback `.unwrap()` calls on slices or iterators (e.g., `mips.last().unwrap()`) with `.unwrap_or(&default_value)` to safely handle empty collections without explicit if/else branching.
## [Raytracer `unwrap()` Panic on Object Hit]
**Learning:** In raytracing logic where a hit is guaranteed to correspond to a scene object, managing `closest_hit: Option<Hit>` and `hit_obj: Option<&SceneObject>` as separate variables creates an implicit synchronization dependency and a dangerous `unwrap()` panic point when extracting the object later.
**Action:** Group synchronized optional state (like a raycast hit and its intersected object) into a single `Option<(Hit, &SceneObject)>` tuple. This enforces synchronization at the type level and allows safe destructuring without risky `unwrap()` calls.
**[Heat Vision Graceful Degradation]**
**Learning:** `f32::clamp(min, max)` on a value that is `f32::NAN` does not panic; it simply returns `NaN`. Furthermore, casting `f32::NAN` (e.g. `(t * 255.0) as u32`) yields `0`. When auditing rendering or math logic for panics, recognize that `NaN` values will often silently propagate or cast to `0` rather than causing an explicit crash, providing unexpected default values instead of fatal errors.
**Action:** When testing float boundaries, explicitly write tests that push `f32::MIN` and `f32::MAX` to simulate out-of-bounds inputs rather than solely relying on `f32::NAN` to trigger float panics.
**[Coverage Gap: SIMD Fast-Path Triggers]**
**Learning:** `unsafe` SIMD fast-paths that operate on specific chunk sizes (e.g., `while i + 8 <= len`) are entirely skipped by tests that supply fewer elements than the chunk size (e.g., 2 spheres). The tests might pass via the scalar tail logic, leaving the complex SIMD logic silently untested and prone to regression.
**Action:** When auditing `unsafe` SIMD or chunked logic, always ensure tests provide exactly or greater than the required threshold to trigger the fast-path loop block, and test non-multiple sizes to ensure both fast-path and tail paths run.
