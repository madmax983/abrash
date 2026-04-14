import re

with open('crates/abrash-raycast/src/bsp.rs', 'r') as f:
    content = f.read()

# Fix the test data: The original MockBspMap had tests passing but IntegrationBspMap didn't pass these two tests.
# That means I should restore the `BspMapData` segs back to the original `MockBspMap` segs and use them for tests!
# Oh wait, the issue is that MockBspMap was used for bsp.rs tests and IntegrationBspMap for renderer/bsp.rs tests.
# If I unify them, one set of tests breaks.
# Wait, MockBspMap and IntegrationBspMap had DIFFERENT segs!
# MockBspMap: (0,0)->(512,0), (512,0)->(512,512), (512,512)->(0,512), (0,512)->(0,0)
# IntegrationBspMap: (512,512)->(512,0), (512,0)->(0,0), (0,0)->(0,512), (0,512)->(512,512)
# Ah! They are completely different!

# The easiest way to fix this is to add a `new_integration()` constructor.
bsp_map_data_extra = """
    #[must_use]
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
"""

content = content.replace("pub fn traverse_front_to_back", bsp_map_data_extra + "\n    pub fn traverse_front_to_back")

# Also I need to revert BspMapData::new() back to MockBspMap's version
content = re.sub(r'pub fn new\(\) -> Self \{.*?\}\n\n    #\[must_use\]\n    pub fn new_integration', """pub fn new() -> Self {
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

    #[must_use]
    pub fn new_integration""", content, flags=re.DOTALL)

with open('crates/abrash-raycast/src/bsp.rs', 'w') as f:
    f.write(content)

with open('crates/abrash-raycast/src/renderer/bsp.rs', 'r') as f:
    content2 = f.read()

content2 = content2.replace("BspMapData::new()", "BspMapData::new_integration()")

with open('crates/abrash-raycast/src/renderer/bsp.rs', 'w') as f:
    f.write(content2)
