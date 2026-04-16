import re

with open('crates/abrash-raycast/src/renderer/bsp.rs', 'r') as f:
    lines = f.readlines()

new_lines = []
skip = False
for i, line in enumerate(lines):
    if line.startswith('    struct MockTextures {'):
        skip = True
    elif line.startswith('    impl MockTextures {'):
        skip = True
    elif line.startswith('    impl BspTextures for MockTextures {'):
        skip = True
    elif line.startswith('    struct IntegrationBspMap {'):
        skip = True
    elif line.startswith('    impl IntegrationBspMap {'):
        skip = True
    elif line.startswith('    impl BspMap for IntegrationBspMap {'):
        skip = True

    if not skip:
        # Replacements
        line = line.replace('MockTextures::new()', 'BspTextureCache::new()')
        line = line.replace('IntegrationBspMap::new()', 'BspMapData::new_integration()')
        line = line.replace('map: &impl BspMap', 'map: &BspMapData')
        line = line.replace('textures: &impl BspTextures', 'textures: &BspTextureCache')
        line = line.replace('use crate::bsp::{BspMap, BspSector, BspSeg};', 'use crate::bsp::{BspMapData, BspSector, BspSeg};')
        line = line.replace('use crate::renderer::bsp_lighting::{BspTextures, colormap_index};', 'use crate::renderer::bsp_lighting::{BspTextureCache, colormap_index};')
        new_lines.append(line)

    if skip and line.startswith('    }'):
        # Check if the next line is empty or part of the next block
        if i + 1 < len(lines) and (lines[i+1].strip() == '' or lines[i+1].startswith('    ')):
            skip = False

with open('crates/abrash-raycast/src/renderer/bsp.rs', 'w') as f:
    f.writelines(new_lines)
