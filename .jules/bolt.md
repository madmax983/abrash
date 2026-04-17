**[Optimizing draw list extraction]**
**Learning:** Calling `.get().unwrap()` on resource pools (`SlotMap`/`HashMap`) inside the Rayon `par_extend` closure incurs measurable performance overhead due to bounds checking and synchronization/cache misses.
**Action:** Always hoist resource handle resolution out of parallel closures by using a single sequential loop to collect validated references into a pre-allocated vector (`Vec<(&Resource, ...)>`).
