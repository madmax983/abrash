import re

# Fix bsp
with open("crates/abrash-raycast/src/bsp.rs", "r") as f:
    code = f.read()

code = code.replace("pub fn subsector_segs(&self, ssector_idx: usize) -> &[BspSeg]", "#[must_use]\n    pub fn subsector_segs(&self, ssector_idx: usize) -> &[BspSeg]")
code = code.replace("pub fn seg_front_sector(&self, _seg: &BspSeg) -> &BspSector", "#[must_use]\n    pub const fn seg_front_sector(&self, _seg: &BspSeg) -> &BspSector")
code = code.replace("pub fn seg_back_sector(&self, seg: &BspSeg) -> Option<&BspSector>", "#[must_use]\n    pub fn seg_back_sector(&self, seg: &BspSeg) -> Option<&BspSector>")

with open("crates/abrash-raycast/src/bsp.rs", "w") as f:
    f.write(code)

with open("crates/abrash-raycast/src/renderer/bsp.rs", "r") as f:
    code = f.read()

code = code.replace("pub fn render_bsp_view(", "#[allow(clippy::too_many_lines)]\npub fn render_bsp_view(")

with open("crates/abrash-raycast/src/renderer/bsp.rs", "w") as f:
    f.write(code)

with open("crates/abrash-raycast/src/renderer/bsp_lighting.rs", "r") as f:
    code = f.read()

code = code.replace("pub fn wall_column(&self, _texture_id: u16, _col: usize) -> &[u8]", "#[must_use]\n    pub fn wall_column(&self, _texture_id: u16, _col: usize) -> &[u8]")
code = code.replace("pub fn flat_data(&self, _texture_id: u16) -> &[u8; 4096]", "#[must_use]\n    pub const fn flat_data(&self, _texture_id: u16) -> &[u8; 4096]")
code = code.replace("pub fn colormap(&self, index: u8) -> &[u8; 256]", "#[must_use]\n    pub fn colormap(&self, index: u8) -> &[u8; 256]")
code = code.replace("pub fn palette_argb(&self, palette_idx: u8) -> u32", "#[must_use]\n    pub const fn palette_argb(&self, palette_idx: u8) -> u32")

with open("crates/abrash-raycast/src/renderer/bsp_lighting.rs", "w") as f:
    f.write(code)
