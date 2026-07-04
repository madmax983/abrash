## 2025-01-28 - [Vulnerable Dependencies]
**Threat:** Found multiple vulnerabilities in `Cargo.lock` during `cargo audit`, including bincode, fxhash, instant, paste, drm, and uds_windows.
**Defense:** These crates are used indirectly. Some have advisories or are unmaintained. Need to investigate if we can update them.

## 2025-01-28 - [Memory Safety: TriangleLists]
**Threat:** The structs `ClippedTriangles`, `PreparedTrianglesList`, `PreparedGouraudTrianglesList`, and `PreparedTexturedTrianglesList` have their `count` fields marked as `pub`. This allows safe code to manually set the `count` higher than the number of initialized elements, leading to reading uninitialized memory (Undefined Behavior) when iterating over the list, since `assume_init()` is called in an `unsafe` block assuming `count` elements are initialized.
**Defense:** Make the `count` field private and provide a safe `len()` or `count()` accessor, ensuring the invariant that `count` always reflects the number of initialized elements cannot be broken by safe code.

**2026-04-13 - Uninitialized Memory Read in PreparedTrianglesLists**
**Threat:** `PreparedTrianglesList`, `PreparedGouraudTrianglesList`, and `PreparedTexturedTrianglesList` had public `count` fields while wrapping `MaybeUninit` arrays. This allowed safe code to arbitrarily modify the length tracking, causing the internal iterators to call `.assume_init()` on uninitialized memory, leading to Undefined Behavior and potential information disclosure or crashes.
**Defense:** Made the `count` fields private across all tile binning lists and exposed a safe `count()` getter, enforcing the memory safety invariant at the module boundary.
## 2026-04-12 - [Denial of Service: Integer Overflow in Bresenham's Circle]
**Threat:** The `draw_circle` and `fill_circle` algorithms blindly used unchecked math operations (`3 - 2 * radius`) for computing the decision variable. This allowed massive malicious `radius` inputs to overflow the `i32` integers, bypassing checks or triggering unhandled panics, leading to DoS. A previous PR introduced an artificial bound `< 16384` but used an early return rather than safely panicking, masking the vulnerability.
**Defense:** Replaced the unchecked `3 - 2 * radius` math with `checked_mul` and `checked_sub` (using `map_or_else` to avoid branch lint warnings). Now the functions will safely and loudly panic with "Circle drawing integer overflow" on malicious boundaries rather than behaving unsoundly.

**2026-04-16 - TileRenderer Dimensions Overflow**
**Threat:** `TileRenderer` calculated `expected_len` sizes by silently risking an integer overflow on huge `width` and `height` properties in constructor boundaries. Once overflown, the array length expectation shrunk erroneously small. Because the `SendPtr::write` bounds were evaluated using unchecked buffer references under multithreading (`parallel` feature), it allowed attackers to perform out-of-bounds heap modifications by passing tiny `Framebuffer` slices that unexpectedly met the shrinked size constraint.
**Defense:** Hardened the boundaries. Embedded an explicit `checked_mul(height as usize).expect("TileRenderer dimensions overflow")` inside `TileRenderer::new(width: u32, height: u32)` to guarantee that `TileRenderer` construction panics cleanly during generation before creating mismatched, dangerous instances.

**2026-04-14 - Uninitialized Memory Read in PreparedTrianglesLists via Rayon Parallel Iterators**
**Threat:** The `PreparedTrianglesList`, `PreparedGouraudTrianglesList`, and `PreparedTexturedTrianglesList` structs implemented `IntoParallelIterator` which initialized a temporary array using `unsafe { MaybeUninit::zeroed().assume_init() }`. Since `assume_init()` acts on uninitialized generic memory containing padding and floats, this leads to immediate Undefined Behavior. Additionally, iterating over this and copying elements leads to further memory un-safety as values are accessed before being properly verified.
**Defense:** Replaced the unsafe UB with safe array initialization via `[const { MaybeUninit::uninit() }; 8]`, writing via the `.write` method on `MaybeUninit`, and safely casting the array back via pointer reads avoiding `.assume_init()` on uninitialized memory fields.

## 2026-04-18 - [Heap Buffer Overflow in Pixel Sort via Unchecked SendPtr]
**Threat:** The `SendPtr` wrapper inside `crates/abrash-render/src/experimental/pixel_sort.rs` lacked a length parameter and blindly added indices to the raw pointer via `*ptr.0.add(...)`. If a framebuffer lied about its dimensions or if sorting logic failed, it would lead to a catastrophic out-of-bounds heap read/write.
**Defense:** Rewrote `SendPtr` to capture and store the length of the slice at instantiation. Replaced direct raw pointer dereferencing with safe `read()` and `write()` methods containing an `assert!(index < self.1, "Index out of bounds")` guard before evaluating the `unsafe` block.
**2025-01-20 - [memmap2 Unchecked Pointer Offset]**
**Threat:** Unchecked pointer offset in `memmap2` (RUSTSEC-2026-0186).
**Defense:** Bumped `memmap2` to version `0.9.11`.

**2025-01-20 - [rand Unsound Custom Logger]**
**Threat:** `rand` is unsound with a custom logger using `rand::rng()` (RUSTSEC-2026-0097).
**Defense:** Bumped `rand` from `0.8.5` to `0.8.6` and `0.9.2` to `0.9.4`.

**2025-01-20 - [imageproc Fragile Bounds Check]**
**Threat:** Fragile bounds check when sampling from image (RUSTSEC-2026-0115, RUSTSEC-2026-0117).
**Defense:** Bumped `imageproc` from `0.25.0` to `0.25.1`.

**2025-01-20 - [anyhow Unsoundness]**
**Threat:** Unsoundness in `Error::downcast_mut()` (RUSTSEC-2026-0190).
**Defense:** Bumped `anyhow` from `1.0.102` to `1.0.103`.
