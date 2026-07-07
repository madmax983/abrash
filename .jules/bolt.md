**[Optimizing angle_between]**
**Learning:** `Vec2::angle_between` and `Vec3::angle_between` were doing two separate `sqrt` calls: `self.length() * other.length()`.
**Action:** Replaced it with calculating the squared lengths, taking their product, and passing it through `fast_inv_sqrt`.

**[Removing redundant reserve calls]**
**Learning:** `scene.rs` was calling `.reserve()` on its vectors, followed immediately by `.reserve_exact()`.
**Action:** Removed the redundant `.reserve()` calls.
