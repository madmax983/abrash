1. **Refactor `PreparedTrianglesList` structures into a single generic type.**
   - In `src/rasterizer/tile.rs`, there are three distinct but identical custom list types: `PreparedGouraudTrianglesList`, `PreparedTrianglesList`, and `PreparedTexturedTrianglesList`, along with their associated iterator structs `PreparedGouraudTrianglesIter`, `PreparedTrianglesIter`, and `PreparedTexturedTrianglesIter`.
   - I will replace all of these with a single generic `PreparedTrianglesList<T>` struct (and its corresponding `PreparedTrianglesIter<T>`).
   - The generic struct will have `[MaybeUninit<T>; 8]` and `count: usize`, just like the original structs.
   - I will update the methods `push`, `new`, and the implementations of `IntoIterator` and `IntoParallelIterator` to use generics.
2. **Update usages of the refactored types.**
   - Replace occurrences of `PreparedGouraudTrianglesList` with `PreparedTrianglesList<PreparedGouraudTriangle>`.
   - Replace occurrences of `PreparedTexturedTrianglesList` with `PreparedTrianglesList<PreparedTexturedTriangle>`.
   - Replace occurrences of `PreparedTrianglesList` (non-generic) with `PreparedTrianglesList<PreparedTriangle>`.
   - Update return types of the methods `prepare_gouraud_triangle`, `prepare_textured_triangle`, and `prepare_triangle`.
3. **Complete pre-commit steps to ensure proper testing, verification, review, and reflection are done.**
4. **Submit the change.**
   - Use the commit format `🪒 Razor: [Reduction]`.
