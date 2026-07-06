1. **Optimize TileBins clear overhead with generational indices**
   - The current `TileBins::clear()` function takes O(N) memory bandwidth because it has to `fill(u32::MAX)` on the `heads` and `tails` vectors. This is particularly expensive for high resolutions where there are many tiles.
   - I will change `TileBins` to include a `generations: Vec<u32>` and a `current_generation: u32`.
   - In `clear()`, I will increment `current_generation`. If it overflows, I will reset `generations` to 0. This makes `clear()` an O(1) operation most of the time instead of O(N) memory memset.
   - I will update `push()`, `iter()`, `sort_bins_flat()`, `sort_bins_textured()`, `sort_bins_gouraud()`, `clear_tile_bounds()`, and parallel bin iterations to respect the generation counter.
2. **Pre-commit Steps**
   - Execute `cargo fmt --all`, `cargo clippy --all-targets --all-features -- -D warnings`, and `cargo test --workspace --features "parallel nova"`.
   - Update `.jules/bolt.md` with the new learning.
   - Run `pre_commit_instructions` to ensure proper testing, verification, review, and reflection are done.
3. **Submit the PR**
   - Submit the PR with the title "⚡ Bolt: [TileBins generation optimization]" and a description detailing the impact.
