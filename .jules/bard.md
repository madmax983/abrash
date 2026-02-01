# Bard's Journal - Critical Learnings

## 2024-05-23 - Vec3::normalize Magic Threshold
**Confusion:** Users might wonder why normalizing a very small vector returns the original vector instead of a zero vector or panicking.
**Clarification:** `Vec3::normalize` checks if the length is > 0.0001. If smaller, it returns the original vector to avoid division by zero or precision issues. This is a fail-safe but "magic" behavior.
