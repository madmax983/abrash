**[Optimized Scene Pre-Calculation Loops]**
**Learning:** In hot loops iterating over parallel arrays (e.g., culling results and objects), using manual indexing like `cull_results[i]` introduces bounds-checking overhead. Replacing indexed loops with zipped iterators (e.g., `objects.iter().zip(cull_results.iter())`) and avoiding indexing elides bounds checks and yields measurable performance gains.
**Action:** Replaced `for (i, obj) in self.objects.iter().enumerate()` with `for (obj, &is_visible) in self.objects.iter().zip(cull_results.iter())` in `scene.rs` `extract` function.

**[Optimized Scene Pre-Calculation Loops]**
**Learning:** In hot loops iterating over parallel arrays (e.g., culling results and objects), using manual indexing like `cull_results[i]` introduces bounds-checking overhead. Replacing indexed loops with zipped iterators (e.g., `objects.iter().zip(cull_results.iter())`) and avoiding indexing elides bounds checks and yields measurable performance gains.
**Action:** Replaced `for (i, obj) in self.objects.iter().enumerate()` with `for (obj, &is_visible) in self.objects.iter().zip(cull_results.iter())` in `scene.rs` `extract` function.
