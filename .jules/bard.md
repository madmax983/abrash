# Bard's Journal - Critical Learnings

## 2024-05-23 - Vec3::normalize Magic Threshold
**Confusion:** Users might wonder why normalizing a very small vector returns the original vector instead of a zero vector or panicking.
**Clarification:** `Vec3::normalize` checks if the length is > 0.0001. If smaller, it returns the original vector to avoid division by zero or precision issues. This is a fail-safe but "magic" behavior.

## 2024-05-24 - Matrix Multiplication Order
**Confusion:** Users (and Bard) were confused about the order of matrix multiplication for . The conceptual example used , but the row-major implementation requires .
**Clarification:** Updated documentation and examples to explicitly state 32 files to edit and the row-vector convention.

## 2024-05-25 - The Hidden PBR Rasterizer
**Confusion:** Users looking for realistic rendering might miss the Physically Based Rendering implementation because it wasn't listed in the main documentation or module summaries.
**Clarification:** The `pbr` module in `src/rasterizer/pbr.rs` contains a fully functional Cook-Torrance BRDF rasterizer (`fill_triangle_pbr`). Updated docs to feature this prominently.
