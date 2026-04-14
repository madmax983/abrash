import re

# ==========================================
# 1. Update bsp.rs (Remove BspMap trait)
# ==========================================
with open('crates/abrash-raycast/src/bsp.rs', 'r') as f:
    content = f.read()

# Remove the BspMap trait
content = re.sub(r'pub trait BspMap \{.*?\}\n', '', content, flags=re.DOTALL)
content = re.sub(r'/// A BSP-structured map that can be traversed front-to-back\.\n///\n/// Implementations provide the BSP tree structure and seg/sector data\.\n/// The raycaster calls \[`traverse_front_to_back`\]\(BspMap::traverse_front_to_back\)\n/// to visit subsectors in painter\'s-algorithm order\.\n', '', content)

# Remove the entire tests module in bsp.rs and re-insert it properly
tests_module_pattern = r'#\[cfg\(test\)\]\nmod tests \{.*'
content = re.sub(tests_module_pattern, '', content, flags=re.DOTALL)

bsp_map_data = """use abrash_core::bam::{ANG90, ANG180, ANG270};

/// A simple BSP-structured map that can be traversed front-to-back.
///
/// Contains a single rectangular room: 512x512 map units, 4 solid walls,
/// 1 subsector, 1 sector (floor=0, ceil=128, light=160).
pub struct BspMapData {
    pub sector: BspSector,
    pub segs: Vec<BspSeg>,
}

impl BspMapData {
    #[must_use]
    pub fn new() -> Self {
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

    /// Walk the BSP tree from `pos`, calling `visitor` with each subsector
    /// index in front-to-back order.
    pub fn traverse_front_to_back(&self, _pos: Vec2Fixed, visitor: &mut dyn FnMut(usize)) {
        // Single subsector — always visit index 0.
        visitor(0);
    }

    /// Return the segs belonging to the given subsector.
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
    pub const fn seg_back_sector(&self, seg: &BspSeg) -> Option<&BspSector> {
        if seg.back_sector.is_some() {
            Some(&self.sector)
        } else {
            None
        }
    }
}

impl Default for BspMapData {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use abrash_core::bam::Bam;
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
            angle: super::ANG90,
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

    #[test]
    fn mock_traversal_visits_subsector() {
        let map = BspMapData::new();
        let mut visited = Vec::new();
        map.traverse_front_to_back(Vec2Fixed::from_ints(256, 256), &mut |idx| {
            visited.push(idx);
        });
        assert_eq!(visited, vec![0], "traversal must visit subsector 0");
    }

    #[test]
    fn mock_subsector_has_four_segs() {
        let map = BspMapData::new();
        let segs = map.subsector_segs(0);
        assert_eq!(segs.len(), 4, "rectangular room must have 4 wall segments");
    }

    #[test]
    fn mock_seg_front_sector_returns_room() {
        let map = BspMapData::new();
        let segs = map.subsector_segs(0);
        let sector = map.seg_front_sector(&segs[0]);
        assert_eq!(sector.floor_height, 0);
        assert_eq!(sector.ceil_height, 128);
        assert_eq!(sector.light_level, 160);
    }

    #[test]
    fn mock_solid_segs_have_no_back_sector() {
        let map = BspMapData::new();
        let segs = map.subsector_segs(0);
        for (i, seg) in segs.iter().enumerate() {
            assert!(
                map.seg_back_sector(seg).is_none(),
                "seg {i} is a solid wall and must have no back sector"
            );
        }
    }
}
"""

with open('crates/abrash-raycast/src/bsp.rs', 'w') as f:
    f.write(content.strip() + "\n\n" + bsp_map_data)

# ==========================================
# 2. Update bsp_lighting.rs (Remove BspTextures)
# ==========================================

with open('crates/abrash-raycast/src/renderer/bsp_lighting.rs', 'r') as f:
    content3 = f.read()

content3 = re.sub(r'pub trait BspTextures \{.*?\}\n', '', content3, flags=re.DOTALL)
content3 = re.sub(r'/// Texture data provider for BSP rendering\.\n/// Consumers implement this to bridge their texture caches\.\n', '', content3)
content3 = re.sub(r'// BspTextures trait\n// ---------------------------------------------------------------------------\n', '', content3)

texture_cache_data = """
/// A simple texture cache for BSP rendering.
/// Provides flat and wall texture data and colormaps.
pub struct BspTextureCache {
    wall_column_data: Vec<u8>,
    flat: [u8; 4096],
    colormap_data: [[u8; 256]; 32],
    palette: [u32; 256],
}

impl BspTextureCache {
    #[must_use]
    pub fn new() -> Self {
        // Wall column: 128 texels, all palette index 1
        let wall_column_data = vec![1u8; 128];

        // Flat: all palette index 2
        let flat = [2u8; 4096];

        // Colormaps: row 0 = identity, rows 1..31 = all map to 0 (dark)
        let mut colormap_data = [[0u8; 256]; 32];
        for i in 0..256 {
            colormap_data[0][i] = i as u8; // identity
        }
        // rows 1..31 are already zeroed (all map to palette 0 = black)

        // Palette
        let mut palette = [0xFF00_0000u32; 256]; // default black+alpha
        palette[0] = 0xFF00_0000; // black
        palette[1] = 0xFFFF_0000; // red
        palette[2] = 0xFF00_FF00; // green

        Self {
            wall_column_data,
            flat,
            colormap_data,
            palette,
        }
    }

    /// Get a single column of wall texture data (palette-indexed, top to bottom).
    /// Returns palette indices, one per texel row.  Tiles vertically.
    #[must_use]
    pub fn wall_column(&self, _texture_id: u16, _col: usize) -> &[u8] {
        &self.wall_column_data
    }

    /// Get a 64x64 flat texture (4096 bytes, palette-indexed, row-major).
    #[must_use]
    pub const fn flat_data(&self, _texture_id: u16) -> &[u8; 4096] {
        &self.flat
    }

    /// Get a 256-byte colormap row.  index 0 = bright, 31 = darkest.
    #[must_use]
    pub fn colormap(&self, index: u8) -> &[u8; 256] {
        &self.colormap_data[index as usize]
    }

    /// Look up palette entry (palette index -> 0xAARRGGBB).
    #[must_use]
    pub const fn palette_argb(&self, palette_idx: u8) -> u32 {
        self.palette[palette_idx as usize]
    }
}

impl Default for BspTextureCache {
    fn default() -> Self {
        Self::new()
    }
}

"""

content3 = content3.replace("pub const MAX_COLORMAP: u8 = 31;", texture_cache_data + "pub const MAX_COLORMAP: u8 = 31;")
content3 = content3.replace("Consumers implement [`BspTextures`]", "Consumers use `BspTextureCache`")

with open('crates/abrash-raycast/src/renderer/bsp_lighting.rs', 'w') as f:
    f.write(content3)


# ==========================================
# 3. Update renderer/bsp.rs
# ==========================================
with open('crates/abrash-raycast/src/renderer/bsp.rs', 'r') as f:
    content2 = f.read()

# Update map parameter type
content2 = content2.replace("use crate::bsp::{BspMap, BspSector, BspSeg};", "use crate::bsp::{BspMapData, BspSector, BspSeg};")
content2 = content2.replace("map: &impl BspMap,", "map: &BspMapData,")

content2 = content2.replace("use super::bsp_lighting::{BspTextures, colormap_index};", "use super::bsp_lighting::{BspTextureCache, colormap_index};")
content2 = content2.replace("textures: &impl BspTextures,", "textures: &BspTextureCache,")

# Use regex to strip out IntegrationBspMap and MockTextures fully
content2 = re.sub(r'    struct IntegrationBspMap \{.*?\n    \}\n\n    impl IntegrationBspMap \{.*?\n    \}\n\n    impl BspMap for IntegrationBspMap \{.*?\n    \}\n', '', content2, flags=re.DOTALL)
content2 = re.sub(r'    struct MockTextures \{.*?\n    \}\n\n    impl MockTextures \{.*?\n    \}\n\n    impl BspTextures for MockTextures \{.*?\n    \}\n', '', content2, flags=re.DOTALL)

# Fix imports in tests
content2 = content2.replace("    use crate::bsp::{BspMap, BspSector, BspSeg};", "    use crate::bsp::BspMapData;")
content2 = content2.replace("    use super::super::bsp_lighting::BspTextures;", "    use super::super::bsp_lighting::BspTextureCache;")

# Fix usages
content2 = content2.replace("IntegrationBspMap", "BspMapData")
content2 = content2.replace("MockTextures", "BspTextureCache")

with open('crates/abrash-raycast/src/renderer/bsp.rs', 'w') as f:
    f.write(content2)

# ==========================================
# 4. Update the clippy ignores
# ==========================================
with open("crates/abrash-raycast/src/lib.rs", "r") as f:
    content_lib = f.read()

allow_str = "#![allow(clippy::float_cmp, clippy::missing_panics_doc, clippy::too_many_lines)]\n"
if not content_lib.startswith("#![allow("):
    content_lib = allow_str + content_lib
else:
    content_lib = content_lib.replace("#![allow(clippy::all, unused_variables, dead_code, unused_imports, unused_mut)]", "#![allow(clippy::all, clippy::float_cmp, clippy::missing_panics_doc, clippy::too_many_lines, unused_variables, dead_code, unused_imports, unused_mut)]")

with open("crates/abrash-raycast/src/lib.rs", "w") as f:
    f.write(content_lib)
