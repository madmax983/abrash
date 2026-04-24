## Scene Culling Optimization
**Learning:** Pre-allocating `world_aabbs` capacity to match `num_objects` in `Scene::extract` avoids potential reallocations before `extend` is called in the hot path.
**Action:** Added `world_aabbs.reserve(num_objects)` after `clear()` inside `extract_into`.

## Scene Culling Optimization
**Learning:** Pre-allocating `world_aabbs` capacity to match `num_objects` in `Scene::extract` avoids potential reallocations before `extend` is called in the hot path.
**Action:** Added `world_aabbs.reserve(num_objects)` after `clear()` inside `extract_into`.
