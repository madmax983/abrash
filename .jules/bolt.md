## 2024-05-14 - Replace Sequential Push with Extend for Arrays

**Learning:** Replacing sequential `.push()` calls within a hot loop (like unpacking 3 vertex indices) with a single `.extend([a, b, c])` call combined with array destructuring allows the compiler to elide repetitive vector bounds checking, resulting in measurable performance gains in hot paths.

**Action:** Whenever iterating over fixed-size inner arrays to populate a `Vec` in a hot loop, destructure the array elements first and append them in bulk using `.extend([])` instead of individual `.push()` calls.

## 2024-05-09 - [Eliding Bounds Checks on Pixel Conversion Loops]
**Learning:** In hot pixel conversion loops (e.g., extracting RGBA channels), building a `Vec` dynamically with `.extend_from_slice()` introduces capacity check overhead.
**Action:** Pre-allocating a zeroed vector (`vec![0u8; size]`) and writing directly via `.chunks_exact_mut(4)` paired with `.zip()` allows the compiler to elide bounds checks and significantly improves performance.
