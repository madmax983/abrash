# Sentry's Journal

## [Framebuffer `clear_rect` Out-of-Bounds]
**Learning:** The `clear_rect` function in `Framebuffer` panicked when arguments were fully out of bounds or negatively out of bounds due to incorrect scaling of the start/end slices, as `y.saturating_add` on large heights could be smaller than expected. Also, `clamp` was useful to enforce proper boundaries.
**Action:** When working with grid buffers and arbitrary rectangular boundaries, use `clamp(0, max_size)` for correct region intersection, ensuring `start <= end` before looping over the rows.
## [Texture `generate_mipmaps` Unwrap Panic]
**Learning:** Using `.unwrap()` on dynamic collections like `mips.last()` within algorithms like `generate_mipmaps` poses a panic risk if the collection state diverges (e.g., cleared before generation). It's crucial to substitute risky fallback unwraps with safer alternatives.
**Action:** Replace risky fallback `.unwrap()` calls on slices or iterators (e.g., `mips.last().unwrap()`) with `.unwrap_or(&default_value)` to safely handle empty collections without explicit if/else branching.
## [Raytracer `unwrap()` Panic on Object Hit]
**Learning:** In raytracing logic where a hit is guaranteed to correspond to a scene object, managing `closest_hit: Option<Hit>` and `hit_obj: Option<&SceneObject>` as separate variables creates an implicit synchronization dependency and a dangerous `unwrap()` panic point when extracting the object later.
**Action:** Group synchronized optional state (like a raycast hit and its intersected object) into a single `Option<(Hit, &SceneObject)>` tuple. This enforces synchronization at the type level and allows safe destructuring without risky `unwrap()` calls.
**[Heat Vision Graceful Degradation]**
**Learning:** `f32::clamp(min, max)` on a value that is `f32::NAN` does not panic; it simply returns `NaN`. Furthermore, casting `f32::NAN` (e.g. `(t * 255.0) as u32`) yields `0`. When auditing rendering or math logic for panics, recognize that `NaN` values will often silently propagate or cast to `0` rather than causing an explicit crash, providing unexpected default values instead of fatal errors.
**Action:** When testing float boundaries, explicitly write tests that push `f32::MIN` and `f32::MAX` to simulate out-of-bounds inputs rather than solely relying on `f32::NAN` to trigger float panics.
**[Validating Unreachable Guards]
**Learning:** To verify internal `unreachable!()` or panic safety guards against corrupted state, write unit tests in the same module (`mod tests`) to access private fields, intentionally mutate the internal state to simulate the illegal condition, and assert the guard triggers using `#[should_panic(expected = "...")]`.
**Action:** Always write a corresponding `#[should_panic]` test for any defensive `unreachable!()` statements that check internal invariants, directly testing the explosion.
**[Testing Frustum Culling Fallback Logic]**
**Learning:** In graphics code or similar spatial algorithms, "fast paths" often assume all vertices are inside a certain bounds (like the view frustum), while a "fallback path" splits and clips geometry that crosses boundaries. It's easy for these fallbacks to lack coverage, leaving them vulnerable to panics on edge cases.
**Action:** Always proactively write tests with explicitly constructed out-of-bounds geometries (e.g., quads entirely outside the frustum where all `x > w`) to force execution into these fallback blocks, ensuring they cull correctly and safely.
**[Testing Isosurface Polygonization Configurations]
**Learning:** When testing complex geometric lookup tables like `tri_table` in Marching Tetrahedra algorithms, it's easy to miss edge cases. A table-driven unit test that iterates through all possible vertex sign permutations (e.g., 16 cases for 4 vertices) and asserts the expected number of generated triangles ensures full branch coverage of the extraction logic.
**Action:** Use table-driven unit tests to iterate through all possible vertex sign permutations (e.g., 16 cases for 4 vertices) and assert the expected number of generated triangles to ensure full branch coverage of the extraction logic.
**[Steganography Integer Overflow Prevention]**
**Learning:** When calculating buffer sizes or bit counts from arbitrary or untrusted lengths (e.g., from an image or framebuffer), standard arithmetic operators (`+`, `*`) can easily overflow `usize` boundaries and cause panics. For example, `(4 + len) * 8` can panic if `len` is close to `usize::MAX`.
**Action:** Always use safe arithmetic like `checked_add` and `checked_mul` when dealing with lengths or sizes derived from untrusted inputs, and bubble up safe `Option` or `Result` types instead of crashing.
**[Testing Corrupted Handle Freelist Guards]**
**Learning:** Defensive `unreachable!()` guards in resource pools (e.g. verifying that a slot pulled from a free list is actually `Vacant` rather than `Occupied`) can only be tested by directly mutating private internal state (corrupting the free list or entry state) and simulating a double-alloc or memory corruption bug.
**Action:** When auditing custom allocation pools or handle maps, write tests in the same module that artificially mutate the internal structures (e.g. modifying the `entries` or `free_list`) to ensure these hard-to-reach defensive panics actually fire when invariants are broken.
**[DrawList Capacity Overflow]**
**Learning:** Functions that pre-allocate massive vectors inside graphic pipelines (like `DrawList::with_capacity(..., usize::MAX, ...)`) will rightfully panic with a built-in rust `capacity overflow` if they are given absurd memory bounds (e.g., from an untrusted fuzzed command buffer length).
**Action:** Always constrain maximum batch, vertex, or object counts (and correctly handle OOM via checked math) prior to dynamically allocating memory bounds from frame descriptions. Use `should_panic(expected = "capacity overflow")` tests to document exactly when boundaries are expected to burst.

**[Total Ordering for Floats]
**Learning:** Using `.partial_cmp().unwrap()` to sort floating-point numbers can panic when encountering `NaN` values, and `.unwrap_or(Ordering::Equal)` breaks the strict total ordering requirement of sorting algorithms. Using an unhandled `.unwrap()` at the end of a `min_by` or `max_by` call on iterators can also panic if the collection is empty.
**Action:** Use `f32::total_cmp` (or `f64::total_cmp`) instead of `partial_cmp` to provide a robust total ordering when sorting or finding min/max values in float slices. Also, replace terminal `.unwrap()` calls on iterators with `.unwrap_or()` or `.map_or()` to handle empty collections safely without panics.
**[Simulating Unreachable Guards in Tests]**
**Learning:** To verify internal `unreachable!()` or panic safety guards that protect against mathematically impossible paths or state corruption (such as invalid enum matching or invalid state transitions), write unit tests within the same module (`#[cfg(test)] mod tests`). Since normal API usage cannot trigger these paths, you must directly simulate the illegal condition by isolating the exact matching logic or manually overriding state, and then assert the guard triggers correctly using `#[should_panic(expected = "...")]`.
**Action:** When a guard is "unreachable" via public methods, isolate the specific `match` or state check block into the test body and manually force the illegal value to verify the `unreachable!()` panic executes with the correct message.

**[Validating Expect Guards on Allocation Dimensions]**
**Learning:** Calculations that multiply dimensions (like `width * height`) to determine allocation size can easily overflow `usize`, leading to panics via `.expect("... overflow")` or implicitly during allocation. These guards are critical for security and stability but are often untested because they require absurdly large inputs (like `u32::MAX`).

**[Validating Expect Guards on Allocation Dimensions]**
**Learning:** Calculations that multiply dimensions (like `width * height`) to determine allocation size can easily overflow `usize`, leading to panics via `.expect("... overflow")` or implicitly during allocation. These guards are critical for security and stability but are often untested because they require absurdly large inputs (like `u32::MAX`).
**Action:** Always write a corresponding `#[should_panic]` test for bounds checking logic covering allocation counts by testing the explicit limits (e.g., `u32::MAX`).

**[Missing Tests for Handle Error Propagation]**
**Learning:** Even when errors like `StaleHandle` are correctly propagated from internal handle lookup failure via `ok_or()`, there might not be explicit tests verifying the exact enum variant of the returned `Result`. The existing test suite was heavily verifying stale `Mesh` handles, but silently lacked mirror tests for `Material` handles, leaving a logic path untested.
**Action:** Audit functions that interact with multiple types of resource handles (e.g. `Mesh` and `Material`) to ensure that *every* handle type has a corresponding failure test, rather than relying on a single representative test case.
**[Validating Time/Tick Loop Logic Without Flakiness]**
**Learning:** Testing frame-time loops using `std::thread::sleep` introduces severe flakiness because CI runners or test threads can delay execution unpredictably. Instead of testing "real" time elapsed, test the internal response to elapsed time by directly simulating it on the structure (e.g. `timer.last_time = Instant::now()`, `timer.accumulator += 50ms`).
**Action:** Never use `std::thread::sleep` for unit testing timing systems. Always manipulate the simulated time state directly to test logic boundaries.

**[Handles `get_mut` Vacant Entry Coverage]**
**Learning:** Functions that return options based on internal state matching (like `ResourcePool::get_mut` checking for `PoolEntry::Occupied`) often lack coverage for the negative `Vacant` path if normal operations only query active handles.
**Action:** When auditing resource pools or custom allocators, ensure explicit tests exist that insert, remove, and then immediately query (`get` or `get_mut`) the removed handle to verify the `None` path is safely triggered without panicking.

**[Validating Unreachable Guards on BorrowedRenderTarget]**
**Learning:** `RenderTarget::borrow_mut()` contains a protective `.expect("owned render target has matching color and depth buffers")` guard when converting its owned memory into a borrowed view. While theoretically impossible during normal usage, validating this explicit panic boundary requires artificially mutating internal slice boundaries (e.g. `target.framebuffer = Framebuffer::new(2, 2).unwrap()`) and then calling `borrow_mut()` to confirm it crashes correctly.
**Action:** When a struct provides an explicit boundary or conversion guard utilizing `.expect()` or `unreachable!()`, always write a dedicated `#[should_panic]` test that intentionally breaks internal invariants to guarantee the crash occurs as intended.

**HiZBuffer Safety Checks**
**Learning:** `HiZBuffer` operations involve several strict internal safety assertions (`assert_eq!` for dimensions, `assert!` for size bounds and slice lengths) to prevent downstream index out of bounds or allocation errors. These are vital for safe rendering paths.
**Action:** Always ensure that structural parameter verification (like `assert!(width > 0)`) and cross-component dimension checks (like matching width/height between `HiZBuffer` and a given `ZBuffer` slice) are explicitly covered by `#[should_panic]` unit tests.
**[Total Ordering for Floats - Testing Transitivity Panics]**
**Learning:** When testing for 'strict weak ordering' panics (e.g., incorrect float sorting handling `NaN`s with `unwrap_or(Equal)`), small or uniform arrays may pass by chance. To reliably trigger the standard library's transitivity panic in `sort_unstable_by`, use a sufficiently large array (e.g., 100+ elements) with an alternating pattern of `NaN`s and distinct valid numbers.
**Action:** When writing tests to verify that `total_cmp` correctly replaced `unwrap_or(Equal)`, ensure the test data creates a specific sequence (like alternating NaNs and differing valid floats) to reliably trigger the transitivity violation that causes sorting algorithms to panic.

## [Bounds Check Panic on apply_fire]
**Learning:** Functions that accept parallel slices for reading (e.g. `cooling_map`) and map over the primary framebuffer dimensions MUST assert that the provided slices have sufficient length to cover the entire framebuffer before starting iteration. Otherwise, out-of-bounds indexing will cause panics in multithreaded loops, which can crash the entire application instantly.
**Action:** Always add an explicit bounds check (e.g. `if map.len() < width * height { return; }`) at the start of array-processing functions to prevent out-of-bounds access.

**[Missing Tests for Handle Error Propagation]**
**Learning:** Even when errors like `StaleHandle` are correctly propagated from internal handle lookup failure via `ok_or()`, there might not be explicit tests verifying the exact enum variant of the returned `Result`. The existing test suite was heavily verifying stale `Mesh` handles, but silently lacked mirror tests for `Material` handles, leaving a logic path untested.
**Action:** Audit functions that interact with multiple types of resource handles (e.g. `Mesh` and `Material`) to ensure that *every* handle type has a corresponding failure test, rather than relying on a single representative test case.
