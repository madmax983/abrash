**[Eliminate false `clone()` zero-cost abstraction]
**Learning:** Calling `.clone().into_bytes()` on a `String` is not a zero-cost abstraction. It performs a deep copy and creates a new heap allocation, offering no performance benefit over `.as_bytes().to_vec()`.
**Action:** Do not attempt to use `.clone().into_bytes()` as an optimization over `.to_vec()`. To truly eliminate allocations, either reuse a pre-allocated vector or avoid cloning the underlying string entirely.

**[Pre-allocate all vectors in `with_capacity`]
**Learning:** When implementing `with_capacity` constructors for structs with multiple vector fields (e.g., `DrawList`), leaving some fields initialized with `Vec::new()` introduces hidden heap allocations when they are later populated.
**Action:** Always provide explicit capacity parameters for all relevant vector fields in a `with_capacity` constructor.
