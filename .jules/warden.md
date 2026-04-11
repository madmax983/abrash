## 2025-01-28 - [Vulnerable Dependencies]
**Threat:** Found multiple vulnerabilities in `Cargo.lock` during `cargo audit`, including bincode, fxhash, instant, paste, drm, and uds_windows.
**Defense:** These crates are used indirectly. Some have advisories or are unmaintained. Need to investigate if we can update them.

## 2025-01-28 - [Memory Safety: TriangleLists]
**Threat:** The structs `ClippedTriangles`, `PreparedTrianglesList`, `PreparedGouraudTrianglesList`, and `PreparedTexturedTrianglesList` have their `count` fields marked as `pub`. This allows safe code to manually set the `count` higher than the number of initialized elements, leading to reading uninitialized memory (Undefined Behavior) when iterating over the list, since `assume_init()` is called in an `unsafe` block assuming `count` elements are initialized.
**Defense:** Make the `count` field private and provide a safe `len()` or `count()` accessor, ensuring the invariant that `count` always reflects the number of initialized elements cannot be broken by safe code.

## 2025-01-28 - [UB via Public Length Tracker in Partially Initialized Arrays]
**Threat:** The `ClippedTriangles` and various `Prepared*TrianglesList` structs exposed their `count` fields publicly while wrapping partially initialized arrays (`[MaybeUninit<T>; N]`). This allowed safe external code to directly modify the initialized length without adding elements, bypassing the safety invariant of the `unsafe` block `assume_init()` inside bounds checks, resulting in reading uninitialized memory (Undefined Behavior).
**Defense:** Made the `count` field private across all triangle list structs and introduced a safe, read-only `count(&self) -> usize` accessor method, ensuring external modules cannot violate the memory initialization invariant.
