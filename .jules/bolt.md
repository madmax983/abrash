**TileBins Initial Capacity Allocation**
**Learning:** In tile-based rendering or binning systems, dynamic heap collections like `Vec::new()` embedded inside per-frame structures (like `TileBins`) can cause significant initialization stutter due to continuous heap capacity resizing across thousands of overlapping triangles in early frames.
**Action:** Always estimate and allocate a baseline working capacity using `Vec::with_capacity(n)` based on the grid constraints (e.g. `num_tiles * 4`) to safely elide these initial heap reallocations.
