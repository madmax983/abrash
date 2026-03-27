# BSP Raycaster Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add BSP traversal types and traits to `abrash-raycast`, and a Doom-compatible BSP software renderer to `abrash-render`, enabling any BSP-based game to use abrash as its rendering backend.

**Architecture:** BSP spatial types and traits live in `abrash-raycast` (parallel to the existing DDA modules). The BSP column/span renderer lives in `abrash-render/src/raycaster/bsp.rs` (parallel to `hybrid.rs`). Consumer games (e.g., doom-rs) implement `BspMap` and `BspTextures` traits. Zero-alloc visitor-based BSP traversal. Doom-compatible projection math (R_ScaleFromGlobalAngle).

**Tech Stack:** Rust 2024 edition, `Fixed16_16` (16.16 fixed-point), `Bam` (u32 binary angle), abrash-core (Framebuffer, ZBuffer), proptest for property tests.

**Design Doc:** User-provided BSP raycaster design document (2026-03-26).

---

## Task 1: BSP Data Types

**Files:**
- Create: `crates/abrash-raycast/src/bsp.rs`
- Modify: `crates/abrash-raycast/src/lib.rs` (add `pub mod bsp;`)

**Step 1: Write failing tests**

Create `crates/abrash-raycast/src/bsp.rs` with types and test module:

```rust
//! BSP tree types for Doom-style map traversal.
//!
//! Parallel to the DDA grid raycaster — both answer "what does this ray hit?"
//! but BSP iterates segs front-to-back while DDA steps through grid cells.

use abrash_core::bam::Bam;
use abrash_core::fixed16_16::Fixed16_16;
use crate::types::Vec2Fixed;

/// A wall segment in BSP space.
///
/// Segs are the atomic rendering unit: each represents a visible portion of a
/// linedef, carrying texture IDs and sector references. In Doom's data format,
/// segs within a subsector are contiguous in memory.
#[derive(Clone, Debug)]
pub struct BspSeg {
    /// Start vertex in world space.
    pub v1: Vec2Fixed,
    /// End vertex in world space.
    pub v2: Vec2Fixed,
    /// Texture offset along the seg (for texture alignment).
    pub offset: Fixed16_16,
    /// Angle of the seg (direction from v1 to v2).
    pub angle: Bam,
    /// Index of the front sector (always valid).
    pub front_sector: u16,
    /// Index of the back sector (None = one-sided / solid wall).
    pub back_sector: Option<u16>,
    /// Upper texture ID (pegging above portal openings). 0 = none.
    pub upper_texture: u16,
    /// Middle texture ID (the main wall face). 0 = none.
    pub middle_texture: u16,
    /// Lower texture ID (pegging below portal openings). 0 = none.
    pub lower_texture: u16,
    /// Line flags (ML_BLOCKING, ML_TWOSIDED, etc.).
    pub line_flags: u16,
}

/// Sector geometry and appearance.
///
/// Sectors define floor/ceiling heights, lighting, and textures. Every seg
/// references a front sector (and optionally a back sector for portals).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BspSector {
    /// Floor height in map units.
    pub floor_height: i16,
    /// Ceiling height in map units.
    pub ceil_height: i16,
    /// Light level (0 = dark, 255 = full bright).
    pub light_level: u16,
    /// Floor flat texture ID.
    pub floor_texture: u16,
    /// Ceiling flat texture ID.
    pub ceil_texture: u16,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bsp_seg_solid_wall() {
        let seg = BspSeg {
            v1: Vec2Fixed::from_ints(0, 0),
            v2: Vec2Fixed::from_ints(256, 0),
            offset: Fixed16_16::ZERO,
            angle: Bam::ZERO,
            front_sector: 0,
            back_sector: None,
            upper_texture: 0,
            middle_texture: 1,
            lower_texture: 0,
            line_flags: 0,
        };
        assert!(seg.back_sector.is_none(), "solid wall has no back sector");
        assert_eq!(seg.middle_texture, 1);
    }

    #[test]
    fn bsp_seg_two_sided() {
        let seg = BspSeg {
            v1: Vec2Fixed::from_ints(64, 0),
            v2: Vec2Fixed::from_ints(192, 0),
            offset: Fixed16_16::ZERO,
            angle: Bam::ZERO,
            front_sector: 0,
            back_sector: Some(1),
            upper_texture: 2,
            middle_texture: 0,
            lower_texture: 3,
            line_flags: 0x04, // ML_TWOSIDED
        };
        assert_eq!(seg.back_sector, Some(1));
        assert_eq!(seg.upper_texture, 2);
        assert_eq!(seg.lower_texture, 3);
    }

    #[test]
    fn bsp_sector_construction() {
        let sector = BspSector {
            floor_height: 0,
            ceil_height: 128,
            light_level: 160,
            floor_texture: 1,
            ceil_texture: 2,
        };
        assert_eq!(sector.ceil_height - sector.floor_height, 128);
        assert_eq!(sector.light_level, 160);
    }

    #[test]
    fn bsp_sector_negative_floor() {
        let sector = BspSector {
            floor_height: -64,
            ceil_height: 64,
            light_level: 255,
            floor_texture: 0,
            ceil_texture: 0,
        };
        assert_eq!(sector.ceil_height - sector.floor_height, 128);
    }
}
```

**Step 2: Run tests to verify they fail**

Run: `cargo test -p abrash-raycast bsp::tests --lib`
Expected: PASS (these are construction tests — they verify the types compile and hold values correctly. The "RED" for the BspMap trait is in Task 2.)

**Step 3: Add module to lib.rs**

In `crates/abrash-raycast/src/lib.rs`, add:
```rust
pub mod bsp;
```

**Step 4: Run tests to verify pass**

Run: `cargo test -p abrash-raycast --lib`
Expected: All existing tests (33) + new tests (4) = 37 tests PASS.

**Step 5: Commit**

```bash
git add crates/abrash-raycast/src/bsp.rs crates/abrash-raycast/src/lib.rs
git commit -m "feat(raycast): add BSP data types — BspSeg, BspSector"
```

---

## Task 2: BspMap Trait

**Files:**
- Modify: `crates/abrash-raycast/src/bsp.rs`

**Step 1: Write failing test using BspMap trait**

Add the trait definition and a MockBspMap test to `bsp.rs`:

```rust
// --- Add above the tests module ---

/// A BSP-structured map that can be traversed front-to-back.
///
/// Consumers (e.g., doom-rs) implement this trait as a thin wrapper around
/// their BSP tree. The trait is designed for static dispatch (`&impl BspMap`)
/// — object safety is not required.
pub trait BspMap {
    /// Traverse the BSP tree front-to-back from the camera position.
    ///
    /// Calls `visitor` with each subsector index in front-to-back order.
    /// The visitor receives subsector indices — use `subsector_segs()` to
    /// get the actual segs for rendering.
    fn traverse_front_to_back(&self, pos: Vec2Fixed, visitor: &mut dyn FnMut(usize));

    /// Get the segs belonging to a subsector (contiguous slice).
    fn subsector_segs(&self, ssector_idx: usize) -> &[BspSeg];

    /// Get the front sector for a seg (always valid).
    fn seg_front_sector(&self, seg: &BspSeg) -> &BspSector;

    /// Get the back sector for a two-sided seg (portal), if any.
    fn seg_back_sector(&self, seg: &BspSeg) -> Option<&BspSector>;
}
```

Add test helper and tests inside `#[cfg(test)] mod tests`:

```rust
    // --- Test helper: single-room mock BSP map ---

    /// A minimal BSP map: one rectangular room with 4 solid walls.
    /// Room is 512x512 map units centered at (256, 256).
    /// Single subsector (index 0) containing 4 segs.
    /// Single sector: floor=0, ceil=128, light=160.
    struct MockBspMap {
        sector: BspSector,
        segs: Vec<BspSeg>,
    }

    impl MockBspMap {
        fn single_room() -> Self {
            let sector = BspSector {
                floor_height: 0,
                ceil_height: 128,
                light_level: 160,
                floor_texture: 1,
                ceil_texture: 2,
            };
            // Four walls: south, east, north, west (clockwise, normals face inward)
            let segs = vec![
                // South wall (y=0, facing north / inward)
                BspSeg {
                    v1: Vec2Fixed::from_ints(0, 0),
                    v2: Vec2Fixed::from_ints(512, 0),
                    offset: Fixed16_16::ZERO,
                    angle: Bam::ZERO,
                    front_sector: 0,
                    back_sector: None,
                    upper_texture: 0,
                    middle_texture: 1,
                    lower_texture: 0,
                    line_flags: 0,
                },
                // East wall (x=512, facing west / inward)
                BspSeg {
                    v1: Vec2Fixed::from_ints(512, 0),
                    v2: Vec2Fixed::from_ints(512, 512),
                    offset: Fixed16_16::ZERO,
                    angle: Bam::from_raw(0x4000_0000), // ANG90
                    front_sector: 0,
                    back_sector: None,
                    upper_texture: 0,
                    middle_texture: 1,
                    lower_texture: 0,
                    line_flags: 0,
                },
                // North wall (y=512, facing south / inward)
                BspSeg {
                    v1: Vec2Fixed::from_ints(512, 512),
                    v2: Vec2Fixed::from_ints(0, 512),
                    offset: Fixed16_16::ZERO,
                    angle: Bam::from_raw(0x8000_0000), // ANG180
                    front_sector: 0,
                    back_sector: None,
                    upper_texture: 0,
                    middle_texture: 1,
                    lower_texture: 0,
                    line_flags: 0,
                },
                // West wall (x=0, facing east / inward)
                BspSeg {
                    v1: Vec2Fixed::from_ints(0, 512),
                    v2: Vec2Fixed::from_ints(0, 0),
                    offset: Fixed16_16::ZERO,
                    angle: Bam::from_raw(0xC000_0000), // ANG270
                    front_sector: 0,
                    back_sector: None,
                    upper_texture: 0,
                    middle_texture: 1,
                    lower_texture: 0,
                    line_flags: 0,
                },
            ];
            Self { sector, segs }
        }
    }

    impl BspMap for MockBspMap {
        fn traverse_front_to_back(&self, _pos: Vec2Fixed, visitor: &mut dyn FnMut(usize)) {
            // Single subsector — always visit index 0.
            visitor(0);
        }

        fn subsector_segs(&self, ssector_idx: usize) -> &[BspSeg] {
            assert_eq!(ssector_idx, 0, "MockBspMap has only one subsector");
            &self.segs
        }

        fn seg_front_sector(&self, _seg: &BspSeg) -> &BspSector {
            &self.sector
        }

        fn seg_back_sector(&self, seg: &BspSeg) -> Option<&BspSector> {
            // Single sector — no back sectors.
            None
        }
    }

    #[test]
    fn mock_traversal_visits_subsector() {
        let map = MockBspMap::single_room();
        let mut visited = Vec::new();
        map.traverse_front_to_back(Vec2Fixed::from_ints(256, 256), &mut |idx| {
            visited.push(idx);
        });
        assert_eq!(visited, vec![0]);
    }

    #[test]
    fn mock_subsector_has_four_segs() {
        let map = MockBspMap::single_room();
        assert_eq!(map.subsector_segs(0).len(), 4);
    }

    #[test]
    fn mock_seg_front_sector_returns_room() {
        let map = MockBspMap::single_room();
        let seg = &map.subsector_segs(0)[0];
        let sector = map.seg_front_sector(seg);
        assert_eq!(sector.floor_height, 0);
        assert_eq!(sector.ceil_height, 128);
    }

    #[test]
    fn mock_solid_segs_have_no_back_sector() {
        let map = MockBspMap::single_room();
        for seg in map.subsector_segs(0) {
            assert!(map.seg_back_sector(seg).is_none());
        }
    }
```

**Step 2: Run tests to verify they pass**

Run: `cargo test -p abrash-raycast bsp::tests --lib`
Expected: All 8 BSP tests PASS (4 type tests + 4 trait tests).

**Step 3: Commit**

```bash
git add crates/abrash-raycast/src/bsp.rs
git commit -m "feat(raycast): add BspMap trait with visitor-based traversal"
```

---

## Task 3: Seg Projection Math

**Files:**
- Create: `crates/abrash-raycast/src/bsp_clip.rs`
- Modify: `crates/abrash-raycast/src/lib.rs` (add `pub mod bsp_clip;`)

This module handles projecting BSP segs from world space to screen columns.

Three key functions:
1. `point_to_angle` — angle from camera to a world point (BAM)
2. `clip_seg_angles` — clip a seg's angular span to the FOV
3. `angle_to_column` — map a view-relative angle to a screen column

Plus a helper:
4. `projection_distance` — the constant relating distance to screen pixels

**Step 1: Write failing tests**

Create `crates/abrash-raycast/src/bsp_clip.rs`:

```rust
//! BSP seg-to-screen projection and frustum clipping.
//!
//! Projects BSP seg endpoints from world space to screen columns via angle
//! computation. Handles FOV clipping and the Doom-compatible
//! R_ScaleFromGlobalAngle projection formula.

use abrash_core::bam::{Bam, ANG90, ANG180, ANG270};
use abrash_core::fixed16_16::Fixed16_16;
use crate::types::Vec2Fixed;

/// Compute the angle from `origin` to `target` as a BAM angle.
///
/// Returns the absolute world-space angle (0 = east, ANG90 = north).
pub fn point_to_angle(origin: Vec2Fixed, target: Vec2Fixed) -> Bam {
    todo!()
}

/// Compute the projection distance: how many pixels correspond to 1 map unit
/// at distance 1.0. This is `(screen_width / 2) / tan(fov / 2)`.
pub fn projection_distance(screen_width: u32, fov: Bam) -> Fixed16_16 {
    todo!()
}

/// Convert a view-relative angle to a screen column index.
///
/// `view_angle` is relative to the camera's facing direction:
/// - 0 = dead center
/// - Positive = left of center
/// - Negative (large BAM) = right of center
///
/// Returns the column index (may be outside screen bounds).
pub fn angle_to_column(view_angle: Bam, projection: Fixed16_16, screen_width: u32) -> i32 {
    todo!()
}

/// Clip a seg's angular extent to the field of view.
///
/// Takes the world-space angles to the seg's two endpoints and the camera's
/// facing angle + half-FOV. Returns `None` if the seg is entirely outside
/// the FOV or is back-facing. Returns `Some((clipped_left, clipped_right))`
/// as view-relative angles.
///
/// Convention: `angle1` is the angle to v1, `angle2` to v2. In a correctly
/// wound front-facing seg, `angle1 - angle2` (in BAM) < ANG180.
pub fn clip_seg_angles(
    angle1: Bam,
    angle2: Bam,
    camera_angle: Bam,
    half_fov: Bam,
) -> Option<(Bam, Bam)> {
    todo!()
}

/// Compute the perpendicular distance from camera to a seg's infinite line.
///
/// Used for wall scale computation. The seg defines an infinite line through
/// v1 and v2; this returns the shortest distance from `camera_pos` to that line.
pub fn seg_perpendicular_distance(
    seg_v1: Vec2Fixed,
    seg_angle: Bam,
    camera_pos: Vec2Fixed,
) -> Fixed16_16 {
    todo!()
}

/// Compute wall scale (pixels per map unit) at a given screen column.
///
/// This is the Doom R_ScaleFromGlobalAngle formula:
/// `scale = projection * sin(angleb) / (distance * sin(anglea))`
/// where `anglea = ANG90 + (vis_angle - camera_angle)`
/// and `angleb = ANG90 + (vis_angle - seg_normal_angle)`.
///
/// Clamped to [0.0039, 64.0] in fixed-point to prevent overflow.
pub fn wall_scale_at_column(
    col: i32,
    screen_width: u32,
    projection: Fixed16_16,
    camera_angle: Bam,
    seg_normal_angle: Bam,
    seg_distance: Fixed16_16,
) -> Fixed16_16 {
    todo!()
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- point_to_angle ---

    #[test]
    fn point_to_angle_east() {
        let origin = Vec2Fixed::from_ints(0, 0);
        let target = Vec2Fixed::from_ints(100, 0);
        let angle = point_to_angle(origin, target);
        // East = 0 degrees = BAM 0
        assert_eq!(angle.raw(), 0, "due east should be angle 0");
    }

    #[test]
    fn point_to_angle_north() {
        let origin = Vec2Fixed::from_ints(0, 0);
        let target = Vec2Fixed::from_ints(0, 100);
        let angle = point_to_angle(origin, target);
        // North = 90 degrees = ANG90
        let diff = angle.raw().wrapping_sub(ANG90.raw());
        assert!(
            diff < 0x0100_0000 || diff > 0xFF00_0000,
            "due north should be ~ANG90, got {:#010X}",
            angle.raw()
        );
    }

    #[test]
    fn point_to_angle_west() {
        let origin = Vec2Fixed::from_ints(0, 0);
        let target = Vec2Fixed::from_ints(-100, 0);
        let angle = point_to_angle(origin, target);
        let diff = angle.raw().wrapping_sub(ANG180.raw());
        assert!(
            diff < 0x0100_0000 || diff > 0xFF00_0000,
            "due west should be ~ANG180, got {:#010X}",
            angle.raw()
        );
    }

    // --- projection_distance ---

    #[test]
    fn projection_distance_90_fov_320_wide() {
        // For 90° FOV at 320px: projection = 160 / tan(45°) = 160 / 1.0 = 160
        let proj = projection_distance(320, ANG90);
        let proj_f = proj.to_f32();
        assert!(
            (proj_f - 160.0).abs() < 1.0,
            "90° FOV at 320w should give projection ≈ 160, got {proj_f}"
        );
    }

    #[test]
    fn projection_distance_wider_fov_gives_smaller_value() {
        let proj_90 = projection_distance(320, ANG90);
        // 120° FOV: projection = 160 / tan(60°) ≈ 160 / 1.732 ≈ 92
        let fov_120 = Bam::from_raw(ANG90.raw() + ANG90.raw() / 3); // ~120°
        let proj_120 = projection_distance(320, fov_120);
        assert!(
            proj_120.raw() < proj_90.raw(),
            "wider FOV should give smaller projection"
        );
    }

    // --- angle_to_column ---

    #[test]
    fn angle_to_column_center() {
        let proj = Fixed16_16::from_int(160);
        // View angle 0 (dead center) → column 160 (center of 320px screen)
        let col = angle_to_column(Bam::ZERO, proj, 320);
        assert_eq!(col, 160, "center angle should map to center column");
    }

    #[test]
    fn angle_to_column_left_edge() {
        let proj = Fixed16_16::from_int(160);
        // At 90° FOV, the leftmost column is at +45° view angle.
        // tan(45°) = 1.0, so column = 160 - 1.0 * 160 = 0.
        // But +45° in BAM is ANG45 = 0x2000_0000 / 2 ... wait
        // Actually let me use a specific angle:
        // column = half_w - tan(angle) * projection
        // For column=0: 0 = 160 - tan(a)*160 → tan(a)=1 → a=45°
        use abrash_core::bam::ANG45;
        let col = angle_to_column(ANG45, proj, 320);
        assert!(
            (col - 0).abs() <= 1,
            "45° left should be near column 0, got {col}"
        );
    }

    // --- clip_seg_angles ---

    #[test]
    fn clip_seg_fully_visible() {
        // Camera at origin facing east (angle=0). Seg spans from NE to SE
        // (both within 90° FOV).
        let angle1 = Bam::from_raw(0x1000_0000); // ~22.5° left
        let angle2 = Bam::from_raw(0xF000_0000_u32); // ~22.5° right (wraps)
        let result = clip_seg_angles(angle1, angle2, Bam::ZERO, Bam::from_raw(ANG90.raw() / 2));
        assert!(result.is_some(), "seg within FOV should be visible");
    }

    #[test]
    fn clip_seg_behind_camera() {
        // Seg entirely behind the camera (angle span > 180°).
        let angle1 = Bam::from_raw(0xC000_0000_u32); // -90° (behind-left)
        let angle2 = Bam::from_raw(0x4000_0000); // +90° (behind-right via long way)
        // span = angle1 - angle2 = 0xC000_0000 - 0x4000_0000 = 0x8000_0000 = 180°
        // This is back-facing (span >= 180°).
        let result = clip_seg_angles(angle1, angle2, Bam::ZERO, Bam::from_raw(ANG90.raw() / 2));
        assert!(result.is_none(), "back-facing seg should be clipped away");
    }

    #[test]
    fn clip_seg_partially_visible_left() {
        // Seg extends beyond left FOV edge but is partially visible.
        // Camera facing east, 90° FOV → half_fov = 45°
        // angle1 = 60° (beyond left edge), angle2 = 10° (visible)
        let angle1 = Bam::from_raw(0x2AAA_AAAA); // ~60°
        let angle2 = Bam::from_raw(0x0B60_0000); // ~10° (approximate)
        let half_fov = Bam::from_raw(ANG90.raw() / 2); // 45°
        let result = clip_seg_angles(angle1, angle2, Bam::ZERO, half_fov);
        assert!(result.is_some(), "partially visible seg should not be fully clipped");
    }

    // --- seg_perpendicular_distance ---

    #[test]
    fn seg_perp_distance_facing_directly() {
        // Camera at (256, 128), seg at y=0 running east (angle=0).
        // Perpendicular distance to the line y=0 is 128.
        let dist = seg_perpendicular_distance(
            Vec2Fixed::from_ints(0, 0),
            Bam::ZERO, // seg runs east
            Vec2Fixed::from_ints(256, 128),
        );
        let dist_f = dist.to_f32();
        assert!(
            (dist_f - 128.0).abs() < 1.0,
            "perpendicular distance should be ≈128, got {dist_f}"
        );
    }

    // --- wall_scale_at_column ---

    #[test]
    fn wall_scale_center_column() {
        // Camera facing east, seg is east wall at distance 128.
        // At center column, scale should be projection / distance.
        let projection = Fixed16_16::from_int(160);
        let scale = wall_scale_at_column(
            160,          // center column
            320,          // screen width
            projection,
            Bam::ZERO,    // camera facing east
            ANG90,        // seg normal faces west (toward camera)
            Fixed16_16::from_int(128), // distance
        );
        let scale_f = scale.to_f32();
        // Expected: 160 / 128 = 1.25
        assert!(
            (scale_f - 1.25).abs() < 0.1,
            "center column scale should be ≈1.25, got {scale_f}"
        );
    }

    #[test]
    fn wall_scale_edge_column_smaller() {
        // Scale at screen edges should be smaller than at center
        // (walls are farther at oblique angles).
        let projection = Fixed16_16::from_int(160);
        let center_scale = wall_scale_at_column(
            160, 320, projection, Bam::ZERO, ANG90, Fixed16_16::from_int(128),
        );
        let edge_scale = wall_scale_at_column(
            40, 320, projection, Bam::ZERO, ANG90, Fixed16_16::from_int(128),
        );
        assert!(
            edge_scale.raw() <= center_scale.raw(),
            "edge scale ({}) should be <= center scale ({})",
            edge_scale.to_f32(),
            center_scale.to_f32()
        );
    }
}
```

**Step 2: Run tests to verify they fail**

Run: `cargo test -p abrash-raycast bsp_clip::tests --lib`
Expected: FAIL — all functions are `todo!()`.

**Step 3: Implement projection functions**

Key implementation notes:

- `point_to_angle`: Use `atan2` on f32 deltas, convert to BAM via `Bam::from_radians()`.
- `projection_distance`: `(screen_width/2) / tan(fov/2)`, convert to Fixed16_16.
- `angle_to_column`: `half_width - tan(view_angle) * projection`. Handle BAM wrapping for angles > 180° (treat as negative).
- `clip_seg_angles`: Convert to view-relative, check span < 180° (front-facing), clamp to `[-half_fov, +half_fov]`.
- `seg_perpendicular_distance`: Dot product of (camera - seg.v1) with the seg's normal vector. Normal is perpendicular to the seg direction: if seg angle is `a`, normal angle is `a + ANG90`.
- `wall_scale_at_column`: Doom's R_ScaleFromGlobalAngle — `projection * sin(angleb) / (distance * sin(anglea))`, clamped to `[256, 64*FRACUNIT]` (0.0039 to 64.0).

**Step 4: Run tests to verify pass**

Run: `cargo test -p abrash-raycast bsp_clip::tests --lib`
Expected: All projection tests PASS.

**Step 5: Add module to lib.rs and commit**

Add `pub mod bsp_clip;` to `crates/abrash-raycast/src/lib.rs`.

```bash
git add crates/abrash-raycast/src/bsp_clip.rs crates/abrash-raycast/src/lib.rs
git commit -m "feat(raycast): add BSP seg projection math — angle_to_column, clip, wall_scale"
```

---

## Task 4: Column Clip State

**Files:**
- Modify: `crates/abrash-raycast/src/bsp_clip.rs` (add ColumnClip below the projection functions)

The column clip tracks which screen rows are still "open" per column. As walls are drawn front-to-back, they claim rows from the top (ceiling) and bottom (floor), narrowing the open range. When top meets bottom, the column is fully filled.

**Step 1: Write failing tests**

Add to `crates/abrash-raycast/src/bsp_clip.rs`:

```rust
/// Per-column clipping state for BSP rendering.
///
/// Tracks the open (undrawn) vertical range for each screen column.
/// Walls drawn front-to-back progressively narrow the open range by
/// advancing `open_top` downward and `open_bot` upward.
pub struct ColumnClip {
    /// Topmost open row per column (inclusive). Starts at 0.
    open_top: Vec<i32>,
    /// Bottommost open row per column (inclusive). Starts at height - 1.
    open_bot: Vec<i32>,
    /// Screen width.
    width: u32,
    /// Screen height.
    height: u32,
}

impl ColumnClip {
    /// Create clip state for a screen of the given dimensions.
    /// All columns start fully open (row 0 to height-1).
    pub fn new(width: u32, height: u32) -> Self {
        todo!()
    }

    /// Is this column still open (has undrawn rows)?
    pub fn is_open(&self, col: i32) -> bool {
        todo!()
    }

    /// Get the topmost open row for a column.
    pub fn top(&self, col: i32) -> i32 {
        todo!()
    }

    /// Get the bottommost open row for a column.
    pub fn bot(&self, col: i32) -> i32 {
        todo!()
    }

    /// Advance the top clip downward (ceiling was drawn to this row).
    pub fn set_top(&mut self, col: i32, row: i32) {
        todo!()
    }

    /// Advance the bottom clip upward (floor was drawn to this row).
    pub fn set_bot(&mut self, col: i32, row: i32) {
        todo!()
    }

    /// Mark a column as fully solid (middle wall fills the entire open range).
    pub fn mark_solid(&mut self, col: i32) {
        todo!()
    }

    /// Are all columns fully filled? (Early exit optimization.)
    pub fn all_filled(&self) -> bool {
        todo!()
    }

    /// Reset all columns to fully open.
    pub fn reset(&mut self) {
        todo!()
    }
}
```

Tests:

```rust
    // --- ColumnClip tests ---

    #[test]
    fn column_clip_initial_state() {
        let clip = ColumnClip::new(320, 200);
        assert!(clip.is_open(0));
        assert!(clip.is_open(159));
        assert!(clip.is_open(319));
        assert_eq!(clip.top(0), 0);
        assert_eq!(clip.bot(0), 199);
        assert!(!clip.all_filled());
    }

    #[test]
    fn column_clip_set_top() {
        let mut clip = ColumnClip::new(320, 200);
        clip.set_top(10, 50);
        assert_eq!(clip.top(10), 50);
        assert_eq!(clip.bot(10), 199);
        assert!(clip.is_open(10));
    }

    #[test]
    fn column_clip_set_bot() {
        let mut clip = ColumnClip::new(320, 200);
        clip.set_bot(10, 150);
        assert_eq!(clip.top(10), 0);
        assert_eq!(clip.bot(10), 150);
        assert!(clip.is_open(10));
    }

    #[test]
    fn column_clip_mark_solid() {
        let mut clip = ColumnClip::new(320, 200);
        clip.mark_solid(10);
        assert!(!clip.is_open(10));
    }

    #[test]
    fn column_clip_all_filled() {
        let mut clip = ColumnClip::new(4, 200);
        assert!(!clip.all_filled());
        for col in 0..4 {
            clip.mark_solid(col);
        }
        assert!(clip.all_filled());
    }

    #[test]
    fn column_clip_reset() {
        let mut clip = ColumnClip::new(320, 200);
        clip.mark_solid(0);
        clip.set_top(1, 100);
        clip.reset();
        assert!(clip.is_open(0));
        assert_eq!(clip.top(1), 0);
    }

    #[test]
    fn column_clip_out_of_bounds_not_open() {
        let clip = ColumnClip::new(320, 200);
        assert!(!clip.is_open(-1));
        assert!(!clip.is_open(320));
    }
```

**Step 2: Run tests to verify they fail**

Run: `cargo test -p abrash-raycast bsp_clip::tests::column_clip --lib`
Expected: FAIL — all methods are `todo!()`.

**Step 3: Implement ColumnClip**

Straightforward: `Vec<i32>` for open_top/open_bot, bounds-checked access, `all_filled` checks `open_top[col] > open_bot[col]` for all columns. For efficiency, track a `filled_count` counter.

**Step 4: Run tests to verify pass**

Run: `cargo test -p abrash-raycast bsp_clip::tests --lib`
Expected: All clip tests PASS.

**Step 5: Commit**

```bash
git add crates/abrash-raycast/src/bsp_clip.rs
git commit -m "feat(raycast): add ColumnClip for BSP front-to-back rendering"
```

---

## Task 5: Visplane Allocator

**Files:**
- Create: `crates/abrash-raycast/src/bsp_visplane.rs`
- Modify: `crates/abrash-raycast/src/lib.rs` (add `pub mod bsp_visplane;`)

Visplanes collect horizontal floor/ceiling strips during the column-by-column wall pass. After all walls are drawn, the visplanes are iterated to draw floor/ceiling spans.

A visplane groups columns that share the same (height, texture, light_level). Each column records its top and bottom screen rows, forming spans to draw.

**Step 1: Write failing tests**

Create `crates/abrash-raycast/src/bsp_visplane.rs`:

```rust
//! Visplane allocator for BSP floor/ceiling rendering.
//!
//! During the wall-drawing pass, floor and ceiling strips are emitted into
//! visplanes grouped by (height, texture, light_level). After all walls are
//! processed, each visplane's spans are drawn as horizontal texture-mapped runs.

/// A single visplane: a horizontal surface at one height/texture/light.
#[derive(Clone, Debug)]
pub struct Visplane {
    /// Floor or ceiling height in map units.
    pub height: i16,
    /// Flat texture ID.
    pub texture: u16,
    /// Light level for shading.
    pub light_level: u16,
    /// Leftmost column with data.
    pub min_x: i32,
    /// Rightmost column with data.
    pub max_x: i32,
    /// Top screen row per column. `i32::MAX` = unused.
    top: Vec<i32>,
    /// Bottom screen row per column. `i32::MIN` = unused.
    bottom: Vec<i32>,
}

impl Visplane {
    /// Get the top row for a column, or None if unused.
    pub fn top(&self, col: i32) -> Option<i32> {
        todo!()
    }

    /// Get the bottom row for a column, or None if unused.
    pub fn bottom(&self, col: i32) -> Option<i32> {
        todo!()
    }
}

/// Allocates and manages visplanes during BSP rendering.
pub struct VisplaneAllocator {
    planes: Vec<Visplane>,
    screen_width: u32,
}

impl VisplaneAllocator {
    /// Create a new allocator for the given screen width.
    pub fn new(screen_width: u32) -> Self {
        todo!()
    }

    /// Find an existing visplane matching (height, texture, light) or create one.
    ///
    /// If a matching visplane exists but the requested column already has data,
    /// a new visplane is created (Doom's "visplane overflow" split).
    pub fn find_or_create(&mut self, height: i16, texture: u16, light: u16, col: i32) -> usize {
        todo!()
    }

    /// Set the screen row range for a column in a visplane.
    pub fn set_span(&mut self, plane_idx: usize, col: i32, top: i32, bottom: i32) {
        todo!()
    }

    /// Get all allocated visplanes.
    pub fn planes(&self) -> &[Visplane] {
        todo!()
    }

    /// Reset for a new frame (clears all visplanes).
    pub fn reset(&mut self) {
        todo!()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allocator_starts_empty() {
        let alloc = VisplaneAllocator::new(320);
        assert!(alloc.planes().is_empty());
    }

    #[test]
    fn find_or_create_makes_new_plane() {
        let mut alloc = VisplaneAllocator::new(320);
        let idx = alloc.find_or_create(0, 1, 160, 10);
        assert_eq!(idx, 0);
        assert_eq!(alloc.planes().len(), 1);
        assert_eq!(alloc.planes()[0].height, 0);
        assert_eq!(alloc.planes()[0].texture, 1);
    }

    #[test]
    fn find_or_create_reuses_matching_plane() {
        let mut alloc = VisplaneAllocator::new(320);
        let idx1 = alloc.find_or_create(0, 1, 160, 10);
        alloc.set_span(idx1, 10, 150, 199);
        // Same properties, different column → should reuse
        let idx2 = alloc.find_or_create(0, 1, 160, 20);
        assert_eq!(idx1, idx2);
        assert_eq!(alloc.planes().len(), 1);
    }

    #[test]
    fn find_or_create_splits_on_column_conflict() {
        let mut alloc = VisplaneAllocator::new(320);
        let idx1 = alloc.find_or_create(0, 1, 160, 10);
        alloc.set_span(idx1, 10, 150, 199);
        // Same properties, SAME column → must split into new visplane
        let idx2 = alloc.find_or_create(0, 1, 160, 10);
        assert_ne!(idx1, idx2);
        assert_eq!(alloc.planes().len(), 2);
    }

    #[test]
    fn find_or_create_different_properties() {
        let mut alloc = VisplaneAllocator::new(320);
        let idx1 = alloc.find_or_create(0, 1, 160, 10);
        let idx2 = alloc.find_or_create(128, 2, 200, 10); // different height/texture
        assert_ne!(idx1, idx2);
        assert_eq!(alloc.planes().len(), 2);
    }

    #[test]
    fn set_span_updates_bounds() {
        let mut alloc = VisplaneAllocator::new(320);
        let idx = alloc.find_or_create(0, 1, 160, 50);
        alloc.set_span(idx, 50, 100, 180);
        alloc.set_span(idx, 60, 95, 185);
        assert_eq!(alloc.planes()[idx].min_x, 50);
        assert_eq!(alloc.planes()[idx].max_x, 60);
        assert_eq!(alloc.planes()[idx].top(50), Some(100));
        assert_eq!(alloc.planes()[idx].bottom(50), Some(180));
    }

    #[test]
    fn visplane_unused_columns_return_none() {
        let mut alloc = VisplaneAllocator::new(320);
        let idx = alloc.find_or_create(0, 1, 160, 10);
        alloc.set_span(idx, 10, 100, 199);
        assert!(alloc.planes()[idx].top(5).is_none());
        assert!(alloc.planes()[idx].top(10).is_some());
    }

    #[test]
    fn reset_clears_all() {
        let mut alloc = VisplaneAllocator::new(320);
        alloc.find_or_create(0, 1, 160, 10);
        alloc.find_or_create(128, 2, 200, 20);
        alloc.reset();
        assert!(alloc.planes().is_empty());
    }
}
```

**Step 2: Run tests to verify they fail**

Run: `cargo test -p abrash-raycast bsp_visplane::tests --lib`
Expected: FAIL — all methods are `todo!()`.

**Step 3: Implement VisplaneAllocator**

Key details:
- `top` and `bottom` vecs are `screen_width` long, initialized to sentinel values.
- `find_or_create` does a linear scan (Doom does the same — typically < 128 visplanes).
- Column conflict check: if `top[col] != sentinel`, the column is already used → split.
- `min_x`/`max_x` updated on each `set_span` call.

**Step 4: Run tests to verify pass**

Run: `cargo test -p abrash-raycast bsp_visplane::tests --lib`
Expected: All 8 visplane tests PASS.

**Step 5: Add module and commit**

Add `pub mod bsp_visplane;` to `crates/abrash-raycast/src/lib.rs`.

```bash
git add crates/abrash-raycast/src/bsp_visplane.rs crates/abrash-raycast/src/lib.rs
git commit -m "feat(raycast): add visplane allocator for BSP floor/ceiling rendering"
```

---

## Task 6: BspTextures Trait + Colormap Lighting

**Files:**
- Create: `crates/abrash-render/src/raycaster/bsp_lighting.rs`
- Modify: `crates/abrash-render/src/raycaster/mod.rs` (add modules)

**Step 1: Write failing tests**

Create `crates/abrash-render/src/raycaster/bsp_lighting.rs`:

```rust
//! Colormap-based distance shading for BSP rendering.
//!
//! Doom uses 32 colormaps (rows in COLORMAP lump) to shade surfaces by
//! distance. Index 0 = full bright, index 31 = nearly black. The colormap
//! index is computed from perpendicular distance and base sector light level.

use abrash_core::fixed16_16::Fixed16_16;

/// Texture data provider for BSP rendering.
///
/// Consumers implement this to bridge their WAD/PK3 texture caches into
/// the BSP renderer. Texture IDs are opaque u16 handles — abrash doesn't
/// know about WAD lumps.
pub trait BspTextures {
    /// Get a single column of wall texture data (palette-indexed, top to bottom).
    ///
    /// Returns a slice of palette indices, one per texel row. The renderer
    /// tiles vertically if the column is taller than the texture.
    fn wall_column(&self, texture_id: u16, col: usize) -> &[u8];

    /// Get a 64×64 flat texture (4096 bytes, palette-indexed, row-major).
    fn flat_data(&self, texture_id: u16) -> &[u8; 4096];

    /// Get a 256-byte colormap row for light-level shading.
    ///
    /// `index` 0 = full bright, 31 = darkest. Each entry maps a palette
    /// index to a shaded palette index.
    fn colormap(&self, index: u8) -> &[u8; 256];

    /// Look up a palette entry (palette index → ARGB).
    fn palette_argb(&self, palette_idx: u8) -> u32;
}

/// Maximum colormap index (darkest).
pub const MAX_COLORMAP: u8 = 31;

/// Compute the colormap index for distance-based shading.
///
/// Doom's formula: `index = (MAXLIGHTSCALE - scale) + (LIGHTLEVELS - 1 - (light >> LIGHTSEGSHIFT))`
/// Simplified: farther distance → higher index → darker.
///
/// Returns 0 (brightest) to 31 (darkest).
pub fn colormap_index(distance: Fixed16_16, light_level: u16) -> u8 {
    todo!()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn colormap_close_bright_sector_is_zero() {
        // Very close to camera + bright sector → should be full bright (0).
        let idx = colormap_index(Fixed16_16::from_int(1), 255);
        assert_eq!(idx, 0, "close + bright should be colormap 0");
    }

    #[test]
    fn colormap_far_dark_sector_is_max() {
        // Very far + dark sector → should be darkest.
        let idx = colormap_index(Fixed16_16::from_int(2048), 0);
        assert_eq!(idx, MAX_COLORMAP, "far + dark should be max colormap");
    }

    #[test]
    fn colormap_increases_with_distance() {
        let near = colormap_index(Fixed16_16::from_int(32), 160);
        let far = colormap_index(Fixed16_16::from_int(512), 160);
        assert!(
            far >= near,
            "farther distance should give >= colormap index: near={near}, far={far}"
        );
    }

    #[test]
    fn colormap_decreases_with_light() {
        let dark = colormap_index(Fixed16_16::from_int(128), 64);
        let bright = colormap_index(Fixed16_16::from_int(128), 224);
        assert!(
            bright <= dark,
            "brighter sector should give <= colormap index: bright={bright}, dark={dark}"
        );
    }

    #[test]
    fn colormap_clamped_to_range() {
        // Edge cases should never exceed [0, 31].
        let min = colormap_index(Fixed16_16::from_int(0), 255);
        let max = colormap_index(Fixed16_16::from_int(10000), 0);
        assert!(min <= MAX_COLORMAP);
        assert!(max <= MAX_COLORMAP);
    }
}
```

**Step 2: Run tests to verify they fail**

Run: `cargo test -p abrash-render bsp_lighting::tests --lib`
Expected: FAIL — `colormap_index` is `todo!()`.

**Step 3: Implement colormap_index**

Doom's shading model: `base_light = light_level >> 4` (0-15 light scale). `distance_fade = clamp(distance / FADE_DISTANCE, 0, MAX_COLORMAP)`. `index = clamp(distance_fade - base_light, 0, 31)`.

Add `pub mod bsp_lighting;` to `crates/abrash-render/src/raycaster/mod.rs`.

**Step 4: Run tests to verify pass**

Run: `cargo test -p abrash-render bsp_lighting::tests --lib`
Expected: All 5 lighting tests PASS.

**Step 5: Commit**

```bash
git add crates/abrash-render/src/raycaster/bsp_lighting.rs crates/abrash-render/src/raycaster/mod.rs
git commit -m "feat(render): add BspTextures trait and colormap distance shading"
```

---

## Task 7: Wall Column Renderer

**Files:**
- Create: `crates/abrash-render/src/raycaster/bsp.rs`
- Modify: `crates/abrash-render/src/raycaster/mod.rs` (add `pub mod bsp;`)

This draws a single vertical strip of textured wall. It's called per-column for each seg during the BSP traversal.

**Step 1: Write failing tests**

Create `crates/abrash-render/src/raycaster/bsp.rs`:

```rust
//! BSP column and span renderer.
//!
//! Draws Doom-style textured walls (columns) and floors/ceilings (spans)
//! using data from `BspMap` (geometry) and `BspTextures` (art/lighting).

use abrash_core::fixed16_16::Fixed16_16;
use abrash_core::framebuffer::Framebuffer;
use abrash_core::zbuffer::ZBuffer;
use super::bsp_lighting::BspTextures;

/// Draw a single textured wall column.
///
/// Renders a vertical strip of wall texture from `y_top` to `y_bot` (inclusive),
/// applying colormap shading. The texture tiles vertically.
///
/// # Parameters
/// - `fb` / `zbuf` — destination buffers
/// - `textures` — texture data provider
/// - `col` — screen column (x coordinate)
/// - `y_top` / `y_bot` — vertical screen range (inclusive, pre-clipped)
/// - `texture_id` — wall texture handle
/// - `texture_col` — which column of the texture to sample
/// - `colormap_idx` — shading level (0=bright, 31=dark)
/// - `tex_y_frac` — starting texture Y in 16.16 fixed-point
/// - `tex_y_step` — texture Y increment per screen row in 16.16 fixed-point
/// - `distance` — wall distance for z-buffer
pub fn draw_wall_column(
    fb: &mut Framebuffer,
    zbuf: &mut ZBuffer,
    textures: &impl BspTextures,
    col: i32,
    y_top: i32,
    y_bot: i32,
    texture_id: u16,
    texture_col: usize,
    colormap_idx: u8,
    tex_y_frac: Fixed16_16,
    tex_y_step: Fixed16_16,
    distance: f32,
) {
    todo!()
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::bsp_lighting::MAX_COLORMAP;

    /// Mock textures for testing: solid-color walls, flat floors, identity colormap.
    struct MockTextures {
        wall_column_data: Vec<u8>,
        flat: [u8; 4096],
        colormap_data: [[u8; 256]; 32],
        palette: [u32; 256],
    }

    impl MockTextures {
        fn new() -> Self {
            let mut palette = [0xFF000000_u32; 256];
            palette[1] = 0xFFFF0000; // red
            palette[2] = 0xFF00FF00; // green

            // Identity colormap for index 0 (full bright)
            let mut colormap_data = [[0u8; 256]; 32];
            for (i, entry) in colormap_data[0].iter_mut().enumerate() {
                *entry = i as u8;
            }
            // Dark colormap: everything maps to index 0 (black)
            // (indices 1..31 all map to 0)

            Self {
                wall_column_data: vec![1; 128], // 128-pixel tall, all palette index 1
                flat: [2; 4096], // all palette index 2
                colormap_data,
                palette,
            }
        }
    }

    impl BspTextures for MockTextures {
        fn wall_column(&self, _texture_id: u16, _col: usize) -> &[u8] {
            &self.wall_column_data
        }
        fn flat_data(&self, _texture_id: u16) -> &[u8; 4096] {
            &self.flat
        }
        fn colormap(&self, index: u8) -> &[u8; 256] {
            &self.colormap_data[index as usize]
        }
        fn palette_argb(&self, palette_idx: u8) -> u32 {
            self.palette[palette_idx as usize]
        }
    }

    #[test]
    fn draw_wall_column_fills_pixels() {
        let mut fb = Framebuffer::new(320, 200).unwrap();
        let mut zbuf = ZBuffer::new(320, 200).unwrap();
        let textures = MockTextures::new();
        fb.clear(0xFF000000);

        draw_wall_column(
            &mut fb,
            &mut zbuf,
            &textures,
            160,                    // center column
            80,                     // y_top
            120,                    // y_bot
            1,                      // texture_id
            0,                      // texture_col
            0,                      // colormap 0 = full bright
            Fixed16_16::ZERO,       // tex_y start
            Fixed16_16::from_int(1),// tex_y step (1:1 mapping)
            100.0,                  // distance
        );

        // Pixels in range should be drawn (not black).
        let pixel = fb.get_pixel(160, 100).unwrap();
        assert_ne!(pixel, 0xFF000000, "wall column should paint pixels");

        // Pixels outside range should still be black.
        let above = fb.get_pixel(160, 79).unwrap();
        assert_eq!(above, 0xFF000000, "pixels above wall should be untouched");
    }

    #[test]
    fn draw_wall_column_writes_zbuffer() {
        let mut fb = Framebuffer::new(320, 200).unwrap();
        let mut zbuf = ZBuffer::new(320, 200).unwrap();
        let textures = MockTextures::new();

        draw_wall_column(
            &mut fb, &mut zbuf, &textures,
            160, 80, 120, 1, 0, 0,
            Fixed16_16::ZERO, Fixed16_16::from_int(1), 42.0,
        );

        let depth = zbuf.get_depth(160, 100).unwrap();
        assert!(
            (depth - 42.0).abs() < 0.01,
            "z-buffer should have wall distance, got {depth}"
        );
    }

    #[test]
    fn draw_wall_column_empty_range_no_op() {
        let mut fb = Framebuffer::new(320, 200).unwrap();
        let mut zbuf = ZBuffer::new(320, 200).unwrap();
        let textures = MockTextures::new();
        fb.clear(0xFF000000);

        // y_top > y_bot → nothing to draw.
        draw_wall_column(
            &mut fb, &mut zbuf, &textures,
            160, 120, 80, 1, 0, 0,
            Fixed16_16::ZERO, Fixed16_16::from_int(1), 100.0,
        );

        let pixel = fb.get_pixel(160, 100).unwrap();
        assert_eq!(pixel, 0xFF000000, "empty range should not draw");
    }
}
```

**Step 2: Run tests to verify they fail**

Run: `cargo test -p abrash-render raycaster::bsp::tests --lib`
Expected: FAIL — `draw_wall_column` is `todo!()`.

**Step 3: Implement draw_wall_column**

The column renderer:
1. Early return if `y_top > y_bot` or column out of screen bounds.
2. Get the texture column data: `textures.wall_column(texture_id, texture_col)`.
3. Get the colormap: `textures.colormap(colormap_idx)`.
4. For each row from `y_top` to `y_bot`:
   - Compute texture Y: `tex_y = (tex_y_frac + tex_y_step * (row - y_top)) >> 16`
   - Wrap to texture height: `tex_y % tex_height`
   - Look up palette index from texture column
   - Shade via colormap: `shaded_idx = colormap[palette_idx]`
   - Convert to ARGB: `textures.palette_argb(shaded_idx)`
   - Write pixel and z-buffer depth.

Add `pub mod bsp;` to `crates/abrash-render/src/raycaster/mod.rs`.

**Step 4: Run tests to verify pass**

Run: `cargo test -p abrash-render raycaster::bsp::tests --lib`
Expected: All 3 wall column tests PASS.

**Step 5: Commit**

```bash
git add crates/abrash-render/src/raycaster/bsp.rs crates/abrash-render/src/raycaster/mod.rs
git commit -m "feat(render): add BSP wall column renderer with texture + colormap shading"
```

---

## Task 8: Floor/Ceiling Span Renderer

**Files:**
- Modify: `crates/abrash-render/src/raycaster/bsp.rs`

Draw horizontal texture-mapped spans from visplane data. Each visplane has per-column top/bottom rows; we iterate rows and draw horizontal runs.

**Step 1: Write failing tests**

Add to `crates/abrash-render/src/raycaster/bsp.rs`:

```rust
/// Draw all visplane spans (floors and ceilings).
///
/// For each visplane, iterates columns left-to-right. For each column,
/// draws horizontal spans from the stored top/bottom rows using
/// perspective-correct flat texture mapping.
///
/// The flat texture is 64×64 and tiles infinitely. Screen-space coordinates
/// are reverse-projected to world-space floor/ceiling coordinates via:
/// `world_x = camera_x + (col - half_w) * step_distance / projection`
/// `world_y = camera_y + step_distance`
/// where `step_distance = height_diff * projection / (row - half_h)`.
pub fn draw_visplane_spans(
    fb: &mut Framebuffer,
    zbuf: &mut ZBuffer,
    textures: &impl BspTextures,
    visplanes: &abrash_raycast::bsp_visplane::VisplaneAllocator,
    camera_pos: Vec2Fixed,
    camera_angle: Bam,
    camera_z: Fixed16_16,
    projection: Fixed16_16,
) {
    todo!()
}
```

Tests:

```rust
    #[test]
    fn draw_visplane_spans_fills_floor() {
        use abrash_raycast::bsp_visplane::VisplaneAllocator;

        let mut fb = Framebuffer::new(320, 200).unwrap();
        let mut zbuf = ZBuffer::new(320, 200).unwrap();
        let textures = MockTextures::new();
        fb.clear(0xFF000000);

        let mut alloc = VisplaneAllocator::new(320);
        // Create a floor visplane spanning columns 100-200, rows 150-199.
        let idx = alloc.find_or_create(0, 1, 160, 100);
        for col in 100..=200 {
            if col != 100 {
                // Ensure the allocator knows about this column
                let _ = alloc.find_or_create(0, 1, 160, col);
            }
            alloc.set_span(idx, col, 150, 199);
        }

        draw_visplane_spans(
            &mut fb,
            &mut zbuf,
            &textures,
            &alloc,
            Vec2Fixed::from_ints(256, 256),
            Bam::ZERO,
            Fixed16_16::from_int(41), // eye height = 41 map units
            Fixed16_16::from_int(160),
        );

        // Pixel in the visplane region should be drawn.
        let pixel = fb.get_pixel(150, 175).unwrap();
        assert_ne!(pixel, 0xFF000000, "floor visplane should paint pixels");

        // Pixel outside should be untouched.
        let outside = fb.get_pixel(50, 175).unwrap();
        assert_eq!(outside, 0xFF000000, "area outside visplane should be untouched");
    }

    #[test]
    fn draw_visplane_spans_empty_allocator_is_noop() {
        use abrash_raycast::bsp_visplane::VisplaneAllocator;

        let mut fb = Framebuffer::new(320, 200).unwrap();
        let mut zbuf = ZBuffer::new(320, 200).unwrap();
        let textures = MockTextures::new();
        fb.clear(0xFF000000);

        let alloc = VisplaneAllocator::new(320);
        draw_visplane_spans(
            &mut fb, &mut zbuf, &textures, &alloc,
            Vec2Fixed::from_ints(256, 256), Bam::ZERO,
            Fixed16_16::from_int(41), Fixed16_16::from_int(160),
        );

        // Nothing should be drawn.
        let pixel = fb.get_pixel(160, 100).unwrap();
        assert_eq!(pixel, 0xFF000000);
    }
```

**Step 2: Run tests to verify they fail**

Run: `cargo test -p abrash-render raycaster::bsp::tests::draw_visplane --lib`
Expected: FAIL — `draw_visplane_spans` is `todo!()`.

**Step 3: Implement span renderer**

For each visplane, for each column in `[min_x, max_x]`:
1. Get `top` and `bottom` rows for this column.
2. For each row in `[top, bottom]`:
   - Compute `height_diff = |camera_z - plane_height|` (in map units).
   - Compute `step_distance = height_diff * projection / (row - half_h)`.
   - Reverse-project to world coords using camera angle.
   - Sample flat texture: `flat_data[(world_y & 63) * 64 + (world_x & 63)]`.
   - Apply colormap shading based on `step_distance`.
   - Write pixel + z-buffer.

**Step 4: Run tests to verify pass**

Run: `cargo test -p abrash-render raycaster::bsp::tests --lib`
Expected: All span tests PASS.

**Step 5: Commit**

```bash
git add crates/abrash-render/src/raycaster/bsp.rs
git commit -m "feat(render): add BSP floor/ceiling visplane span renderer"
```

---

## Task 9: render_bsp_view Entry Point + Integration

**Files:**
- Modify: `crates/abrash-render/src/raycaster/bsp.rs` (add entry point)
- Modify: `crates/abrash-raycast/src/lib.rs` (verify re-exports)
- Modify: `crates/abrash-render/src/raycaster/mod.rs` (verify re-exports)

Wire the entire BSP rendering pipeline into one entry point.

**Step 1: Write integration test**

Add to `crates/abrash-render/src/raycaster/bsp.rs`:

```rust
use abrash_core::bam::{Bam, ANG90};
use abrash_core::fixed16_16::Fixed16_16;
use abrash_core::framebuffer::Framebuffer;
use abrash_core::zbuffer::ZBuffer;
use abrash_raycast::bsp::{BspMap, BspSeg, BspSector};
use abrash_raycast::bsp_clip::ColumnClip;
use abrash_raycast::bsp_visplane::VisplaneAllocator;
use abrash_raycast::types::Vec2Fixed;
use super::bsp_lighting::{BspTextures, colormap_index};

/// Render a BSP first-person view into the framebuffer and z-buffer.
///
/// This is the main entry point for Doom-style BSP rendering. It traverses
/// the BSP tree front-to-back, drawing walls column-by-column and collecting
/// floor/ceiling visplanes, then draws the visplane spans.
///
/// # Parameters
/// - `fb` / `zbuf` — destination buffers
/// - `map` — BSP map geometry (consumer implements `BspMap`)
/// - `textures` — texture/colormap provider (consumer implements `BspTextures`)
/// - `camera_pos` — camera position in world space
/// - `camera_angle` — camera facing direction
/// - `camera_z` — camera eye height in map units
/// - `fov` — horizontal field of view
pub fn render_bsp_view(
    fb: &mut Framebuffer,
    zbuf: &mut ZBuffer,
    map: &impl BspMap,
    textures: &impl BspTextures,
    camera_pos: Vec2Fixed,
    camera_angle: Bam,
    camera_z: Fixed16_16,
    fov: Bam,
) {
    todo!()
}
```

Integration tests (in the tests module):

```rust
    // --- Integration tests for render_bsp_view ---

    // Reuse MockBspMap pattern from abrash-raycast (can't share test code
    // across crates, so we define a local copy).

    struct IntegrationBspMap {
        sector: BspSector,
        segs: Vec<BspSeg>,
    }

    impl IntegrationBspMap {
        fn single_room() -> Self {
            let sector = BspSector {
                floor_height: 0,
                ceil_height: 128,
                light_level: 160,
                floor_texture: 1,
                ceil_texture: 2,
            };
            let segs = vec![
                // South wall
                BspSeg {
                    v1: Vec2Fixed::from_ints(0, 0),
                    v2: Vec2Fixed::from_ints(512, 0),
                    offset: Fixed16_16::ZERO,
                    angle: Bam::ZERO,
                    front_sector: 0,
                    back_sector: None,
                    upper_texture: 0,
                    middle_texture: 1,
                    lower_texture: 0,
                    line_flags: 0,
                },
                // East wall
                BspSeg {
                    v1: Vec2Fixed::from_ints(512, 0),
                    v2: Vec2Fixed::from_ints(512, 512),
                    offset: Fixed16_16::ZERO,
                    angle: ANG90,
                    front_sector: 0,
                    back_sector: None,
                    upper_texture: 0,
                    middle_texture: 1,
                    lower_texture: 0,
                    line_flags: 0,
                },
                // North wall
                BspSeg {
                    v1: Vec2Fixed::from_ints(512, 512),
                    v2: Vec2Fixed::from_ints(0, 512),
                    offset: Fixed16_16::ZERO,
                    angle: ANG180,
                    front_sector: 0,
                    back_sector: None,
                    upper_texture: 0,
                    middle_texture: 1,
                    lower_texture: 0,
                    line_flags: 0,
                },
                // West wall
                BspSeg {
                    v1: Vec2Fixed::from_ints(0, 512),
                    v2: Vec2Fixed::from_ints(0, 0),
                    offset: Fixed16_16::ZERO,
                    angle: ANG270,
                    front_sector: 0,
                    back_sector: None,
                    upper_texture: 0,
                    middle_texture: 1,
                    lower_texture: 0,
                    line_flags: 0,
                },
            ];
            Self { sector, segs }
        }
    }

    impl BspMap for IntegrationBspMap {
        fn traverse_front_to_back(&self, _pos: Vec2Fixed, visitor: &mut dyn FnMut(usize)) {
            visitor(0);
        }
        fn subsector_segs(&self, _idx: usize) -> &[BspSeg] {
            &self.segs
        }
        fn seg_front_sector(&self, _seg: &BspSeg) -> &BspSector {
            &self.sector
        }
        fn seg_back_sector(&self, _seg: &BspSeg) -> Option<&BspSector> {
            None
        }
    }

    #[test]
    fn render_bsp_view_does_not_panic() {
        let map = IntegrationBspMap::single_room();
        let textures = MockTextures::new();
        let mut fb = Framebuffer::new(320, 200).unwrap();
        let mut zbuf = ZBuffer::new(320, 200).unwrap();

        render_bsp_view(
            &mut fb,
            &mut zbuf,
            &map,
            &textures,
            Vec2Fixed::from_ints(256, 256), // center of room
            Bam::ZERO,                      // facing east
            Fixed16_16::from_int(41),       // eye height
            ANG90,                          // 90° FOV
        );
    }

    #[test]
    fn render_bsp_view_draws_walls() {
        let map = IntegrationBspMap::single_room();
        let textures = MockTextures::new();
        let mut fb = Framebuffer::new(320, 200).unwrap();
        let mut zbuf = ZBuffer::new(320, 200).unwrap();
        fb.clear(0xFF000000);

        render_bsp_view(
            &mut fb, &mut zbuf, &map, &textures,
            Vec2Fixed::from_ints(256, 256),
            Bam::ZERO,
            Fixed16_16::from_int(41),
            ANG90,
        );

        // Center of screen should have a wall drawn (not black).
        let center = fb.get_pixel(160, 100).unwrap();
        assert_ne!(
            center, 0xFF000000,
            "center should have a wall rendered"
        );
    }

    #[test]
    fn render_bsp_view_writes_depth() {
        let map = IntegrationBspMap::single_room();
        let textures = MockTextures::new();
        let mut fb = Framebuffer::new(320, 200).unwrap();
        let mut zbuf = ZBuffer::new(320, 200).unwrap();

        render_bsp_view(
            &mut fb, &mut zbuf, &map, &textures,
            Vec2Fixed::from_ints(256, 256),
            Bam::ZERO,
            Fixed16_16::from_int(41),
            ANG90,
        );

        let depth = zbuf.get_depth(160, 100).unwrap();
        assert!(
            depth.is_finite() && depth > 0.0,
            "z-buffer should have positive finite depth at wall, got {depth}"
        );
    }

    #[test]
    fn render_bsp_view_zero_size_noop() {
        let map = IntegrationBspMap::single_room();
        let textures = MockTextures::new();
        let mut fb = Framebuffer::new(1, 1).unwrap();
        let mut zbuf = ZBuffer::new(1, 1).unwrap();

        // Should not panic on tiny framebuffer.
        render_bsp_view(
            &mut fb, &mut zbuf, &map, &textures,
            Vec2Fixed::from_ints(256, 256),
            Bam::ZERO, Fixed16_16::from_int(41), ANG90,
        );
    }
```

**Step 2: Run tests to verify they fail**

Run: `cargo test -p abrash-render raycaster::bsp::tests::render_bsp --lib`
Expected: FAIL — `render_bsp_view` is `todo!()`.

**Step 3: Implement render_bsp_view**

The rendering loop (see design doc for full algorithm):

```rust
pub fn render_bsp_view(...) {
    let w = fb.width();
    let h = fb.height();
    if w == 0 || h == 0 { return; }

    let half_fov = Bam::from_raw(fov.raw() / 2);
    let projection = projection_distance(w, fov);
    let mut clip = ColumnClip::new(w, h);
    let mut visplanes = VisplaneAllocator::new(w);
    let half_h = h as i32 / 2;

    map.traverse_front_to_back(camera_pos, &mut |ssector_idx| {
        if clip.all_filled() { return; }

        for (seg_idx, seg) in map.subsector_segs(ssector_idx).iter().enumerate() {
            // 1. Compute angles to endpoints
            let angle1 = point_to_angle(camera_pos, seg.v1);
            let angle2 = point_to_angle(camera_pos, seg.v2);

            // 2. Clip to FOV
            let Some((a1, a2)) = clip_seg_angles(angle1, angle2, camera_angle, half_fov)
                else { continue; };

            // 3. Map to screen columns
            let x1 = angle_to_column(a1, projection, w);
            let x2 = angle_to_column(a2, projection, w);
            if x1 > x2 { continue; }

            // 4. Per-seg constants
            let front = map.seg_front_sector(seg);
            let back = map.seg_back_sector(seg);
            let seg_normal = seg.angle.wrapping_add(ANG90);
            let rw_distance = seg_perpendicular_distance(seg.v1, seg.angle, camera_pos);

            // 5. Per-column loop
            for x in x1.max(0)..=x2.min(w as i32 - 1) {
                if !clip.is_open(x) { continue; }

                let scale = wall_scale_at_column(
                    x, w, projection, camera_angle, seg_normal, rw_distance,
                );
                let scale_f = scale.to_f32();

                // Ceiling and floor screen rows
                let ceil_row = half_h - ((front.ceil_height as i32 - camera_z.to_int()) as f32 * scale_f) as i32;
                let floor_row = half_h - ((front.floor_height as i32 - camera_z.to_int()) as f32 * scale_f) as i32;

                let open_t = clip.top(x);
                let open_b = clip.bot(x);
                let distance_f = rw_distance.to_f32().max(0.001);
                let cmap = colormap_index(rw_distance, front.light_level);

                if let Some(_back) = back {
                    // Two-sided: handle upper/lower walls and portal opening
                    // (implementation follows same pattern with back sector heights)
                    // ... compute back_ceil_row, back_floor_row ...
                    // ... draw upper/lower walls, update clip ...
                } else {
                    // One-sided: draw ceiling visplane, middle wall, floor visplane

                    // Ceiling
                    let cy_top = open_t;
                    let cy_bot = (ceil_row - 1).min(open_b);
                    if cy_top <= cy_bot {
                        let vp = visplanes.find_or_create(
                            front.ceil_height, front.ceil_texture, front.light_level, x,
                        );
                        visplanes.set_span(vp, x, cy_top, cy_bot);
                    }

                    // Middle wall
                    let wy_top = ceil_row.max(open_t);
                    let wy_bot = floor_row.min(open_b);
                    if wy_top <= wy_bot && seg.middle_texture != 0 {
                        // Compute texture column and stepping
                        let tex_col = compute_texture_column(seg, x, ...);
                        let wall_height = front.ceil_height - front.floor_height;
                        let tex_y_step = Fixed16_16::from_f32(wall_height as f32 / (floor_row - ceil_row) as f32);
                        let tex_y_start = if ceil_row < open_t {
                            tex_y_step * Fixed16_16::from_int(open_t - ceil_row)
                        } else {
                            Fixed16_16::ZERO
                        };

                        draw_wall_column(
                            fb, zbuf, textures, x, wy_top, wy_bot,
                            seg.middle_texture, tex_col, cmap,
                            tex_y_start, tex_y_step, distance_f,
                        );
                    }

                    // Floor
                    let fy_top = (floor_row + 1).max(open_t);
                    let fy_bot = open_b;
                    if fy_top <= fy_bot {
                        let vp = visplanes.find_or_create(
                            front.floor_height, front.floor_texture, front.light_level, x,
                        );
                        visplanes.set_span(vp, x, fy_top, fy_bot);
                    }

                    clip.mark_solid(x);
                }
            }
        }
    });

    // Draw floor/ceiling spans from visplanes.
    draw_visplane_spans(fb, zbuf, textures, &visplanes, camera_pos, camera_angle, camera_z, projection);
}
```

**Step 4: Run all tests**

Run: `cargo test -p abrash-raycast --lib && cargo test -p abrash-render --lib`
Expected: ALL tests PASS (abrash-raycast BSP tests + abrash-render BSP tests).

**Step 5: Commit**

```bash
git add crates/abrash-render/src/raycaster/bsp.rs
git commit -m "feat(render): add render_bsp_view entry point — complete BSP software renderer"
```

---

## Summary

| Task | Module | Key Deliverable |
|------|--------|----------------|
| 1 | abrash-raycast/bsp.rs | BspSeg, BspSector data types |
| 2 | abrash-raycast/bsp.rs | BspMap trait + MockBspMap |
| 3 | abrash-raycast/bsp_clip.rs | point_to_angle, clip_seg_angles, angle_to_column, wall_scale |
| 4 | abrash-raycast/bsp_clip.rs | ColumnClip open_top/open_bot tracking |
| 5 | abrash-raycast/bsp_visplane.rs | Visplane + VisplaneAllocator |
| 6 | abrash-render/bsp_lighting.rs | BspTextures trait + colormap_index |
| 7 | abrash-render/bsp.rs | draw_wall_column textured column renderer |
| 8 | abrash-render/bsp.rs | draw_visplane_spans floor/ceiling renderer |
| 9 | abrash-render/bsp.rs | render_bsp_view entry point + integration tests |

**New test count:** ~45 tests across both crates.

**Files created:** 4 (`bsp.rs`, `bsp_clip.rs`, `bsp_visplane.rs`, `bsp_lighting.rs`)
**Files modified:** 3 (`abrash-raycast/lib.rs`, `abrash-render/raycaster/mod.rs`, existing `mod.rs`)
