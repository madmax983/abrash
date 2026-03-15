
## [Vulnerability Found]
**Target:** `src/obj_loader.rs` (Wavefront OBJ Parser)
**Trigger:** `f 1/1111111/1`
**Flaw:** The parser safely bounded vertex indices (`v_idx`) against the maximum allowed vertices but silently failed to check bounds for UV (`vt_idx`) and Normal (`vn_idx`) indices *before* attempting to pack them into a 20-bit limited `VertexKey`.
**Outcome:** Feeding integers larger than the 20-bit `SENTINEL` (1,048,575) triggered a debug assertion panic (`assertion failed: k_vt <= SENTINEL`). In release mode without debug assertions, this would cause silent bitwise truncation leading to corrupted mesh output or out-of-bounds indexing.
**Resolution:** Explicit length-checks against `raw_uvs` and `raw_normals` are now enforced *before* packing the `VertexKey`. Added comprehensive `proptest` harness enforcing string-length torture and integer overflow bounds.

## [Vulnerability Found]
**Target:** `TileRenderer::submit_mesh`
**Trigger:** Submitting arbitrary vertex indices that exceed the underlying `vertices` slice length (e.g., `[0, 1, 9999]`).
**Flaw:** The function blindly trusts user-supplied indices and indexes directly into `vertices` without checking if the indices are within valid bounds. It delegates validation to Rust's native bounds checking logic.
**Outcome:** When out-of-bounds indices are supplied, the thread cleanly panics (`index out of bounds`). However, in parallel mode (`feature = "parallel"`), passing untrusted arrays will cause thread-level panics or potentially crash the runtime if unhandled.
**Resolution:** Purposefully documented as systemic fragility and left completely unfixed. An exploit test is included that deliberately sends a garbage index to trigger the panic.
