**O(1) Array Clearing**
**Learning:** When resetting large, sparse arrays or grid structures (such as tile bin heads/tails) every frame, replacing O(N) `.fill()` operations with a generational index (`current_generation` counter and `generations` array) eliminates significant memory bandwidth overhead, effectively making the clear operation O(1).
**Action:** Replace `heads.fill(u32::MAX)` with generational increments in hot structures like `TileBins`.
