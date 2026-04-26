import re

# Fix bsp
with open("crates/abrash-raycast/src/bsp.rs", "r") as f:
    code = f.read()

code = code.replace("pub fn subsector_segs(&self, ssector_idx: usize) -> &[BspSeg] {", "/// Returns the segments of the given subsector.\n    ///\n    /// # Panics\n    ///\n    /// Panics if the subsector index is out of bounds.\n    #[must_use]\n    pub fn subsector_segs(&self, ssector_idx: usize) -> &[BspSeg] {")

with open("crates/abrash-raycast/src/bsp.rs", "w") as f:
    f.write(code)

with open("crates/abrash-skeletal/src/gltf_loader.rs", "r") as f:
    code = f.read()

# the diff tool is easier here.
