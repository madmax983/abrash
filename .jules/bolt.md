**[Optimizing Sparse Grid Clearing]**
**Learning:** Unconditionally clearing large grid or tile structures (e.g., using `slice.fill` on an entire buffer) can cause O(N) overhead when only a fraction of elements are updated.
**Action:** Tracking active elements in a separate `used_bins: Vec<usize>` array and iterating over it to clear only dirtied items reduces clearing complexity to O(K), providing significant performance improvements in sparse operations.

**[Iterator pre-allocation with collect]**
**Learning:** Replacing `.collect::<Vec<_>>()` with `.with_capacity()` and `.extend()` for iterators that implement `TrustedLen` (like `Range` or `Map` over ranges) provides zero performance benefit and is less idiomatic, as `.collect()` already perfectly pre-allocates the exact capacity.
**Action:** Do not replace `.collect()` with manual pre-allocation if the iterator's size hint is exact and trusted.
