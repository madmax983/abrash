1. **Explore and Identify**
   - Review Sentry guidelines requiring strict code safety and `cargo audit` zero-vulnerability limits.
   - Investigate vulnerable dependencies via `cargo audit` (`rand`, `imageproc`, `paste` etc). Note that many are upstream inside `fyrox` or `bevy` sub-trees and cannot be simply eliminated via `cargo update` without breaking semantic compatibility in those workspaces. As per instructions, focus strictly on direct application logic fixes.

2. **UB in Tile Initialization**
   - Audit `PreparedGouraudTrianglesList`, `PreparedTrianglesList`, and `PreparedTexturedTrianglesList` in `crates/abrash-render/src/rasterizer/tile.rs`. Their `count` field is marked `pub`, allowing safe code to arbitrarily inflate the active element length. Since these wrap `MaybeUninit` arrays, any safe consumer can force the list to execute `.assume_init()` on uninitialized memory, triggering UB.
   - Refactor these structs to make the `count` field private and expose a safe `.count()` getter method.
   - Update usages across the test suite (`sentry_clipping_fuzz.rs`).

3. **Out-of-Bounds Pointer Arithmetic in AlignedBuffer**
   - In `AlignedBuffer::new()` and `resize()`, pointer addition uses the unguarded `unsafe { start_ptr.add(offset_elements) }`.
   - Update this to safely use `start_ptr.wrapping_add(offset_elements)` so massive capacity calculations safely wrap and panic organically downstream instead of manifesting as silent memory layout corruption.

4. **Unchecked Unwraps in Hi-Z Culling**
   - The method `process_triangles` uses `unsafe { hiz_buffer_ref.unwrap_unchecked() }` conditioned on an unrelated boolean flag (`has_hiz`).
   - Rewrite this to standard idiomatic rust `if let Some(hiz) = hiz_buffer_ref` to safely bind the scope.

5. **Clippy Pedantic Cleanup (`-D warnings`)**
   - Run `cargo clippy --all-targets --all-features -- -D warnings`.
   - Resolve various syntax and formatting lint issues, including missing document backticks (`clippy::doc_markdown`), unnecessary Result wraps (`clippy::unnecessary_wraps`), missing trailing semicolons in Criterion benchmarks (`clippy::semicolon_if_nothing_returned`), long unreadable hexadecimal literals (`clippy::unreadable_literal`), missing type casting (`clippy::cast_lossless`), missing `#allow(clippy::imprecise_flops)` over `sqrt()`, etc.

6. **Verify Constraints**
   - Run `cargo test --all-targets --all-features` to ensure no runtime regressions occurred due to the struct field privacy modifications and macro refactoring.
   - Run `cargo clippy --all-targets --all-features -- -D warnings` and confirm 0 warnings.
   - Re-run `cargo fmt --all`.
   - Complete pre-commit verifications.

7. **Submit Changes**
   - Submit under branch `warden-safety-audit` with a detailed PR adhering to the Warden persona reporting guidelines.
