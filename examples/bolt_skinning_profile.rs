//! Deterministic profiling harness for the glTF viewer's per-frame skinning +
//! mesh-update pipeline (`examples/gltf_viewer.rs`'s `update()`: forward
//! kinematics, `skin_vertices`, and rebuilding the `Mesh` handed to
//! `CpuRenderer::update_mesh`).
//!
//! Not a criterion benchmark: wall-clock is unreliable on this machine. This
//! binary is meant to be run under `valgrind --tool=callgrind` / `--tool=dhat`
//! to get deterministic instruction and allocation counts for a realistic
//! workload — a 12,000-vertex / 20,000-triangle skinned mesh (typical
//! mid-poly character), a 50-joint chain skeleton, pose varying every frame
//! so no frame is identical to the last (avoids constant-folding).
//!
//! Mirrors `GltfViewerApp::update()` exactly for the CPU-side work: forward
//! kinematics via the allocating `compute_global_transforms` /
//! `compute_skin_matrices` (the same calls the real app makes), `skin_vertices`,
//! then updating the scratch mesh's vertices via `clone_from(...)` — the exact
//! sequence built right before `renderer.update_mesh(mh, updated_mesh)`. The
//! actual GPU/device upload is skipped (it needs a window/softbuffer surface
//! and isn't part of the CPU cost under test); everything up to the `Mesh`
//! that would be handed to it is reproduced faithfully.
//!
//! `updated_mesh` is cloned from `skinned_mesh.mesh` once, outside the frame
//! loop, and only its `.vertices` field is overwritten per frame — this
//! harness originally cloned the whole mesh inside the loop every frame (see
//! the baseline commit that added this file); that was the bug this harness
//! exists to catch a regression of.
use abrash::math::{Vec2, Vec3, Vec4};
use abrash::mesh::Mesh;
use abrash::quat::Quat;
use abrash::skeletal::pose::Pose;
use abrash::skeletal::skeleton::{Joint, JointId, Skeleton};
use abrash::skeletal::skin::{SkinData, SkinnedMesh};
use abrash::skeletal::skinning::skin_vertices;
use abrash::transform::Transform;
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use std::hint::black_box;

const VERTEX_COUNT: usize = 12_000;
const TRIANGLE_COUNT: usize = 20_000;
const JOINT_COUNT: usize = 50;

fn make_skeleton() -> Skeleton {
    let mut joints = Vec::with_capacity(JOINT_COUNT);
    for i in 0..JOINT_COUNT {
        joints.push(Joint {
            name: format!("joint{i}"),
            parent: if i == 0 {
                None
            } else {
                Some(JointId((i - 1) as u16))
            },
            inverse_bind_matrix: abrash::math::Mat4::identity(),
            bind_transform: Transform::from_position(Vec3::new(0.0, i as f32 * 0.1, 0.0)),
        });
    }
    Skeleton::new(joints)
}

fn make_skinned_mesh(rng: &mut StdRng) -> SkinnedMesh {
    let mut mesh = Mesh::with_capacity(VERTEX_COUNT, TRIANGLE_COUNT);
    for _ in 0..VERTEX_COUNT {
        mesh.vertices.push(Vec3::new(
            rng.gen_range(-1.0..1.0),
            rng.gen_range(0.0..5.0),
            rng.gen_range(-1.0..1.0),
        ));
        mesh.normals.push(Vec3::new(
            rng.gen_range(-1.0..1.0),
            rng.gen_range(-1.0..1.0),
            rng.gen_range(-1.0..1.0),
        ));
        mesh.uvs
            .push(Vec2::new(rng.gen_range(0.0..1.0), rng.gen_range(0.0..1.0)));
        mesh.tangents.push(Vec4::new(
            rng.gen_range(-1.0..1.0),
            rng.gen_range(-1.0..1.0),
            rng.gen_range(-1.0..1.0),
            1.0,
        ));
    }
    for _ in 0..TRIANGLE_COUNT {
        mesh.indices.push([
            rng.gen_range(0..VERTEX_COUNT),
            rng.gen_range(0..VERTEX_COUNT),
            rng.gen_range(0..VERTEX_COUNT),
        ]);
    }

    let mut joint_indices = Vec::with_capacity(VERTEX_COUNT);
    let mut weights = Vec::with_capacity(VERTEX_COUNT);
    for _ in 0..VERTEX_COUNT {
        // Realistic bone-influence distribution: ~40% single-bone, ~40%
        // two-bone blend, ~20% four-bone blend (typical of skinned character
        // meshes, heaviest near joints).
        let base = rng.gen_range(0..JOINT_COUNT) as u16;
        let roll: f32 = rng.gen_range(0.0..1.0);
        let (idx, w) = if roll < 0.4 {
            ([base, 0, 0, 0], [1.0, 0.0, 0.0, 0.0])
        } else if roll < 0.8 {
            let b2 = ((base as usize + 1) % JOINT_COUNT) as u16;
            ([base, b2, 0, 0], [0.7, 0.3, 0.0, 0.0])
        } else {
            let b2 = ((base as usize + 1) % JOINT_COUNT) as u16;
            let b3 = ((base as usize + 2) % JOINT_COUNT) as u16;
            let b4 = ((base as usize + 3) % JOINT_COUNT) as u16;
            ([base, b2, b3, b4], [0.4, 0.3, 0.2, 0.1])
        };
        joint_indices.push(idx);
        weights.push(w);
    }

    SkinnedMesh {
        mesh,
        skin: SkinData {
            joint_indices,
            weights,
        },
    }
}

fn main() {
    let frames: usize = std::env::args()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(60);

    let mut rng = StdRng::seed_from_u64(0xB017_5C1D);
    let skeleton = make_skeleton();
    let skinned_mesh = make_skinned_mesh(&mut rng);

    let bind_pose = Pose::from_bind(&skeleton);
    let mut pose = bind_pose.clone();
    let mut skinned_positions = vec![Vec3::ZERO; VERTEX_COUNT];

    // Scratch mesh reused every frame: only `.vertices` changes frame to
    // frame, so it's cloned once here instead of inside the loop (matches
    // the fix in `examples/gltf_viewer.rs`'s `GltfViewerApp::update()`).
    let mut updated_mesh = skinned_mesh.mesh.clone();

    let mut checksum: u64 = 0;

    for frame in 0..frames {
        // Vary every joint's local rotation a little each frame so no two
        // frames produce the same skin matrices (avoids constant-folding).
        for (i, local) in pose.local_transforms.iter_mut().enumerate() {
            let angle = frame as f32 * 0.013 + i as f32 * 0.021;
            local.rotation = Quat::from_axis_angle(Vec3::new(0.0, 1.0, 0.0), angle);
        }

        // Exactly what `GltfViewerApp::update()` does per frame (the two
        // allocating forward-kinematics calls, then `skin_vertices`).
        let globals = skeleton.compute_global_transforms(&pose);
        let skin_mats = skeleton.compute_skin_matrices(&globals);
        skin_vertices(&skinned_mesh, &skin_mats, &mut skinned_positions);

        // Exactly what `GltfViewerApp::update()` does right before
        // `renderer.update_mesh(mh, &updated_mesh)`: overwrite the reused
        // scratch mesh's vertices with the skinned positions in place.
        updated_mesh.vertices.clone_from(&skinned_positions);
        black_box(&updated_mesh);

        checksum = checksum.wrapping_add(
            skinned_positions
                .iter()
                .fold(0u64, |acc, v| acc.wrapping_add(v.x.to_bits() as u64)),
        );
    }

    println!("frames={frames} checksum={checksum}");
}
