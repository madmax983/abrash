## 2024-05-15 - Documenting Structs and their fields
**Confusion:** The users were confused about the purpose of individual fields on generic math and geometry structs (e.g. `Vec2`, `Vec3`, `Mat4`, `AABB`, `Texture`).
**Clarification:** Added comprehensive documentation strings (`///`) to the public fields of structs in `math.rs`, `geometry.rs`, `texture.rs`, `quat.rs` and `transform.rs` and documented functions. Added `#[must_use]` attributes where appropriate.
