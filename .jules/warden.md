## 2025-01-28 - [Vulnerable Dependencies]
**Threat:** Found multiple vulnerabilities in `Cargo.lock` during `cargo audit`, including bincode, fxhash, instant, paste, drm, and uds_windows.
**Defense:** These crates are used indirectly. Some have advisories or are unmaintained. Need to investigate if we can update them.

## 2025-01-28 - [Memory Safety: TriangleLists]
**Threat:** The structs `ClippedTriangles`, `PreparedTrianglesList`, `PreparedGouraudTrianglesList`, and `PreparedTexturedTrianglesList` have their `count` fields marked as `pub`. This allows safe code to manually set the `count` higher than the number of initialized elements, leading to reading uninitialized memory (Undefined Behavior) when iterating over the list, since `assume_init()` is called in an `unsafe` block assuming `count` elements are initialized.
**Defense:** Make the `count` field private and provide a safe `len()` or `count()` accessor, ensuring the invariant that `count` always reflects the number of initialized elements cannot be broken by safe code.

**2026-04-13 - Uninitialized Memory Read in PreparedTrianglesLists**
**Threat:** `PreparedTrianglesList`, `PreparedGouraudTrianglesList`, and `PreparedTexturedTrianglesList` had public `count` fields while wrapping `MaybeUninit` arrays. This allowed safe code to arbitrarily modify the length tracking, causing the internal iterators to call `.assume_init()` on uninitialized memory, leading to Undefined Behavior and potential information disclosure or crashes.
**Defense:** Made the `count` fields private across all tile binning lists and exposed a safe `count()` getter, enforcing the memory safety invariant at the module boundary.
