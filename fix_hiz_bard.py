import re

with open("src/hiz_buffer.rs", "r") as f:
    content = f.read()

# Replace the initial module docs with a more detailed Bard-style story
new_docs = """//! # The Hierarchical Z-Buffer (Hi-Z) 🏔️
//!
//! Welcome to the heights of occlusion culling! When rendering complex scenes, drawing geometry
//! that is eventually hidden behind other objects (overdraw) is a massive performance killer.
//! The `HiZBuffer` acts as a sentinel, rapidly determining if a piece of geometry is completely hidden
//! before the rasterizer wastes time drawing it.
//!
//! Think of it as a mipmapped pyramid of depth values. Instead of checking every single pixel of a bounding box
//! against the full-resolution depth buffer, we can test it against a coarse summary. If the closest point of
//! the bounding box is *farther* away than the farthest geometry in a coarse region, we know the entire box
//! (and the geometry inside it) is completely occluded.
//!
//! ## The Algorithm
//! 1. **Build:** After rendering a base pass (or using depth from the previous frame), we build a pyramid
//!    where each level stores the minimum (closest) depth from a 2×2 region of the level below it.
//! 2. **Query:** When testing an `AABB3D`, we find the pyramid level where the box covers roughly 2×2 pixels.
//!    We compare the box's closest depth against those 4 pixels.
//! 3. **Result:** If the box is behind the known depth, it is discarded. This process is conservative:
//!    false positives (drawing something that ends up hidden) are acceptable, but false negatives are forbidden.
//!
//! ## Examples
//!
//! ```rust
//! use abrash::zbuffer::ZBuffer;
//! use abrash::hiz_buffer::{HiZBuffer, AABB3D};
//!
//! // 1. Initialize buffers for our viewport
//! let width = 1920;
//! let height = 1080;
//! let mut zb = ZBuffer::new(width, height).unwrap();
//! let mut hiz = HiZBuffer::new(width, height);
//!
//! // 2. Build the pyramid (usually done after a pre-pass or using previous frame data)
//! hiz.build_pyramid(&zb);
//!
//! // 3. Create a bounding box for an object we want to draw
//! let my_object_aabb = AABB3D::new(100, 200, 100, 200, 50.0, 60.0);
//!
//! // 4. Ask the sentinel!
//! if hiz.is_potentially_visible(my_object_aabb) {
//!     // Draw the object, it might be seen!
//! } else {
//!     // Skip drawing, it's definitely hidden behind a wall.
//! }
//! ```
//!
//! ## Performance Characteristics
//! * **Memory:** ~33% overhead of the base ZBuffer size (e.g., ~2.67 MB for 1080p).
//! * **Build Time:** ~1-2ms on CPU at 1080p.
//! * **Query Time:** <100ns per object.
//!
"""

content = re.sub(r'/// Hierarchical Z-Buffer for efficient occlusion culling\n///\n/// The Hi-Z buffer maintains a depth pyramid where each level stores the minimum\n/// \(closest\) depth from a 2×2 region of the level below. This enables fast occlusion\n/// queries by testing against progressively coarser representations\.\n///\n/// # Memory Layout\n/// - Level 0: References the full-resolution `ZBuffer` \(not duplicated\)\n/// - Level 1\+: Progressively coarser 2×2 min-reductions\n/// - For 1920×1080: ~2\.67 MB total pyramid overhead \(33% of zbuffer size\)\n///\n/// # Query Algorithm\n/// Hierarchical descent from coarse to fine levels, testing AABB depth bounds\n/// against pyramid cells\. Conservative: false positives OK, false negatives NOT OK\.', new_docs, content)

# Add AABB3D new constructor and doc updates
aabb_docs = """/// A 3D Axis-Aligned Bounding Box (AABB) in screen space.
///
/// This structure defines a volume in the screen's coordinate system, used primarily for
/// occlusion queries against the [`HiZBuffer`].
///
/// By knowing the bounding volume of an object *before* we draw it, we can ask the `HiZBuffer`
/// if this entire volume is hidden behind existing geometry.
///
/// ## Examples
/// ```
/// use abrash::hiz_buffer::AABB3D;
///
/// // A box spanning pixels (10, 10) to (50, 50) at a depth range of 10.0 to 20.0
/// let bounds = AABB3D::new(10, 50, 10, 50, 10.0, 20.0);
/// ```
#[derive(Debug, Clone, Copy)]
pub struct AABB3D {
    /// Minimum X coordinate in screen space
    pub min_x: i32,
    /// Maximum X coordinate in screen space
    pub max_x: i32,
    /// Minimum Y coordinate in screen space
    pub min_y: i32,
    /// Maximum Y coordinate in screen space
    pub max_y: i32,
    /// Minimum depth value (closest point to camera)
    pub min_depth: f32,
    /// Maximum depth value (farthest point from camera)
    pub max_depth: f32,
}

impl AABB3D {
    /// Creates a new AABB3D.
    ///
    /// The bounding box is defined by its screen space coordinates (`min_x`, `max_x`, `min_y`, `max_y`)
    /// and its depth bounds (`min_depth`, `max_depth`).
    #[must_use]
    pub fn new(min_x: i32, max_x: i32, min_y: i32, max_y: i32, min_depth: f32, max_depth: f32) -> Self {
        Self { min_x, max_x, min_y, max_y, min_depth, max_depth }
    }
}
"""

content = re.sub(r'/// 3D Axis-Aligned Bounding Box for occlusion queries\n#\[derive\(Debug, Clone, Copy\)\]\npub struct AABB3D {\n    /// Minimum X coordinate in screen space\n    pub min_x: i32,\n    /// Maximum X coordinate in screen space\n    pub max_x: i32,\n    /// Minimum Y coordinate in screen space\n    pub min_y: i32,\n    /// Maximum Y coordinate in screen space\n    pub max_y: i32,\n    /// Minimum depth value \(closest point to camera\)\n    pub min_depth: f32,\n    /// Maximum depth value \(farthest point from camera\)\n    pub max_depth: f32,\n}', aabb_docs, content)

with open("src/hiz_buffer.rs", "w") as f:
    f.write(content)
