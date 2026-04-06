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
**[Validating Unreachable Guards]
**Learning:** To verify internal `unreachable!()` or panic safety guards against corrupted state, write unit tests in the same module (`mod tests`) to access private fields, intentionally mutate the internal state to simulate the illegal condition, and assert the guard triggers using `#[should_panic(expected = "...")]`.
**Action:** Always write a corresponding `#[should_panic]` test for any defensive `unreachable!()` statements that check internal invariants, directly testing the explosion.
**[Testing Frustum Culling Fallback Logic]**
**Learning:** In graphics code or similar spatial algorithms, "fast paths" often assume all vertices are inside a certain bounds (like the view frustum), while a "fallback path" splits and clips geometry that crosses boundaries. It's easy for these fallbacks to lack coverage, leaving them vulnerable to panics on edge cases.
**Action:** Always proactively write tests with explicitly constructed out-of-bounds geometries (e.g., quads entirely outside the frustum where all `x > w`) to force execution into these fallback blocks, ensuring they cull correctly and safely.
**[Testing Isosurface Polygonization Configurations]
**Learning:** When testing complex geometric lookup tables like `tri_table` in Marching Tetrahedra algorithms, it's easy to miss edge cases. A table-driven unit test that iterates through all possible vertex sign permutations (e.g., 16 cases for 4 vertices) and asserts the expected number of generated triangles ensures full branch coverage of the extraction logic.
**Action:** Use table-driven unit tests to iterate through all possible vertex sign permutations (e.g., 16 cases for 4 vertices) and assert the expected number of generated triangles to ensure full branch coverage of the extraction logic.
## [Line Frustum Clipping Constraints]
**Learning:** When writing tests for rasterization or line drawing functions that utilize homogeneous frustum clipping (e.g., `clip_line_to_frustum`), ensure mock vertices strictly adhere to the `-w <= x, y, z <= w` boundaries. Using arbitrary deep Z values (like `z = 5.0` with `w = 1.0`) will cause the geometry to be silently culled, resulting in false test failures where nothing is drawn.
**Action:** When mocking clip-space geometry for tests, either use realistic projection matrices to generate the coordinates or manually ensure that $x, y, z$ are all within the $[-w, w]$ range to survive frustum clipping.
