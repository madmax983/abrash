1. **Remove the unused local benchmark trait `Lerp` and replace it with direct function calls.**
   - In `benches/clipping_optimization.rs`, the `Lerp` trait is a "One-Time" trait implemented only for `(Vec3, f32)`.
   - Remove the `pub trait Lerp` definition and its `impl Lerp for (Vec3, f32)`.
   - Update `clip_triangle_to_frustum_legacy` signature from `pub fn clip_triangle_to_frustum_legacy<V: Lerp + Copy + Default>` to `pub fn clip_triangle_to_frustum_legacy<V: Copy + Default>`. It also needs a new argument `lerp_fn: impl Fn(V, V, f32) -> V`.
   - Update `prev_v.lerp(curr_v, t)` to `lerp_fn(prev_v, curr_v, t)` inside the function.
   - Update the calls to `clip_triangle_to_frustum_legacy` inside `bench_clipping` to pass the `lerp_fn` as the 5th argument: `|a, b, t| (a.0.lerp(b.0, t), a.1 + (b.1 - a.1) * t)`. This matches how `clip_triangle_to_frustum` does it. Note that `Vec3::lerp` is natively available since it's used in the optimized version.

2. **Complete pre-commit steps to ensure proper testing, verification, review, and reflection are done.**
   - Use `cargo test` and `cargo clippy` to verify that the refactoring did not break anything and follows the rules.
   - Log the reduction in `.jules/razor.md`.

3. **Submit the PR.**
   - Title: `🪒 Razor: Remove single-use Lerp trait from clipping benchmark`
   - Describe the bloat removed and code saved.
