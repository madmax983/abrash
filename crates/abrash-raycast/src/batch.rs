//! Batch raycasting for ECS systems — scatter-gather pattern.
//!
//! Provides [`cast_rays_batch`] and [`cast_los_batch`] for processing many
//! raycasts in a single call. When the `parallel` feature is enabled, work
//! is distributed across threads via Rayon.

use abrash_core::bam::Bam;

use crate::cast::{cast_los, cast_ray};
use crate::map::ArrayGridMap;
use crate::types::{RayHit, Vec2Fixed};

/// Cast many rays in batch.
///
/// `rays` is a slice of `(origin, angle)` pairs. Results are written into
/// `results` at matching indices. The two slices must have the same length.
///
/// When the `parallel` feature is enabled, rays are dispatched across Rayon's
/// thread pool. Otherwise they are cast sequentially.
pub fn cast_rays_batch(
    map: &ArrayGridMap,
    rays: &[(Vec2Fixed, Bam)],
    results: &mut [Option<RayHit>],
) {
    debug_assert_eq!(rays.len(), results.len());

    #[cfg(feature = "parallel")]
    {
        use rayon::prelude::*;
        rays.par_iter()
            .zip(results.par_iter_mut())
            .for_each(|((origin, angle), result)| {
                *result = cast_ray(map, *origin, *angle);
            });
    }

    #[cfg(not(feature = "parallel"))]
    {
        for (i, (origin, angle)) in rays.iter().enumerate() {
            results[i] = cast_ray(map, *origin, *angle);
        }
    }
}

/// Check line-of-sight for many point pairs in batch.
///
/// `pairs` is a slice of `(from, to)` pairs. Results are written into
/// `results` at matching indices (`true` = clear LOS). The two slices must
/// have the same length.
///
/// When the `parallel` feature is enabled, checks are dispatched across
/// Rayon's thread pool. Otherwise they run sequentially.
pub fn cast_los_batch(map: &ArrayGridMap, pairs: &[(Vec2Fixed, Vec2Fixed)], results: &mut [bool]) {
    debug_assert_eq!(pairs.len(), results.len());

    #[cfg(feature = "parallel")]
    {
        use rayon::prelude::*;
        pairs
            .par_iter()
            .zip(results.par_iter_mut())
            .for_each(|((from, to), result)| {
                *result = cast_los(map, *from, *to);
            });
    }

    #[cfg(not(feature = "parallel"))]
    {
        for (i, (from, to)) in pairs.iter().enumerate() {
            results[i] = cast_los(map, *from, *to);
        }
    }
}

// ---------------------------------------------------------------------------
// Tests (TDD — written first)
// ---------------------------------------------------------------------------
#[cfg(test)]
mod tests {
    use super::*;
    use crate::map::ArrayGridMap;
    use crate::types::Cell;
    use abrash_core::bam::{ANG90, ANG180, ANG270};

    /// 8x8 map with walls around the border, empty inside.
    ///
    /// ```text
    /// ########
    /// #......#
    /// #......#
    /// #......#
    /// #......#
    /// #......#
    /// #......#
    /// ########
    /// ```
    fn border_map() -> ArrayGridMap {
        let mut map = ArrayGridMap::new(8, 8);
        for x in 0..8 {
            map.set(x, 0, Cell::Solid(1));
            map.set(x, 7, Cell::Solid(1));
        }
        for y in 0..8 {
            map.set(0, y, Cell::Solid(1));
            map.set(7, y, Cell::Solid(1));
        }
        map
    }

    #[test]
    fn batch_ray_matches_individual() {
        let map = border_map();

        let rays = [
            (Vec2Fixed::from_f32(1.5, 1.5), Bam::ZERO), // east
            (Vec2Fixed::from_f32(3.5, 3.5), ANG90),     // north
            (Vec2Fixed::from_f32(5.5, 5.5), ANG180),    // west
            (Vec2Fixed::from_f32(2.5, 2.5), ANG270),    // south
        ];

        // Individual results.
        let individual: Vec<Option<RayHit>> = rays
            .iter()
            .map(|(origin, angle)| cast_ray(&map, *origin, *angle))
            .collect();

        // Batch results.
        let mut batch = vec![None; rays.len()];
        cast_rays_batch(&map, &rays, &mut batch);

        assert_eq!(
            batch, individual,
            "batch results must match individual cast_ray results"
        );
    }

    #[test]
    fn batch_los_matches_individual() {
        let map = border_map();

        let pairs = [
            // Clear paths (interior to interior).
            (Vec2Fixed::from_f32(1.5, 1.5), Vec2Fixed::from_f32(6.5, 1.5)),
            (Vec2Fixed::from_f32(3.5, 3.5), Vec2Fixed::from_f32(3.5, 5.5)),
            // Blocked by border wall.
            (Vec2Fixed::from_f32(1.5, 1.5), Vec2Fixed::from_f32(1.5, 0.5)),
            // Same cell (trivially clear).
            (Vec2Fixed::from_f32(4.0, 4.0), Vec2Fixed::from_f32(4.0, 4.0)),
        ];

        let individual: Vec<bool> = pairs
            .iter()
            .map(|(from, to)| cast_los(&map, *from, *to))
            .collect();

        let mut batch = vec![false; pairs.len()];
        cast_los_batch(&map, &pairs, &mut batch);

        assert_eq!(
            batch, individual,
            "batch LOS results must match individual cast_los results"
        );
    }

    #[test]
    fn batch_empty_input() {
        let map = border_map();

        // Empty ray batch should not panic.
        let rays: &[(Vec2Fixed, Bam)] = &[];
        let mut ray_results: Vec<Option<RayHit>> = vec![];
        cast_rays_batch(&map, rays, &mut ray_results);
        assert!(ray_results.is_empty());

        // Empty LOS batch should not panic.
        let pairs: &[(Vec2Fixed, Vec2Fixed)] = &[];
        let mut los_results: Vec<bool> = vec![];
        cast_los_batch(&map, pairs, &mut los_results);
        assert!(los_results.is_empty());
    }
}
