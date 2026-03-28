**TileRenderer Merge Refactoring**
**Learning:** Found heavily duplicated boilerplate for merging tiles back into the framebuffer (in flat, textured, and gouraud batches) where logic for clear-checking, partial updating, bounds calculation, and threading code was copy-pasted 3 times resulting in ~80 lines of identical code each.
**Action:** Extracted this into `merge_rendered_tile_sequential` and `merge_rendered_tile_parallel` static helper methods. This flattened nesting, reduced file size by hundreds of lines, and made the core rendering loop much more readable without changing behavior.
