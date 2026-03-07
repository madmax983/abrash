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
