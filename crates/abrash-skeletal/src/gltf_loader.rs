//! glTF 2.0 loader — extracts meshes, skeletons, animation clips, textures,
//! and PBR materials into abrash types.
//!
//! Gated behind the `gltf` feature flag.

use foldhash::{HashMap, HashMapExt};
use std::path::Path;

use abrash_core::math::{Mat4, Vec2, Vec3, Vec4};
use abrash_core::mesh::Mesh;
use abrash_core::quat::Quat;
use abrash_core::texture::{FilterMode, Texture};
use abrash_core::transform::Transform;

use crate::clip::{AnimationChannel, AnimationClip, ChannelTarget, ChannelValues};
use crate::skeleton::{Joint, JointId, Skeleton};
use crate::skin::{SkinData, SkinnedMesh};

// ---------------------------------------------------------------------------
// Error type
// ---------------------------------------------------------------------------

/// Errors that can occur during glTF loading.
#[derive(Debug)]
pub enum GltfError {
    /// Filesystem I/O error.
    Io(std::io::Error),
    /// glTF parsing / validation error.
    Gltf(gltf::Error),
    /// Image decoding or conversion error.
    Image(String),
    /// Structurally invalid data inside the glTF document.
    InvalidData(String),
}

impl std::fmt::Display for GltfError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(e) => write!(f, "I/O error: {e}"),
            Self::Gltf(e) => write!(f, "glTF error: {e}"),
            Self::Image(msg) => write!(f, "Image error: {msg}"),
            Self::InvalidData(msg) => write!(f, "Invalid data: {msg}"),
        }
    }
}

impl std::error::Error for GltfError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(e) => Some(e),
            Self::Gltf(e) => Some(e),
            Self::Image(_) | Self::InvalidData(_) => None,
        }
    }
}

impl From<gltf::Error> for GltfError {
    fn from(e: gltf::Error) -> Self {
        Self::Gltf(e)
    }
}

impl From<std::io::Error> for GltfError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}

// ---------------------------------------------------------------------------
// Output types
// ---------------------------------------------------------------------------

/// PBR material properties extracted from glTF.
#[derive(Debug, Clone)]
pub struct GltfMaterial {
    /// Human-readable material name.
    pub name: String,
    /// Index into [`GltfScene::textures`] for the base color texture.
    pub base_color_texture: Option<usize>,
    /// RGBA multiplier for the base color.
    pub base_color_factor: [f32; 4],
    /// Metallic factor (0.0 = dielectric, 1.0 = metallic).
    pub metallic_factor: f32,
    /// Roughness factor (0.0 = smooth, 1.0 = rough).
    pub roughness_factor: f32,
    /// Index into [`GltfScene::textures`] for the normal map.
    pub normal_texture: Option<usize>,
}

/// Complete loaded glTF scene.
pub struct GltfScene {
    /// Skinned meshes (mesh geometry + per-vertex bone weights).
    pub meshes: Vec<SkinnedMesh>,
    /// Skeleton hierarchy (if the scene contains a skin).
    pub skeleton: Option<Skeleton>,
    /// Animation clips targeting skeleton joints.
    pub clips: Vec<AnimationClip>,
    /// Decoded textures in 0xAARRGGBB format.
    pub textures: Vec<Texture>,
    /// PBR materials referencing textures by index.
    pub materials: Vec<GltfMaterial>,
}

impl std::fmt::Debug for GltfScene {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GltfScene")
            .field("meshes", &self.meshes.len())
            .field("skeleton", &self.skeleton.is_some())
            .field("clips", &self.clips.len())
            .field("textures", &self.textures.len())
            .field("materials", &self.materials)
            .finish()
    }
}

// ---------------------------------------------------------------------------
// Pixel format helpers (public for testing)
// ---------------------------------------------------------------------------

/// Convert RGBA8 bytes to 0xAARRGGBB u32.
#[must_use]
pub fn rgba_to_argb(r: u8, g: u8, b: u8, a: u8) -> u32 {
    (u32::from(a) << 24) | (u32::from(r) << 16) | (u32::from(g) << 8) | u32::from(b)
}

/// Convert RGB8 bytes (no alpha) to 0xAARRGGBB u32 with full opacity.
#[must_use]
pub fn rgb_to_argb(r: u8, g: u8, b: u8) -> u32 {
    0xFF00_0000 | (u32::from(r) << 16) | (u32::from(g) << 8) | u32::from(b)
}

/// Transpose a column-major 4x4 matrix into row-major [`Mat4`].
///
/// glTF stores matrices in column-major order (OpenGL convention). abrash uses
/// row-major. This function performs the transpose during loading.
#[must_use]
pub fn transpose_col_major_to_mat4(col_major: &[[f32; 4]; 4]) -> Mat4 {
    let mut m = [[0.0_f32; 4]; 4];
    for row in 0..4 {
        for col in 0..4 {
            m[row][col] = col_major[col][row];
        }
    }
    Mat4 { m }
}

// ---------------------------------------------------------------------------
// Main loader
// ---------------------------------------------------------------------------

/// Load a glTF 2.0 file (`.gltf` or `.glb`) and extract all scene data.
///
/// # Errors
///
/// Returns [`GltfError`] on I/O failure, invalid glTF, or malformed data.
pub fn load_gltf(path: &Path) -> Result<GltfScene, GltfError> {
    let (document, buffers, images) = gltf::import(path)?;

    // --- Skeleton -----------------------------------------------------------
    let (skeleton, node_to_joint) = extract_skeleton(&document, &buffers);

    // --- Meshes -------------------------------------------------------------
    let meshes = extract_meshes(&document, &buffers);

    // --- Animation clips ----------------------------------------------------
    let clips = extract_clips(&document, &buffers, &node_to_joint);

    // --- Textures -----------------------------------------------------------
    let textures = extract_textures(&images)?;

    // --- Materials ----------------------------------------------------------
    let materials = extract_materials(&document);

    Ok(GltfScene {
        meshes,
        skeleton,
        clips,
        textures,
        materials,
    })
}

// ---------------------------------------------------------------------------
// Task 9: Mesh + Skeleton + Skin extraction
// ---------------------------------------------------------------------------

/// Extract all meshes (with optional skin data) from the document.
#[allow(clippy::needless_pass_by_value)]
fn extract_primitive(
    primitive: gltf::Primitive<'_>,
    buffers: &[gltf::buffer::Data],
) -> Option<SkinnedMesh> {
    let reader = primitive.reader(|buffer| Some(&buffers[buffer.index()]));

    // Positions (required for a valid mesh)
    let positions: Vec<Vec3> = reader.read_positions().map_or_else(Vec::new, |iter| {
        let mut v = Vec::with_capacity(iter.size_hint().0);
        v.extend(iter.map(|p| Vec3::new(p[0], p[1], p[2])));
        v
    });

    if positions.is_empty() {
        return None;
    }

    // Normals (optional)
    let normals: Vec<Vec3> = reader.read_normals().map_or_else(Vec::new, |iter| {
        let mut v = Vec::with_capacity(iter.size_hint().0);
        v.extend(iter.map(|n| Vec3::new(n[0], n[1], n[2])));
        v
    });

    // Texture coordinates (optional)
    let uvs: Vec<Vec2> = reader.read_tex_coords(0).map_or_else(Vec::new, |iter| {
        let iter = iter.into_f32();
        let mut v = Vec::with_capacity(iter.size_hint().0);
        v.extend(iter.map(|uv| Vec2::new(uv[0], uv[1])));
        v
    });

    // Tangents (optional)
    let tangents: Vec<Vec4> = reader.read_tangents().map_or_else(Vec::new, |iter| {
        let mut v = Vec::with_capacity(iter.size_hint().0);
        v.extend(iter.map(|t| Vec4::new(t[0], t[1], t[2], t[3])));
        v
    });

    // Indices (triangulated)
    let indices: Vec<[usize; 3]> = reader
        .read_indices()
        .map(|iter| {
            let iter_u32 = iter.into_u32();
            let mut indices = Vec::with_capacity(iter_u32.len() / 3);
            let mut iter_usize = iter_u32.map(|i| i as usize);
            while let (Some(a), Some(b), Some(c)) =
                (iter_usize.next(), iter_usize.next(), iter_usize.next())
            {
                indices.push([a, b, c]);
            }
            indices
        })
        .unwrap_or_default();

    let vertex_count = positions.len();

    // Joint indices (optional — only present on skinned meshes)
    let joint_indices: Vec<[u16; 4]> = reader.read_joints(0).map_or_else(Vec::new, |iter| {
        let iter = iter.into_u16();
        let mut v = Vec::with_capacity(iter.size_hint().0);
        v.extend(iter);
        v
    });

    // Weights (optional)
    let weights: Vec<[f32; 4]> = reader.read_weights(0).map_or_else(Vec::new, |iter| {
        let iter = iter.into_f32();
        let mut v = Vec::with_capacity(iter.size_hint().0);
        v.extend(iter);
        v
    });

    let mesh = Mesh {
        vertices: positions,
        indices,
        uvs,
        normals,
        tangents,
    };

    let skin = if joint_indices.is_empty() {
        // No skin data — bind all verts to joint 0 with full weight.
        SkinData {
            joint_indices: vec![[0, 0, 0, 0]; vertex_count],
            weights: vec![[1.0, 0.0, 0.0, 0.0]; vertex_count],
        }
    } else {
        SkinData {
            joint_indices,
            weights,
        }
    };

    Some(SkinnedMesh { mesh, skin })
}

fn extract_meshes(document: &gltf::Document, buffers: &[gltf::buffer::Data]) -> Vec<SkinnedMesh> {
    // ⚡ Bolt: Pre-allocate capacity using exact iterator bounds to eliminate dynamic heap reallocations
    let mut result = Vec::with_capacity(document.meshes().map(|m| m.primitives().count()).sum());

    for mesh in document.meshes() {
        for primitive in mesh.primitives() {
            if let Some(skinned_mesh) = extract_primitive(primitive, buffers) {
                result.push(skinned_mesh);
            }
        }
    }

    result
}

/// Extract the skeleton from the first skin in the document.
///
/// Returns the skeleton and a mapping from glTF node index to final joint index.
fn extract_skeleton(
    document: &gltf::Document,
    buffers: &[gltf::buffer::Data],
) -> (Option<Skeleton>, HashMap<usize, usize>) {
    // ⚡ Bolt: Uses `foldhash::HashMap` instead of the standard library `HashMap` for integer keys (`usize`) to eliminate SipHash cryptographic overhead during parsing.
    let mut node_to_joint: HashMap<usize, usize> = HashMap::default();

    let Some(skin) = document.skins().next() else {
        return (None, node_to_joint);
    };

    // ⚡ Bolt: Uses `extend` with `with_capacity` instead of `.collect::<Vec<_>>()` to avoid intermediate allocations.
    let joints_iter = skin.joints();
    let mut joint_nodes: Vec<gltf::Node<'_>> = Vec::with_capacity(joints_iter.size_hint().0);
    joint_nodes.extend(joints_iter);
    let joint_count = joint_nodes.len();

    if joint_count == 0 {
        return (None, node_to_joint);
    }

    // Build node_index → provisional joint index map
    // ⚡ Bolt: Uses `foldhash::HashMap` with pre-allocated capacity for integer keys to eliminate SipHash cryptographic overhead during node lookup.
    let mut node_idx_to_provisional: HashMap<usize, usize> = HashMap::with_capacity(joint_count);
    for (i, node) in joint_nodes.iter().enumerate() {
        node_idx_to_provisional.insert(node.index(), i);
    }

    // Build child→parent map by scanning each joint node's children.
    // The gltf crate doesn't expose a parent accessor, so we infer parents
    // from the children lists.
    let child_to_parent = build_parent_map(&joint_nodes, &node_idx_to_provisional);

    // Read inverse bind matrices (column-major in glTF → transpose to row-major)
    let ibms: Vec<Mat4> = skin
        .reader(|b| Some(&buffers[b.index()]))
        .read_inverse_bind_matrices()
        .map_or_else(
            || vec![Mat4::identity(); joint_count],
            |iter| {
                let mut v = Vec::with_capacity(iter.size_hint().0);
                v.extend(iter.map(|m| transpose_col_major_to_mat4(&m)));
                v
            },
        );

    // Collect provisional joint data
    let mut provisional: Vec<ProvisionalJointData> = Vec::with_capacity(joint_count);

    for (prov_idx, node) in joint_nodes.iter().enumerate() {
        let (translation, rotation, scale) = node.transform().decomposed();

        let bind_transform = Transform {
            position: Vec3::new(translation[0], translation[1], translation[2]),
            rotation: Quat::new(rotation[0], rotation[1], rotation[2], rotation[3]),
            scale: Vec3::new(scale[0], scale[1], scale[2]),
        };

        // Look up parent provisional index via child→parent map
        let parent_provisional = child_to_parent
            .get(&node.index())
            .and_then(|&parent_node_idx| node_idx_to_provisional.get(&parent_node_idx))
            .copied();

        provisional.push(ProvisionalJointData {
            node_index: node.index(),
            name: node.name().unwrap_or("unnamed").to_string(),
            parent_provisional,
            inverse_bind_matrix: ibms[prov_idx],
            bind_transform,
        });
    }

    // Topological sort: parents before children.
    // Kahn's algorithm on the provisional indices.
    let sorted_order = topological_sort_joints(&provisional);

    // Build old_provisional_index → new_sorted_index mapping
    // ⚡ Bolt: Uses `foldhash::HashMap` for integer key mapping to eliminate SipHash cryptographic overhead when remapping joint indices.
    let mut old_to_new: HashMap<usize, usize> = HashMap::with_capacity(sorted_order.len());
    for (new_idx, &old_idx) in sorted_order.iter().enumerate() {
        old_to_new.insert(old_idx, new_idx);
    }

    // Build final Joint array
    let mut joints = Vec::with_capacity(joint_count);
    for &old_idx in &sorted_order {
        let pj = &mut provisional[old_idx];

        let parent = pj.parent_provisional.and_then(|parent_old| {
            let &parent_new = old_to_new.get(&parent_old)?;
            #[allow(clippy::cast_possible_truncation)]
            Some(JointId(parent_new as u16))
        });

        // ⚡ Bolt: Use `std::mem::take` to extract the owned String name from the provisional joint
        // instead of cloning it. This eliminates an unnecessary per-element heap allocation.
        joints.push(Joint {
            name: std::mem::take(&mut pj.name),
            parent,
            inverse_bind_matrix: pj.inverse_bind_matrix,
            bind_transform: pj.bind_transform,
        });
    }

    // Build final node_index → joint_index map
    for &old_idx in &sorted_order {
        let node_idx = provisional[old_idx].node_index;
        node_to_joint.insert(node_idx, old_to_new[&old_idx]);
    }

    (Some(Skeleton::new(joints)), node_to_joint)
}

/// Build a map from child node index → parent node index by scanning each
/// joint node's children list.
fn build_parent_map(
    joint_nodes: &[gltf::Node<'_>],
    joint_set: &HashMap<usize, usize>,
) -> HashMap<usize, usize> {
    // ⚡ Bolt: Uses `foldhash::HashMap` for integer-to-integer mapping to eliminate SipHash cryptographic overhead when inferring parent nodes.
    let mut child_to_parent: HashMap<usize, usize> = HashMap::with_capacity(joint_nodes.len());

    for node in joint_nodes {
        for child in node.children() {
            if joint_set.contains_key(&child.index()) {
                child_to_parent.insert(child.index(), node.index());
            }
        }
    }

    child_to_parent
}

/// Topological sort of provisional joints using Kahn's algorithm.
///
/// Guarantees that every joint's parent appears before the joint itself.
fn topological_sort_joints(joints: &[ProvisionalJointData]) -> Vec<usize> {
    let n = joints.len();

    // Count in-degree for each joint (0 or 1 since each has at most one parent)
    let mut in_degree = vec![0_usize; n];
    for j in joints {
        if j.parent_provisional.is_some() {
            // The child itself has in-degree 1
        }
    }
    // Actually: in_degree of a joint = 1 if it has a parent, 0 if root.
    for (i, j) in joints.iter().enumerate() {
        if j.parent_provisional.is_some() {
            in_degree[i] = 1;
        }
    }

    // Build children lists
    // ⚡ Bolt: Pre-allocate capacities for the `children` adjacency list.
    // In typical glTF skeletons, most joints have 0, 1, or 2 children.
    // We count the exact children per parent and allocate inner Vecs perfectly
    // sized to eliminate unnecessary empty Vec clones and heap reallocations.
    let mut child_counts = vec![0; n];
    for j in joints {
        if let Some(parent) = j.parent_provisional {
            child_counts[parent] += 1;
        }
    }

    let mut children: Vec<Vec<usize>> = Vec::with_capacity(n);
    for count in child_counts {
        children.push(Vec::with_capacity(count));
    }

    for (i, j) in joints.iter().enumerate() {
        if let Some(parent) = j.parent_provisional {
            children[parent].push(i);
        }
    }

    // Start with roots (in_degree == 0)
    let mut queue: Vec<usize> = Vec::with_capacity(n);
    for (i, &deg) in in_degree.iter().enumerate() {
        if deg == 0 {
            queue.push(i);
        }
    }

    // ⚡ Bolt: Use a boolean vector for O(1) lookups during orphan resolution,
    // avoiding the O(N^2) behavior of `!sorted.contains(&i)`.
    let mut is_sorted = vec![false; n];
    let mut sorted = Vec::with_capacity(n);
    while let Some(idx) = queue.pop() {
        sorted.push(idx);
        is_sorted[idx] = true;
        for &child in &children[idx] {
            in_degree[child] -= 1;
            if in_degree[child] == 0 {
                queue.push(child);
            }
        }
    }

    // If there are orphaned joints (cycles or missing parents), append them
    if sorted.len() < n {
        for (i, &sorted_flag) in is_sorted.iter().enumerate().take(n) {
            if !sorted_flag {
                sorted.push(i);
            }
        }
    }

    sorted
}

/// Internal provisional joint data used during skeleton construction.
struct ProvisionalJointData {
    node_index: usize,
    name: String,
    parent_provisional: Option<usize>,
    inverse_bind_matrix: Mat4,
    bind_transform: Transform,
}

// ---------------------------------------------------------------------------
// Task 10: Animation clip extraction
// ---------------------------------------------------------------------------

/// Extract animation clips from the document.
fn extract_clips(
    document: &gltf::Document,
    buffers: &[gltf::buffer::Data],
    node_to_joint: &HashMap<usize, usize>,
) -> Vec<AnimationClip> {
    // ⚡ Bolt: Pre-allocate capacity using exact iterator bounds to eliminate dynamic heap reallocations
    let mut clips = Vec::with_capacity(document.animations().count());

    for animation in document.animations() {
        // ⚡ Bolt: Pre-allocate capacity using exact iterator bounds to eliminate dynamic heap reallocations
        let mut channels = Vec::with_capacity(animation.channels().count());
        let mut max_time: f32 = 0.0;

        for channel in animation.channels() {
            let target_node = channel.target().node().index();

            // Skip channels targeting nodes that aren't skeleton joints
            let Some(&joint_idx) = node_to_joint.get(&target_node) else {
                continue;
            };

            let reader = channel.reader(|b| Some(&buffers[b.index()]));

            // Read timestamps
            let Some(timestamps) = reader.read_inputs() else {
                continue;
            };
            let mut timestamps_vec: Vec<f32> = Vec::with_capacity(timestamps.size_hint().0);
            timestamps_vec.extend(timestamps);

            if let Some(&last) = timestamps_vec.last() {
                max_time = max_time.max(last);
            }

            // Read output values based on channel property
            let Some(outputs) = reader.read_outputs() else {
                continue;
            };

            let (target, values) = match outputs {
                gltf::animation::util::ReadOutputs::Translations(iter) => {
                    let mut vals = Vec::with_capacity(iter.size_hint().0);
                    vals.extend(iter.map(|t| Vec3::new(t[0], t[1], t[2])));
                    (ChannelTarget::Translation, ChannelValues::Translation(vals))
                }
                gltf::animation::util::ReadOutputs::Rotations(iter) => {
                    let iter = iter
                        .into_f32()
                        .map(|r| Quat::new(r[0], r[1], r[2], r[3]).normalize());
                    let mut vals = Vec::with_capacity(iter.size_hint().0);
                    vals.extend(iter);
                    (ChannelTarget::Rotation, ChannelValues::Rotation(vals))
                }
                gltf::animation::util::ReadOutputs::Scales(iter) => {
                    let mut vals = Vec::with_capacity(iter.size_hint().0);
                    vals.extend(iter.map(|s| Vec3::new(s[0], s[1], s[2])));
                    (ChannelTarget::Scale, ChannelValues::Scale(vals))
                }
                gltf::animation::util::ReadOutputs::MorphTargetWeights(_) => {
                    // Morph targets are not supported
                    continue;
                }
            };

            // Only include channels with at least 2 keyframes (required by AnimationChannel)
            if timestamps_vec.len() >= 2 {
                #[allow(clippy::cast_possible_truncation)]
                channels.push(AnimationChannel {
                    joint: JointId(joint_idx as u16),
                    target,
                    timestamps: timestamps_vec,
                    values,
                });
            }
        }

        let name = animation.name().unwrap_or("unnamed").to_string();

        clips.push(AnimationClip {
            name,
            duration: max_time,
            channels,
        });
    }

    clips
}

// ---------------------------------------------------------------------------
// Task 11: Texture + PBR material extraction
// ---------------------------------------------------------------------------

/// Extract textures from the glTF image data, converting to 0xAARRGGBB format.
fn extract_textures(images: &[gltf::image::Data]) -> Result<Vec<Texture>, GltfError> {
    let mut textures = Vec::with_capacity(images.len());

    for img in images {
        let mut texture = Texture::new(img.width, img.height).map_err(|e| {
            GltfError::Image(format!(
                "Failed to create {}x{} texture: {e}",
                img.width, img.height
            ))
        })?;
        texture.filter_mode = FilterMode::Bilinear;

        let w = img.width as usize;
        let pixel_count = w * (img.height as usize);

        match img.format {
            gltf::image::Format::R8G8B8A8 => {
                for i in 0..pixel_count {
                    let base = i * 4;
                    if base + 3 < img.pixels.len() {
                        let color = rgba_to_argb(
                            img.pixels[base],
                            img.pixels[base + 1],
                            img.pixels[base + 2],
                            img.pixels[base + 3],
                        );
                        #[allow(clippy::cast_possible_truncation)]
                        texture.set_pixel((i % w) as u32, (i / w) as u32, color);
                    }
                }
            }
            gltf::image::Format::R8G8B8 => {
                for i in 0..pixel_count {
                    let base = i * 3;
                    if base + 2 < img.pixels.len() {
                        let color = rgb_to_argb(
                            img.pixels[base],
                            img.pixels[base + 1],
                            img.pixels[base + 2],
                        );
                        #[allow(clippy::cast_possible_truncation)]
                        texture.set_pixel((i % w) as u32, (i / w) as u32, color);
                    }
                }
            }
            other => {
                return Err(GltfError::Image(format!(
                    "Unsupported image format: {other:?}"
                )));
            }
        }

        textures.push(texture);
    }

    Ok(textures)
}

/// Extract PBR materials from the document.
fn extract_materials(document: &gltf::Document) -> Vec<GltfMaterial> {
    let mut materials = Vec::with_capacity(document.materials().len());
    materials.extend(document.materials().map(|material| {
        let pbr = material.pbr_metallic_roughness();
        let base_color_tex = pbr
            .base_color_texture()
            .map(|info| info.texture().source().index());
        let normal_tex = material
            .normal_texture()
            .map(|info| info.texture().source().index());

        GltfMaterial {
            name: material.name().unwrap_or("unnamed").to_string(),
            base_color_texture: base_color_tex,
            base_color_factor: pbr.base_color_factor(),
            metallic_factor: pbr.metallic_factor(),
            roughness_factor: pbr.roughness_factor(),
            normal_texture: normal_tex,
        }
    }));
    materials
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // --- Pixel conversion tests ---

    #[test]
    fn rgba_to_argb_basic() {
        // R=0xFF, G=0x80, B=0x40, A=0xC0
        let result = rgba_to_argb(0xFF, 0x80, 0x40, 0xC0);
        assert_eq!(result, 0xC0FF_8040);
    }

    #[test]
    fn rgba_to_argb_fully_opaque_white() {
        let result = rgba_to_argb(0xFF, 0xFF, 0xFF, 0xFF);
        assert_eq!(result, 0xFFFF_FFFF);
    }

    #[test]
    fn rgba_to_argb_fully_transparent_black() {
        let result = rgba_to_argb(0, 0, 0, 0);
        assert_eq!(result, 0x0000_0000);
    }

    #[test]
    fn rgba_to_argb_red_only() {
        let result = rgba_to_argb(0xFF, 0, 0, 0xFF);
        assert_eq!(result, 0xFFFF_0000);
    }

    #[test]
    fn rgba_to_argb_green_channel() {
        let result = rgba_to_argb(0, 0xAB, 0, 0xFF);
        assert_eq!(result, 0xFF00_AB00);
    }

    #[test]
    fn rgba_to_argb_blue_channel() {
        let result = rgba_to_argb(0, 0, 0xCD, 0xFF);
        assert_eq!(result, 0xFF00_00CD);
    }

    #[test]
    fn rgb_to_argb_basic() {
        let result = rgb_to_argb(0x10, 0x20, 0x30);
        assert_eq!(result, 0xFF10_2030);
    }

    #[test]
    fn rgb_to_argb_always_opaque() {
        let result = rgb_to_argb(0, 0, 0);
        assert_eq!(result & 0xFF00_0000, 0xFF00_0000);
    }

    #[test]
    fn rgb_to_argb_white() {
        let result = rgb_to_argb(0xFF, 0xFF, 0xFF);
        assert_eq!(result, 0xFFFF_FFFF);
    }

    // --- Matrix transpose tests ---

    #[test]
    fn transpose_identity_is_identity() {
        let col_major = [
            [1.0, 0.0, 0.0, 0.0],
            [0.0, 1.0, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
            [0.0, 0.0, 0.0, 1.0],
        ];
        let result = transpose_col_major_to_mat4(&col_major);
        let identity = Mat4::identity();
        for row in 0..4 {
            for col in 0..4 {
                assert!(
                    (result.m[row][col] - identity.m[row][col]).abs() < f32::EPSILON,
                    "Mismatch at [{row}][{col}]"
                );
            }
        }
    }

    #[test]
    fn transpose_known_matrix() {
        // Column-major layout: col_major[col][row]
        // col0 = [1, 5, 9, 13]
        // col1 = [2, 6, 10, 14]
        // col2 = [3, 7, 11, 15]
        // col3 = [4, 8, 12, 16]
        let col_major = [
            [1.0, 5.0, 9.0, 13.0],
            [2.0, 6.0, 10.0, 14.0],
            [3.0, 7.0, 11.0, 15.0],
            [4.0, 8.0, 12.0, 16.0],
        ];
        let result = transpose_col_major_to_mat4(&col_major);
        // Row-major result:
        // row0 = [1, 2, 3, 4]
        // row1 = [5, 6, 7, 8]
        // row2 = [9, 10, 11, 12]
        // row3 = [13, 14, 15, 16]
        let expected = [
            [1.0, 2.0, 3.0, 4.0],
            [5.0, 6.0, 7.0, 8.0],
            [9.0, 10.0, 11.0, 12.0],
            [13.0, 14.0, 15.0, 16.0],
        ];
        #[allow(clippy::needless_range_loop)]
        for row in 0..4 {
            for col in 0..4 {
                assert!(
                    (result.m[row][col] - expected[row][col]).abs() < f32::EPSILON,
                    "Mismatch at [{row}][{col}]: got {}, expected {}",
                    result.m[row][col],
                    expected[row][col]
                );
            }
        }
    }

    #[test]
    fn transpose_translation_matrix() {
        // Column-major OpenGL translation matrix:
        // col0=[1,0,0,0], col1=[0,1,0,0], col2=[0,0,1,0], col3=[tx,ty,tz,1]
        let tx = 3.0_f32;
        let ty = 4.0;
        let tz = 5.0;
        let col_major = [
            [1.0, 0.0, 0.0, 0.0],
            [0.0, 1.0, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
            [tx, ty, tz, 1.0],
        ];
        let result = transpose_col_major_to_mat4(&col_major);
        // After transpose: row0=[1,0,0,tx], row1=[0,1,0,ty], row2=[0,0,1,tz], row3=[0,0,0,1]
        assert!((result.m[0][3] - tx).abs() < f32::EPSILON);
        assert!((result.m[1][3] - ty).abs() < f32::EPSILON);
        assert!((result.m[2][3] - tz).abs() < f32::EPSILON);
        assert!((result.m[3][3] - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn transpose_double_transpose_is_original() {
        let original = [
            [1.0, 2.0, 3.0, 4.0],
            [5.0, 6.0, 7.0, 8.0],
            [9.0, 10.0, 11.0, 12.0],
            [13.0, 14.0, 15.0, 16.0],
        ];
        let first = transpose_col_major_to_mat4(&original);
        // Transpose again: treat the row-major Mat4.m as if it were column-major
        let second = transpose_col_major_to_mat4(&first.m);
        #[allow(clippy::needless_range_loop)]
        for row in 0..4 {
            for col in 0..4 {
                assert!(
                    (second.m[row][col] - original[row][col]).abs() < f32::EPSILON,
                    "Double transpose mismatch at [{row}][{col}]"
                );
            }
        }
    }
}
