# 👺 Havoc: Proving the Fragility of the System

This PR introduces a proptest demonstrating a fatal flaw in the `cull_aabbs_avx2` optimization.
The AVX2 SIMD path for AABB view frustum culling wildly diverges from the scalar fallback path.

We created `crates/abrash-core/tests/havoc_culling_proptest.rs` to expose the vulnerability.
When a random frustum configuration and randomly sized bounding box is queried, the AVX2 culling reports `false` (culled) while the mathematically correct scalar culling correctly reports `true` (visible).
