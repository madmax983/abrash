
## [Vulnerability Found]
**Target:** `src/obj_loader.rs` (Wavefront OBJ Parser)
**Trigger:** `f 1/1111111/1`
**Flaw:** The parser safely bounded vertex indices (`v_idx`) against the maximum allowed vertices but silently failed to check bounds for UV (`vt_idx`) and Normal (`vn_idx`) indices *before* attempting to pack them into a 20-bit limited `VertexKey`.
**Outcome:** Feeding integers larger than the 20-bit `SENTINEL` (1,048,575) triggered a debug assertion panic (`assertion failed: k_vt <= SENTINEL`). In release mode without debug assertions, this would cause silent bitwise truncation leading to corrupted mesh output or out-of-bounds indexing.
**Resolution:** Explicit length-checks against `raw_uvs` and `raw_normals` are now enforced *before* packing the `VertexKey`. Added comprehensive `proptest` harness enforcing string-length torture and integer overflow bounds.
## [Integer Overflow in AnimationClock]
**Target:** `crates/abrash-anim/src/clock.rs` (`AnimationClock::tick`)
**Trigger:** Calling `tick()` with an extremely small `duration` (e.g., `1e-38f32`) and a standard `delta_secs`.
**Flaw:** The clock calculated `phase_advance` as `delta_secs / duration`. With near-zero subnormal floats, this resulted in a massively large float value. When added to the clock's current `phase` and then casting the integer portion (`whole_cycles`) to `u64` to accumulate into `self.cycle`, it caused a `u64` addition overflow (`attempt to add with overflow`) when the number of cycles exceeded `u64::MAX`.
**Outcome:** Engine crashed via unhandled panic during animation evaluation.
**Resolution:** Replaced the unchecked `self.cycle += whole_cycles` with `self.cycle = self.cycle.saturating_add(whole_cycles)`. The clock gracefully tops out at `u64::MAX` instead of crashing.
## [Chunk Size Panic in Rayon/Iterator Iteration on Empty Dimensions]
**Target:** `crates/abrash-render/src/` (Multiple pixel filter modules: `ascii.rs`, `chroma_key.rs`, `pixel_sort.rs`, `tilt_shift.rs`, `radial_blur.rs`, etc)
**Trigger:** `width = 0` passed into Framebuffer initialization and then into filter/renderer passes.
**Flaw:** Throughout the codebase, pixel modification and iterators operate on per-row chunks, instantiated using `.chunks_exact_mut(width)` or Rayon's `.par_chunks_exact_mut(width)`. If the user passes a `Framebuffer` with a width of `0`, these functions panicked with `"chunk size must be non-zero"`.
**Outcome:** The engine crashed via an unhandled panic inside standard library and Rayon iterators whenever drawing a 0-width (invisible) image filter effect.
**Resolution:** Replaced all instances of `width` inside `.chunks_exact_mut(width)` and `.par_chunks_exact_mut(width)` with `width.max(1)`. The iterators can handle empty slices correctly, as long as the chunk parameter itself is positive.
