**[Obj Loader Coverage]**
**Learning:** Empty components in OBJ formatting, e.g., missing vertex definitions or standalone faces with no elements, should be well covered to ensure parsing handles non-ideal file formats correctly without crashing. We added tests for these boundary conditions.
**Action:** Always include table-driven or edge case tests covering partial strings and completely empty string data for parsing code.

**[Culling SIMD Bounds Safety]**
**Learning:** Pre-allocating parallel culling buffers using `unsafe` blocks required exact matching arrays to prevent memory overflow crashes since SIMD intrinsically ignores checking. Asserting equal array size properly enforces structural bounds.
**Action:** When adding arrays directly interacting with unsafe pointer iterations like AVX2 intrinsics, include `#[should_panic]` unit tests triggering the assert guard to demonstrate it behaves safely on incorrect input.
