## Fire Effect
**Concept:** Implement a classic Doom-style software-rendered fire effect using cellular automaton and palette lookup mapping.
**Fate:** Successfully merged behind `#[cfg(feature = "nova")]`. The render loop was highly optimized via unsafe unchecked pixel setters (`set_pixel_unchecked`) due to pre-checked bounds, granting an ~68% rendering speedup.
**Lesson:** Palettized software effects are still extremely cheap to compute. Extracting 1D array offset calculation out of the inner loop and using unchecked slice assignment allows Rust's optimizer to emit very tight assembly for rendering operations.

## Fire Effect
**Concept:** Implement a classic Doom-style software-rendered fire effect using cellular automaton and palette lookup mapping.
**Fate:** Successfully merged behind `#[cfg(feature = "nova")]`. The render loop was highly optimized via unsafe unchecked pixel setters (`set_pixel_unchecked`) due to pre-checked bounds, granting an ~68% rendering speedup.
**Lesson:** Palettized software effects are still extremely cheap to compute. Extracting 1D array offset calculation out of the inner loop and using unchecked slice assignment allows Rust's optimizer to emit very tight assembly for rendering operations.
