# Bolt's Journal

**Learning:** Bounds checking in inner rasterization loops is a major bottleneck.
**Action:** When optimizing hot loops like `fill_triangle`, clamp coordinates to screen bounds *outside* the loop and use `unsafe` unchecked access inside. This yielded a ~73% speedup.

**Learning:** `fill_triangle_3d` had a hidden per-pixel division for Z-interpolation.
**Action:** Always look for invariants in loops. `dz/dx` is constant across a scanline; pre-calculating it removed a division per pixel.

**Learning:** Windows-specific crates break `cargo bench` on Linux.
**Action:** Provide dummy implementations for platform-specific modules behind `#[cfg(not(target_os = "windows"))]` to allow CI/benchmarking to run everywhere.

**Learning:** Replacing per-scanline division with incremental addition improves performance for small triangles but can regress large triangles due to setup overhead or instruction cache/inlining issues.
**Action:** Always verify both small and large inputs. Ensure hot inner loop functions (like `test_and_set_unchecked`) are `#[inline]`'d when changing the call site structure.
