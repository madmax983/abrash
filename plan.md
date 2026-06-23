# Objective
Refactor God functions and pyramids of doom without altering logic or output.

# Steps
1. Refactor `clip_triangle_to_frustum` in `crates/abrash-core/src/clipping.rs` to extract the trivial accept/reject logic into a helper function `detect_active_planes`. This logic was over 100 lines deeply nested within the function, violating the "God Function" smell. This reduces `clip_triangle_to_frustum` and makes it cleaner to read without changing any SIMD logic.
2. Verify the changes to `clipping.rs` by using `cat` on `crates/abrash-core/src/clipping.rs`.
3. Refactor `fill_triangle_normal_mapped` in `crates/abrash-render/src/rasterizer/texture.rs`. I will extract the closure `calculate_ts_light` into a standalone helper function outside of the large main function to flatten the structure and keep logic separated.
4. Verify the changes to `texture.rs` by using `cat` on `crates/abrash-render/src/rasterizer/texture.rs`.
5. Run workspace tests explicitly: `cargo test -p abrash-core --all-features` and `cargo test -p abrash-render --all-features`.
6. Add an entry to `.jules/forge.md` to document the learning of extracting `active_planes` masking with the format: `**[Title]\n**Learning:** [Insight]\n**Action:** [How to apply next time]`.
7. Complete pre-commit steps to ensure proper testing, verification, review, and reflection are done.
8. Commit the code with PR title format `⚒️ Forge: [refactor name]` and description sections: `🚮 Smell`, `✨ Solution`, `🧼 Benefit`, `🛡️ Verification`.
