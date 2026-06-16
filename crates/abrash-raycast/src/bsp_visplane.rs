//! Visplane allocator for BSP floor/ceiling rendering.
//!
//! Visplanes collect horizontal floor/ceiling strips during BSP wall rendering.
//! Each visplane groups columns sharing the same `(height, texture, light_level)`.
//! After walls are drawn, visplane spans are rendered as horizontal strips.
//!
//! This follows Doom's visplane algorithm: linear scan for matching planes,
//! with "visplane split" when a column is already occupied in a matching plane.

/// Sentinel value indicating an unused top row entry.
const UNUSED_TOP: i32 = i32::MAX;

/// Sentinel value indicating an unused bottom row entry.
const UNUSED_BOTTOM: i32 = i32::MIN;

/// A single visplane: horizontal surface at one height/texture/light.
///
/// Each visplane tracks per-column top and bottom screen rows. Columns that
/// have not been assigned a span use sentinel values (`i32::MAX` for top,
/// `i32::MIN` for bottom).
#[derive(Clone, Debug)]
pub struct Visplane {
    /// The height of the plane in world units.
    pub height: i16,
    /// The flat texture index applied to this plane.
    pub texture: u16,
    /// The ambient light level of the sector this plane belongs to.
    pub light_level: u16,
    /// The minimum screen column this plane is visible on.
    pub min_x: i32,
    /// The maximum screen column this plane is visible on.
    pub max_x: i32,
    top: Vec<i32>,
    bottom: Vec<i32>,
}

impl Visplane {
    /// Create a new visplane with the given properties.
    ///
    /// All columns start as unused (top = `i32::MAX`, bottom = `i32::MIN`).
    fn new(height: i16, texture: u16, light_level: u16, screen_width: u32) -> Self {
        let w = screen_width as usize;
        Self {
            height,
            texture,
            light_level,
            min_x: i32::MAX,
            max_x: i32::MIN,
            top: vec![UNUSED_TOP; w],
            bottom: vec![UNUSED_BOTTOM; w],
        }
    }

    /// Return the top screen row for `col`, or `None` if the column is unused.
    #[must_use]
    pub fn top(&self, col: i32) -> Option<i32> {
        let idx = usize::try_from(col).ok()?;
        let val = *self.top.get(idx)?;
        if val == UNUSED_TOP { None } else { Some(val) }
    }

    /// Return the bottom screen row for `col`, or `None` if the column is unused.
    #[must_use]
    pub fn bottom(&self, col: i32) -> Option<i32> {
        let idx = usize::try_from(col).ok()?;
        let val = *self.bottom.get(idx)?;
        if val == UNUSED_BOTTOM {
            None
        } else {
            Some(val)
        }
    }

    /// Returns `true` if the given column is unused in this visplane.
    fn is_column_free(&self, col: i32) -> bool {
        let Ok(idx) = usize::try_from(col) else {
            return false;
        };
        self.top.get(idx).copied() == Some(UNUSED_TOP)
    }
}

/// Allocator that manages a pool of visplanes during a single frame.
///
/// Usage pattern:
/// 1. Call `find_or_create` for each floor/ceiling column encountered during
///    BSP traversal.
/// 2. Call `set_span` to record the screen-space top/bottom for that column.
/// 3. After all walls are processed, iterate `planes` to draw horizontal spans.
/// 4. Call `reset` before the next frame.
pub struct VisplaneAllocator {
    planes: Vec<Visplane>,
    screen_width: u32,
}

impl VisplaneAllocator {
    /// Create a new allocator for the given screen width (in pixels/columns).
    #[must_use]
    pub const fn new(screen_width: u32) -> Self {
        Self {
            planes: Vec::new(),
            screen_width,
        }
    }

    /// ⚡ Bolt: Create a new allocator with pre-allocated capacity for planes.
    /// This eliminates dynamic heap reallocations per frame since doom typically uses < 128 visplanes.
    #[must_use]
    pub fn with_capacity(capacity: usize, screen_width: u32) -> Self {
        Self {
            planes: Vec::with_capacity(capacity),
            screen_width,
        }
    }

    /// Find a matching `(height, texture, light)` visplane where `col` is free,
    /// or create a new one.
    ///
    /// If a matching visplane exists but `col` is already used in it, a new
    /// visplane is created (Doom's "visplane split" behavior). This linear scan
    /// mirrors the original Doom algorithm -- typically fewer than 128 visplanes
    /// per frame.
    pub fn find_or_create(&mut self, height: i16, texture: u16, light: u16, col: i32) -> usize {
        // Linear scan for a matching plane with the column still free.
        for (i, plane) in self.planes.iter().enumerate() {
            if plane.height == height
                && plane.texture == texture
                && plane.light_level == light
                && plane.is_column_free(col)
            {
                return i;
            }
        }

        // No suitable match -- allocate a new visplane.
        let idx = self.planes.len();
        self.planes
            .push(Visplane::new(height, texture, light, self.screen_width));
        idx
    }

    /// Record the top and bottom screen rows for a column in the given visplane.
    ///
    /// Also updates the visplane's `min_x` / `max_x` bounding range.
    pub fn set_span(&mut self, plane_idx: usize, col: i32, top: i32, bottom: i32) {
        if col < 0 || col as u32 >= self.screen_width {
            return;
        }
        let plane = &mut self.planes[plane_idx];
        let idx = col as usize;
        plane.top[idx] = top;
        plane.bottom[idx] = bottom;
        plane.min_x = plane.min_x.min(col);
        plane.max_x = plane.max_x.max(col);
    }

    /// Return the current set of visplanes.
    #[must_use]
    pub fn planes(&self) -> &[Visplane] {
        &self.planes
    }

    /// Clear all visplanes for the next frame.
    pub fn reset(&mut self) {
        self.planes.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allocator_starts_empty() {
        let alloc = VisplaneAllocator::with_capacity(128, 320);
        assert!(alloc.planes().is_empty());
    }

    #[test]
    fn find_or_create_makes_new_plane() {
        let mut alloc = VisplaneAllocator::with_capacity(128, 320);
        let idx = alloc.find_or_create(128, 1, 200, 10);
        assert_eq!(idx, 0);
        assert_eq!(alloc.planes().len(), 1);
    }

    #[test]
    fn find_or_create_reuses_matching_plane() {
        let mut alloc = VisplaneAllocator::with_capacity(128, 320);
        let idx0 = alloc.find_or_create(128, 1, 200, 10);
        alloc.set_span(idx0, 10, 50, 100);

        // Same properties, different column -- should reuse.
        let idx1 = alloc.find_or_create(128, 1, 200, 20);
        assert_eq!(idx0, idx1);
        assert_eq!(alloc.planes().len(), 1);
    }

    #[test]
    fn find_or_create_splits_on_column_conflict() {
        let mut alloc = VisplaneAllocator::with_capacity(128, 320);
        let idx0 = alloc.find_or_create(128, 1, 200, 10);
        alloc.set_span(idx0, 10, 50, 100);

        // Same properties, SAME column (already used) -- must split.
        let idx1 = alloc.find_or_create(128, 1, 200, 10);
        assert_ne!(idx0, idx1);
        assert_eq!(alloc.planes().len(), 2);
    }

    #[test]
    fn find_or_create_different_properties() {
        let mut alloc = VisplaneAllocator::with_capacity(128, 320);
        let idx0 = alloc.find_or_create(128, 1, 200, 10);
        let idx1 = alloc.find_or_create(64, 2, 180, 10);
        assert_ne!(idx0, idx1);
        assert_eq!(alloc.planes().len(), 2);
    }

    #[test]
    fn set_span_updates_bounds() {
        let mut alloc = VisplaneAllocator::with_capacity(128, 320);
        let idx = alloc.find_or_create(128, 1, 200, 0);

        alloc.set_span(idx, 50, 10, 80);
        alloc.set_span(idx, 100, 20, 90);

        let plane = &alloc.planes()[idx];
        assert_eq!(plane.min_x, 50);
        assert_eq!(plane.max_x, 100);
        assert_eq!(plane.top(50), Some(10));
        assert_eq!(plane.bottom(50), Some(80));
        assert_eq!(plane.top(100), Some(20));
        assert_eq!(plane.bottom(100), Some(90));
    }

    #[test]
    fn visplane_unused_columns_return_none() {
        let mut alloc = VisplaneAllocator::with_capacity(128, 320);
        let idx = alloc.find_or_create(128, 1, 200, 0);
        alloc.set_span(idx, 10, 30, 60);

        let plane = &alloc.planes()[idx];
        // Column 5 was never set.
        assert_eq!(plane.top(5), None);
        assert_eq!(plane.bottom(5), None);
        // Column 10 was set.
        assert_eq!(plane.top(10), Some(30));
        assert_eq!(plane.bottom(10), Some(60));
    }

    #[test]
    fn reset_clears_all() {
        let mut alloc = VisplaneAllocator::with_capacity(128, 320);
        alloc.find_or_create(128, 1, 200, 10);
        alloc.find_or_create(64, 2, 180, 20);
        assert_eq!(alloc.planes().len(), 2);

        alloc.reset();
        assert!(alloc.planes().is_empty());
    }
}
