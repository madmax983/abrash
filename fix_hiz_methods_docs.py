import re

with open("src/hiz_buffer.rs", "r") as f:
    content = f.read()

# Add doc comments to level_dimensions
content = re.sub(
    r'    #\[must_use\]\n    pub fn level_dimensions\(&self, level: u32\) -> Option<\(u32, u32\)> \{',
    r'    /// Gets the width and height of a specific pyramid level.\n    ///\n    /// Level 0 is the base full-resolution dimension. Each subsequent level is half the \n    /// dimensions of the previous level, rounded up.\n    ///\n    /// Returns `None` if the requested level exceeds the [`level_count`](Self::level_count).\n    #[must_use]\n    pub fn level_dimensions(&self, level: u32) -> Option<(u32, u32)> {',
    content
)

# Add doc comments to is_potentially_visible
content = re.sub(
    r'    #\[must_use\]\n    pub fn is_potentially_visible\(&self, aabb: AABB3D\) -> bool \{',
    r'    /// Tests if a given 3D bounding box ([`AABB3D`]) might be visible on screen.\n    ///\n    /// This function performs a conservative occlusion query against the hierarchical depth pyramid.\n    /// It determines the appropriate pyramid level based on the AABB size on-screen, and compares the \n    /// closest depth of the AABB with the farthest depth known in that coarse region.\n    ///\n    /// - Returns `true` if the object **might** be visible (or if the pyramid is invalid).\n    /// - Returns `false` if the object is **definitely** hidden behind existing geometry.\n    #[must_use]\n    pub fn is_potentially_visible(&self, aabb: AABB3D) -> bool {',
    content
)

# Add doc comments to is_coarse_bin_visible
content = re.sub(
    r'    #\[must_use\]\n    pub fn is_coarse_bin_visible\(&self, bin_aabb: AABB3D\) -> bool \{',
    r'    /// Tests if a coarse screen bin (128x128 pixels) is potentially visible.\n    ///\n    /// This is an optimization for two-level hierarchical binning. It quickly checks the \n    /// Hi-Z pyramid at Level 2 to see if an entire coarse tile of the screen is completely occluded.\n    /// If it is, the rasterizer can entirely skip processing fine bins (32x32 pixels) inside it.\n    #[must_use]\n    pub fn is_coarse_bin_visible(&self, bin_aabb: AABB3D) -> bool {',
    content
)

# Add doc comments to write_level_data
content = re.sub(
    r'    #\[cfg\(feature = "gpu-binning"\)\]\n    pub fn write_level_data\(&mut self, level: u32, data: &\[f32\]\) \{',
    r'    /// Writes depth data directly into a specific pyramid level.\n    ///\n    /// Primarily used by the GPU builder to download computed reduction data directly into \n    /// the CPU-side pyramid representation.\n    #[cfg(feature = "gpu-binning")]\n    pub fn write_level_data(&mut self, level: u32, data: &[f32]) {',
    content
)

# Add doc comments to mark_valid
content = re.sub(
    r'    #\[cfg\(feature = "gpu-binning"\)\]\n    pub fn mark_valid\(&mut self\) \{',
    r'    /// Marks the hierarchical depth pyramid as valid and ready for queries.\n    ///\n    /// Called internally after a successful GPU-side pyramid build.\n    #[cfg(feature = "gpu-binning")]\n    pub fn mark_valid(&mut self) {',
    content
)

with open("src/hiz_buffer.rs", "w") as f:
    f.write(content)
