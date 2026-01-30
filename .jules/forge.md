# Forge's Journal

## Critical Learnings
**[Extraction of Rasterization Logic]**
**Learning:** Extracting the `compute_scanline_factors` logic (calculating alpha/beta and second_half flag) significantly reduced the cognitive load of the main rasterization loops in `fill_triangle_3d` and `fill_triangle_gouraud`.
**Action:** Always look for common geometric calculation patterns in rasterization loops and extract them, even if they return a struct of factors.

**[Generic Sorting Helper]**
**Learning:** Using a simple generic helper `sort_triangle_by_y` with a key extractor closure (`|v| v.y`) allowed unified sorting logic across `Vec2` structs and various tuple configurations `(Vec3, f32)` without needing complex traits.
**Action:** Prefer high-order functions (taking closures) over complex trait bounds for simple local helpers.
