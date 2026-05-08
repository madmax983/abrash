1. **Refactor `Evaluable` and `Sample` in `abrash-anim` and `abrash-core`**
   - **Target:** `crates/abrash-core/src/animatable.rs`, `crates/abrash-anim/src/evaluable.rs`, `crates/abrash-anim/src/hold.rs`, `crates/abrash-anim/src/keyframe.rs`, `crates/abrash-anim/src/sequence.rs`, `crates/abrash-anim/src/timeline.rs`, `crates/abrash-anim/src/easing.rs`
   - **Action:** Remove the `velocity` field from `Sample<T>` because it is YAGNI (speculative generality for "future velocity-preserving spring interruption" as stated in the comments). It complicates the `Animatable` trait, which forces implementors to provide `anim_scale`, `anim_add`, `anim_sub`, and `zero` purely to calculate this unused velocity.
   - Remove `anim_scale`, `anim_add`, `anim_sub`, and `zero` from the `Animatable` trait in `crates/abrash-core/src/animatable.rs`.
   - Update implementations of `Animatable` to only include `interpolate` and `distance_squared`.
   - Remove `velocity` field from `Sample<T>` in `crates/abrash-anim/src/evaluable.rs`.
   - Update `Hold::evaluate` and tests.
   - Update `Keyframe::evaluate` and tests.
   - Log reduction to `.jules/razor.md`.

2. **Verify Changes**
   - Run `cargo test` to ensure tests pass.
   - Run `cargo clippy --all-targets --all-features -- -D warnings` and `cargo fmt --all`.
   - Fix `tests/sentry_clipping_fuzz.rs` compilation error (`clipped.count` -> `clipped.count()`).

3. **Pre-commit step**
   - Complete pre-commit steps to ensure proper testing, verification, review, and reflection are done.

4. **Submit**
   - Submit the refactoring using `submit`.
