**Extract Nested Closure**
**Learning:** Identical blocks of nested logic with differing core operations (e.g., metric calculation in `voronoi`) can be cleanly extracted into higher-order functions in Rust, even handling conditionally compiled branches (e.g. `#[cfg(feature = "parallel")]`).
**Action:** When facing deeply nested metric conditions in `process_pixels`, I created `process_pixels` and `process_row_with/without_borders` functions to encapsulate the iteration structure, accepting `dist_fn` as a closure.

**Extract Pheromone Sensing**
**Learning:** `Physarum` diffusion logic and pheromone sensing logic were highly nested. Extracting them to small helper functions makes the main agent update loop readable.
**Action:** Created `sense_trail` and `diffuse_and_decay` functions.
