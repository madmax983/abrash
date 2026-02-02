# Bard's Journal - Critical Learnings

## 2024-05-23 - Vec3::normalize Magic Threshold
**Confusion:** Users might wonder why normalizing a very small vector returns the original vector instead of a zero vector or panicking.
**Clarification:** `Vec3::normalize` checks if the length is > 0.0001. If smaller, it returns the original vector to avoid division by zero or precision issues. This is a fail-safe but "magic" behavior.

## 2024-05-24 - Doctest Main Function
**Confusion:** Writing a "Quick Start" example that looks like a full program (with `fn main`) triggers Clippy's `needless_doctest_main` lint.
**Clarification:** Rust doc tests automatically wrap code in `fn main()`. For examples intended to be copy-pasted into `main.rs`, simply omitting `fn main` in the doctest is the preferred idiomatic way to avoid lints while keeping the example executable.
