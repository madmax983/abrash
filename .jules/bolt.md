**[Title]
**Learning:** When replacing `Mesh::new()` with `Mesh::with_capacity()` to avoid vector reallocations during procedural generation (e.g., in a voxelizer), ensure the exact index count math is correct: a quadrilateral face consists of 2 triangles, which equates to 6 indices (not 2). Miscalculating capacity multipliers will still result in reallocations or excess memory usage.
**Action:** Always map geometric concepts (faces, triangles) explicitly back to array elements (vertices, indices) mathematically before allocating capacity.
