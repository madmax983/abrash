1. **Optimize Math Functions Using Squared Length/Distance**
   - In `crates/abrash-core/src/math/funcs.rs`, inside the `givens_rotation` and `convex_hull_2d` sorting closures, `f32::hypot` calculates the exact Euclidean distance/length, which incurs floating point logic for underflow/overflow bounds checking. Since expected values in these functions do not risk exceeding standard f32 limits, this introduces unnecessary overhead.
   - Replace `a.hypot(b)` with `(a * a + b * b).sqrt()` inside `givens_rotation`. Add `#[allow(clippy::imprecise_flops)]` to quiet the resultant lint.
   - Replace `a.hypot(b)` with `(a * a + b * b)` inside `convex_hull_2d` (where only relative distance comparisons matter).
2. **Verify Math Changes**
   - Run `cargo check -p abrash-core` to verify that the edits in `crates/abrash-core/src/math/funcs.rs` compile successfully without syntax errors or warnings.
3. **Optimize Boids Example**
   - In `crates/abrash-render/src/experimental/boids.rs`, the distance check uses `dx.hypot(dy).hypot(dz)` which is extremely slow. This can be replaced with `distance_sq = dx*dx + dy*dy + dz*dz` for the bounds checks, and only compute `distance_sq.sqrt()` when necessary.
4. **Complete pre-commit steps to ensure proper testing, verification, review, and reflection are done.**
   - Run `cargo clippy --all-targets --all-features -- -D warnings`.
   - Run `cargo fmt --all`.
   - Run `cargo test`.
5. **Submit the PR**
   - Submit the PR with the required Bolt persona title format.
