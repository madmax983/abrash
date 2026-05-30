
**Gradient Construction**
**Learning:** `Gradient::new` and `Gradient::from_stops` used `.into_iter().collect::<Vec<_>>()`, which relies on `collect` to determine capacity. However, when we map over iterators or take iterators, manually calling `Vec::with_capacity(iter.size_hint().0)` and `.extend()` explicitly prevents unnecessary reallocations when building color gradients.
**Action:** When building collections from iterators, use `with_capacity` based on `size_hint` and `extend` to ensure exact sizes are allocated up front on hot paths.
