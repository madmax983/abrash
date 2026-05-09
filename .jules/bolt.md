## 2024-05-14 - Replace Sequential Push with Extend for Arrays

**Learning:** Replacing sequential `.push()` calls within a hot loop (like unpacking 3 vertex indices) with a single `.extend([a, b, c])` call combined with array destructuring allows the compiler to elide repetitive vector bounds checking, resulting in measurable performance gains in hot paths.

**Action:** Whenever iterating over fixed-size inner arrays to populate a `Vec` in a hot loop, destructure the array elements first and append them in bulk using `.extend([])` instead of individual `.push()` calls.
