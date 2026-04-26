//! BSP tree types for Doom-style map traversal.
//!
//! Parallel to the DDA grid raycaster — both answer "what does this ray hit?"
//! but BSP iterates segs front-to-back while DDA steps through grid cells.

use abrash_core::bam::Bam;
use abrash_core::fixed16_16::Fixed16_16;

use crate::types::Vec2Fixed;

/// A wall segment in BSP space.
#[derive(Clone, Debug)]
pub struct BspSeg {
    /// Start vertex of this segment.
    pub v1: Vec2Fixed,
    /// End vertex of this segment.
    pub v2: Vec2Fixed,
    /// Texture offset along the parent linedef.
    pub offset: Fixed16_16,
    /// Angle of this segment in BAM units.
    pub angle: Bam,
    /// Index of the sector on the front side of this seg.
    pub front_sector: u16,
    /// Index of the sector on the back side, or `None` for solid (one-sided) walls.
    pub back_sector: Option<u16>,
    /// Upper texture index (used for two-sided linedefs with height difference).
    pub upper_texture: u16,
    /// Middle texture index (the main wall texture).
    pub middle_texture: u16,
    /// Lower texture index (used for two-sided linedefs with height difference).
    pub lower_texture: u16,
    /// Linedef flags (e.g. impassable, two-sided, upper-unpegged).
    pub line_flags: u16,
}

/// Sector geometry and appearance.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BspSector {
    /// Floor height in map units.
    pub floor_height: i16,
    /// Ceiling height in map units.
    pub ceil_height: i16,
    /// Ambient light level (0..255 typical).
    pub light_level: u16,
    /// Floor flat (texture) index.
    pub floor_texture: u16,
    /// Ceiling flat (texture) index.
    pub ceil_texture: u16,
}

/// A BSP-structured map that can be traversed front-to-back.
///
/// The raycaster calls [`traverse_front_to_back`](BspMapData::traverse_front_to_back)
/// to visit subsectors in painter's-algorithm order.
pub struct BspMapData {
    pub sector: BspSector,
    pub segs: Vec<BspSeg>,
}

impl BspMapData {
    /// Walk the BSP tree from `pos`, calling `visitor` with each subsector
    /// index in front-to-back order.
    pub fn traverse_front_to_back(&self, _pos: Vec2Fixed, visitor: &mut dyn FnMut(usize)) {
        // Single subsector — always visit index 0.
        visitor(0);
    }

    /// Returns the segments of the given subsector.
    ///
    /// # Panics
    ///
    /// Panics if the subsector index is out of bounds.
    #[must_use]
    pub fn subsector_segs(&self, ssector_idx: usize) -> &[BspSeg] {
        assert_eq!(ssector_idx, 0, "mock map only has subsector 0");
        &self.segs
    }

    /// Return the front sector for a seg.
    #[must_use]
    pub const fn seg_front_sector(&self, _seg: &BspSeg) -> &BspSector {
        &self.sector
    }

    /// Return the back sector for a seg, or `None` for solid walls.
    #[must_use]
    pub fn seg_back_sector(&self, seg: &BspSeg) -> Option<&BspSector> {
        seg.back_sector.map(|_| &self.sector)
    }
}

#[cfg(test)]
mod tests {
    use abrash_core::bam::{ANG90, ANG180, ANG270, Bam};
    use abrash_core::fixed16_16::Fixed16_16;

    use super::*;

    // ---- Task 1: data-type construction tests ----

    #[test]
    fn bsp_seg_solid_wall() {
        let seg = BspSeg {
            v1: Vec2Fixed::from_ints(0, 0),
            v2: Vec2Fixed::from_ints(512, 0),
            offset: Fixed16_16::ZERO,
            angle: Bam::ZERO,
            front_sector: 0,
            back_sector: None,
            upper_texture: 0,
            middle_texture: 1,
            lower_texture: 0,
            line_flags: 0x0001, // impassable
        };
        assert!(
            seg.back_sector.is_none(),
            "solid wall must have no back sector"
        );
    }

    #[test]
    fn bsp_seg_two_sided() {
        let seg = BspSeg {
            v1: Vec2Fixed::from_ints(256, 0),
            v2: Vec2Fixed::from_ints(256, 512),
            offset: Fixed16_16::ZERO,
            angle: ANG90,
            front_sector: 0,
            back_sector: Some(1),
            upper_texture: 2,
            middle_texture: 0,
            lower_texture: 3,
            line_flags: 0x0004, // two-sided
        };
        assert_eq!(seg.back_sector, Some(1));
    }

    #[test]
    fn bsp_sector_construction() {
        let sector = BspSector {
            floor_height: 0,
            ceil_height: 128,
            light_level: 160,
            floor_texture: 10,
            ceil_texture: 11,
        };
        assert_eq!(sector.floor_height, 0);
        assert_eq!(sector.ceil_height, 128);
        assert_eq!(sector.light_level, 160);
        assert_eq!(sector.floor_texture, 10);
        assert_eq!(sector.ceil_texture, 11);
    }

    #[test]
    fn bsp_sector_negative_floor() {
        let sector = BspSector {
            floor_height: -64,
            ceil_height: 64,
            light_level: 200,
            floor_texture: 0,
            ceil_texture: 0,
        };
        assert_eq!(sector.floor_height, -64);
        assert_eq!(sector.ceil_height, 64);
        assert_eq!(
            sector.ceil_height - sector.floor_height,
            128,
            "height difference must be 128"
        );
    }

    // ---- Task 2: MockBspMap + trait tests ----

    impl BspMapData {
        pub fn new_mock() -> Self {
            let sector = BspSector {
                floor_height: 0,
                ceil_height: 128,
                light_level: 160,
                floor_texture: 1,
                ceil_texture: 2,
            };

            // Four walls of a 512x512 room, wound clockwise.
            let segs = vec![
                // North wall: (0,0) -> (512,0)
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
                    line_flags: 0x0001,
                },
                // East wall: (512,0) -> (512,512)
                BspSeg {
                    v1: Vec2Fixed::from_ints(512, 0),
                    v2: Vec2Fixed::from_ints(512, 512),
                    offset: Fixed16_16::ZERO,
                    angle: ANG270,
                    front_sector: 0,
                    back_sector: None,
                    upper_texture: 0,
                    middle_texture: 1,
                    lower_texture: 0,
                    line_flags: 0x0001,
                },
                // South wall: (512,512) -> (0,512)
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
                    line_flags: 0x0001,
                },
                // West wall: (0,512) -> (0,0)
                BspSeg {
                    v1: Vec2Fixed::from_ints(0, 512),
                    v2: Vec2Fixed::from_ints(0, 0),
                    offset: Fixed16_16::ZERO,
                    angle: ANG90,
                    front_sector: 0,
                    back_sector: None,
                    upper_texture: 0,
                    middle_texture: 1,
                    lower_texture: 0,
                    line_flags: 0x0001,
                },
            ];

            Self { sector, segs }
        }

        pub fn new_integration() -> Self {
            let sector = BspSector {
                floor_height: 0,
                ceil_height: 128,
                light_level: 160,
                floor_texture: 1,
                ceil_texture: 2,
            };

            let segs = vec![
                BspSeg {
                    v1: Vec2Fixed::from_ints(512, 512),
                    v2: Vec2Fixed::from_ints(512, 0),
                    offset: Fixed16_16::ZERO,
                    angle: ANG270,
                    front_sector: 0,
                    back_sector: None,
                    upper_texture: 0,
                    middle_texture: 1,
                    lower_texture: 0,
                    line_flags: 0x0001,
                },
                BspSeg {
                    v1: Vec2Fixed::from_ints(512, 0),
                    v2: Vec2Fixed::from_ints(0, 0),
                    offset: Fixed16_16::ZERO,
                    angle: ANG180,
                    front_sector: 0,
                    back_sector: None,
                    upper_texture: 0,
                    middle_texture: 1,
                    lower_texture: 0,
                    line_flags: 0x0001,
                },
                BspSeg {
                    v1: Vec2Fixed::from_ints(0, 0),
                    v2: Vec2Fixed::from_ints(0, 512),
                    offset: Fixed16_16::ZERO,
                    angle: ANG90,
                    front_sector: 0,
                    back_sector: None,
                    upper_texture: 0,
                    middle_texture: 1,
                    lower_texture: 0,
                    line_flags: 0x0001,
                },
                BspSeg {
                    v1: Vec2Fixed::from_ints(0, 512),
                    v2: Vec2Fixed::from_ints(512, 512),
                    offset: Fixed16_16::ZERO,
                    angle: Bam::ZERO,
                    front_sector: 0,
                    back_sector: None,
                    upper_texture: 0,
                    middle_texture: 1,
                    lower_texture: 0,
                    line_flags: 0x0001,
                },
            ];

            Self { sector, segs }
        }
    }

    #[test]
    fn mock_traversal_visits_subsector() {
        let map = BspMapData::new_mock();
        let mut visited = Vec::new();
        map.traverse_front_to_back(Vec2Fixed::from_ints(256, 256), &mut |idx| {
            visited.push(idx);
        });
        assert_eq!(visited, vec![0], "traversal must visit subsector 0");
    }

    #[test]
    fn mock_subsector_has_four_segs() {
        let map = BspMapData::new_mock();
        let segs = map.subsector_segs(0);
        assert_eq!(segs.len(), 4, "rectangular room must have 4 wall segments");
    }

    #[test]
    fn mock_seg_front_sector_returns_room() {
        let map = BspMapData::new_mock();
        let segs = map.subsector_segs(0);
        let sector = map.seg_front_sector(&segs[0]);
        assert_eq!(sector.floor_height, 0);
        assert_eq!(sector.ceil_height, 128);
        assert_eq!(sector.light_level, 160);
    }

    #[test]
    fn mock_solid_segs_have_no_back_sector() {
        let map = BspMapData::new_mock();
        let segs = map.subsector_segs(0);
        for (i, seg) in segs.iter().enumerate() {
            assert!(
                map.seg_back_sector(seg).is_none(),
                "seg {i} is a solid wall and must have no back sector"
            );
        }
    }
}
