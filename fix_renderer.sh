#!/bin/bash
set -e

# Update usages in crates/abrash-raycast/src/renderer/bsp.rs
sed -i 's/map: &impl BspMap/map: \&BspMapData/g' crates/abrash-raycast/src/renderer/bsp.rs
sed -i 's/textures: &impl BspTextures/textures: \&BspTextureCache/g' crates/abrash-raycast/src/renderer/bsp.rs

# Also we need to import BspMapData and BspTextureCache
sed -i 's/use crate::bsp::{BspMap, BspSector, BspSeg};/use crate::bsp::{BspMapData, BspSector, BspSeg};/g' crates/abrash-raycast/src/renderer/bsp.rs
sed -i 's/use crate::renderer::bsp_lighting::{BspTextures, colormap_index};/use crate::renderer::bsp_lighting::{BspTextureCache, colormap_index};/g' crates/abrash-raycast/src/renderer/bsp.rs
