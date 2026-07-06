**[Optimizing HashSet Instantiations]**
**Learning:** Replacing `std::collections::HashSet` with `foldhash::HashSet` for simple data types (like integer pairs) in tight loops yields significant performance gains by avoiding SipHash cryptographic overhead.
**Action:** When swapping to foldhash, remember to import the `foldhash::HashSetExt` trait to access extension methods like `with_capacity()`.

**[Avoiding Chained Iterator Collect Allocations]**
**Learning:** To prevent dynamic heap reallocations when mapping over chained or complex iterators, `Iterator::collect::<Vec<_>>()` can sometimes fail to optimize capacity.
**Action:** Pre-allocate the target vector using `Vec::with_capacity` (using `size_hint().0` or exact known lengths) and use `.extend()` to push values directly into the pre-sized memory.
