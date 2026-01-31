//! Scanline iterator for triangle rasterization.
//!
//! Encapsulates the logic for iterating over scanlines of a triangle
//! and calculating interpolation factors (alpha, beta).

#[derive(Debug, Clone, Copy)]
pub struct ScanlineStep {
    /// Current scanline Y coordinate
    pub y: i32,
    /// Interpolation factor along the long edge (v0-v2)
    pub alpha: f32,
    /// Interpolation factor along the active short edge (v0-v1 or v1-v2)
    pub beta: f32,
    /// True if we are processing the second half of the triangle (v1-v2)
    pub second_half: bool,
}

pub struct ScanlineIter {
    current_y: i32,
    end_y: i32,
    y0: i32,
    y1: i32,
    y2: i32,
    total_height: f32,
}

impl ScanlineIter {
    /// Create a new scanline iterator.
    ///
    /// # Arguments
    ///
    /// * `y0, y1, y2` - Y coordinates of the triangle vertices, sorted such that y0 <= y1 <= y2.
    /// * `min_y, max_y` - Vertical clipping bounds (e.g., 0 and height-1).
    pub fn new(y0: i32, y1: i32, y2: i32, min_y: i32, max_y: i32) -> Self {
        let start_y = y0.max(min_y);
        let end_y = y2.min(max_y);
        let total_height = (y2 - y0) as f32;

        Self {
            current_y: start_y,
            end_y,
            y0,
            y1,
            y2,
            total_height,
        }
    }
}

impl Iterator for ScanlineIter {
    type Item = ScanlineStep;

    fn next(&mut self) -> Option<Self::Item> {
        if self.total_height == 0.0 {
            return None;
        }

        loop {
            if self.current_y > self.end_y {
                return None;
            }

            let y = self.current_y;
            self.current_y += 1;

            let second_half = y > self.y1 || self.y1 == self.y0;
            let segment_height = if second_half {
                self.y2 - self.y1
            } else {
                self.y1 - self.y0
            };

            // Skip scanlines where the active short edge has zero height.
            if segment_height == 0 {
                continue;
            }

            let alpha = (y - self.y0) as f32 / self.total_height;
            let beta = if second_half {
                (y - self.y1) as f32 / segment_height as f32
            } else {
                (y - self.y0) as f32 / segment_height as f32
            };

            return Some(ScanlineStep {
                y,
                alpha,
                beta,
                second_half,
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scanline_iter_basic() {
        let iter = ScanlineIter::new(0, 5, 10, 0, 100);
        let steps: Vec<_> = iter.collect();

        assert_eq!(steps.len(), 11); // 0 to 10 inclusive
        assert_eq!(steps[0].y, 0);
        assert_eq!(steps[0].alpha, 0.0);
        assert_eq!(steps[0].beta, 0.0);
        assert!(!steps[0].second_half);

        assert_eq!(steps[10].y, 10);
        assert_eq!(steps[10].alpha, 1.0);
        // At y=10 (v2), beta should be 1.0 (end of v1-v2)
        assert_eq!(steps[10].beta, 1.0);
        assert!(steps[10].second_half);
    }

    #[test]
    fn test_scanline_iter_clamped() {
        // Triangle from 0 to 10, but clamped to 2..8
        let iter = ScanlineIter::new(0, 5, 10, 2, 8);
        let steps: Vec<_> = iter.collect();

        assert_eq!(steps.len(), 7); // 2,3,4,5,6,7,8
        assert_eq!(steps[0].y, 2);
        assert_eq!(steps.last().unwrap().y, 8);

        // Check alpha at start (y=2). total=10. alpha = 2/10 = 0.2
        assert!((steps[0].alpha - 0.2).abs() < 0.001);
    }

    #[test]
    fn test_degenerate_triangle() {
        // Height 0
        let iter = ScanlineIter::new(5, 5, 5, 0, 10);
        let steps: Vec<_> = iter.collect();
        assert!(steps.is_empty());
    }

    #[test]
    fn test_flat_top() {
        // v0.y = v1.y = 0, v2.y = 10
        let iter = ScanlineIter::new(0, 0, 10, 0, 100);
        let steps: Vec<_> = iter.collect();

        assert_eq!(steps.len(), 11);
        // All should be second_half because y >= y1 (0) and y1 == y0
        assert!(steps[0].second_half);
    }

    #[test]
    fn test_flat_bottom() {
        // v0.y = 0, v1.y = v2.y = 10
        let iter = ScanlineIter::new(0, 10, 10, 0, 100);
        let steps: Vec<_> = iter.collect();

        assert_eq!(steps.len(), 11);
        // All should be !second_half because y <= y1 (10)
        // Except potentially the last one?
        // If y=10, y > y1 (10>10 false), y1==y0 (10==0 false). So second_half=false.
        // Wait, logic: y > y1 || y1 == y0.
        // at y=10: 10 > 10 is false. 10 == 0 is false. So first half.
        // But if first half, segment height is y1-y0 = 10-0 = 10. Valid.
        assert!(!steps[0].second_half);
        assert!(!steps[10].second_half);
    }
}
