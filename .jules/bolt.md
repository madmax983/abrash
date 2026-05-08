**[Dynamic Texture Reallocation Prevention]**
**Learning:** `Texture` structures, which contain potentially large vectors of `pixels` and `mips`, were missing an update path in `CpuRenderer` preventing efficient re-use of their allocations, unlike `CpuMesh` which used `clone_from`.
**Action:** Always provide `update_*` functions in rendering APIs to allow developers to leverage `std::clone::Clone::clone_from` for large inner buffers. This effectively turns dynamic updates of data structures like textures or meshes into zero-cost, zero-allocation operations per frame.
