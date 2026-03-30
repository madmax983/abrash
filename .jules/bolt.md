**Pre-allocate Mesh Buffers**
**Learning:** Procedural mesh generation functions (`sphere`, `torus`, `plane`, `cylinder`) were using `Vec::new()` for vertices, normals, uvs, and indices, causing unnecessary heap reallocations during construction.
**Action:** Calculate the exact vertex and index counts mathematically based on the input parameters (stacks, sectors, segments, subdivisions) and initialize the vectors using `Vec::with_capacity(count)` to eliminate O(N) reallocation overhead.
