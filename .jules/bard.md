# Bard's Journal - Critical Learnings

## 2024-05-23 - Vec3::normalize Magic Threshold
**Confusion:** Users might wonder why normalizing a very small vector returns the original vector instead of a zero vector or panicking.
**Clarification:** `Vec3::normalize` checks if the length is > 0.0001. If smaller, it returns the original vector to avoid division by zero or precision issues. This is a fail-safe but "magic" behavior.

## 2024-05-24 - The Missing Pipeline
**Confusion:** The `src/pipeline` directory existed but was not part of `lib.rs`, containing duplicative code of `src/rasterizer.rs`.
**Clarification:** `src/pipeline` was dead code. The actual rendering pipeline logic (projection, clipping, rasterization) lives in `src/rasterizer.rs`. `src/pipeline` has been removed to align the codebase with reality.
