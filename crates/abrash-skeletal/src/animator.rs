//! Top-level skeleton animation driver.
//!
//! Owns per-bone `Timeline`s and an animation clock model. Call
//! [`SkeletonAnimator::tick`] each frame to advance the animation
//! and produce a [`Pose`].

use abrash_core::math::Vec3;
use abrash_core::quat::Quat;

use abrash_anim::clock::PlaybackMode;
use abrash_anim::timeline::Timeline;

use crate::clip::{AnimationClip, ChannelTarget, ChannelValues};
use crate::clip_evaluable::{channel_to_quat_evaluable, channel_to_vec3_evaluable};
use crate::pose::Pose;
use crate::skeleton::Skeleton;

/// Per-bone animation timelines for position, rotation, and scale.
pub struct BoneAnimator {
    pub position: Option<Timeline<Vec3>>,
    pub rotation: Option<Timeline<Quat>>,
    pub scale: Option<Timeline<Vec3>>,
}

/// Top-level skeleton animation driver.
///
/// Owns a skeleton, per-bone animators, and a playback mode.
/// Call [`tick`](Self::tick) each frame to advance the animation and produce a [`Pose`].
pub struct SkeletonAnimator {
    skeleton: Skeleton,
    bone_animators: Vec<BoneAnimator>,
    bind_pose: Pose,
    current_pose: Pose,
    #[allow(dead_code)]
    playback: PlaybackMode,
}

impl SkeletonAnimator {
    /// Build from a skeleton and animation clip.
    ///
    /// Converts each channel into `Timeline<T>` via the Evaluable bridge.
    /// Channels referencing out-of-range joint indices are silently skipped.
    #[must_use]
    pub fn new(skeleton: Skeleton, clip: &AnimationClip, playback: PlaybackMode) -> Self {
        let joint_count = skeleton.joint_count();
        let bind_pose = Pose::from_bind(&skeleton);
        let current_pose = bind_pose.clone();

        // Initialize all bones with no animation
        // ⚡ Bolt: Replace `.collect()` with `Vec::with_capacity` to eliminate iterator overhead and allocations.
        let mut bone_animators: Vec<BoneAnimator> = Vec::with_capacity(joint_count);
        for _ in 0..joint_count {
            bone_animators.push(BoneAnimator {
                position: None,
                rotation: None,
                scale: None,
            });
        }

        // Populate from clip channels
        for channel in &clip.channels {
            let joint_idx = channel.joint.0 as usize;
            if joint_idx >= joint_count {
                continue;
            }

            let animator = &mut bone_animators[joint_idx];
            match (&channel.target, &channel.values) {
                (ChannelTarget::Translation, ChannelValues::Translation(_)) => {
                    let evaluable = channel_to_vec3_evaluable(channel);
                    let tl = Timeline::from_evaluable(evaluable, playback);
                    animator.position = Some(tl);
                }
                (ChannelTarget::Rotation, ChannelValues::Rotation(_)) => {
                    let evaluable = channel_to_quat_evaluable(channel);
                    let tl = Timeline::from_evaluable(evaluable, playback);
                    animator.rotation = Some(tl);
                }
                (ChannelTarget::Scale, ChannelValues::Scale(_)) => {
                    let evaluable = channel_to_vec3_evaluable(channel);
                    let tl = Timeline::from_evaluable(evaluable, playback);
                    animator.scale = Some(tl);
                }
                _ => {} // Mismatched target/values — skip
            }
        }

        Self {
            skeleton,
            bone_animators,
            bind_pose,
            current_pose,
            playback,
        }
    }

    /// Advance the animation by `dt` seconds. The updated pose can be retrieved via [`current_pose`](Self::current_pose).
    ///
    /// Bones without animation channels keep their bind-pose values.
    pub fn tick(&mut self, dt: f32) {
        // ⚡ Bolt: Use `clone_from` to copy bind_pose into current_pose, completely
        // avoiding O(N) heap allocations per frame by reusing the existing Vec capacity.
        self.current_pose
            .local_transforms
            .clone_from(&self.bind_pose.local_transforms);

        for (i, animator) in self.bone_animators.iter_mut().enumerate() {
            if let Some(ref mut tl) = animator.position {
                self.current_pose.local_transforms[i].position = tl.tick(dt).value;
            }
            if let Some(ref mut tl) = animator.rotation {
                self.current_pose.local_transforms[i].rotation = tl.tick(dt).value;
            }
            if let Some(ref mut tl) = animator.scale {
                self.current_pose.local_transforms[i].scale = tl.tick(dt).value;
            }
        }
    }

    /// Get a reference to the current animated pose.
    #[must_use]
    pub const fn current_pose(&self) -> &Pose {
        &self.current_pose
    }

    /// Get a reference to the skeleton.
    #[must_use]
    pub const fn skeleton(&self) -> &Skeleton {
        &self.skeleton
    }

    /// Whether all animation timelines have completed.
    #[must_use]
    pub fn is_completed(&self) -> bool {
        self.bone_animators.iter().all(|ba| {
            ba.position.as_ref().is_none_or(Timeline::is_completed)
                && ba.rotation.as_ref().is_none_or(Timeline::is_completed)
                && ba.scale.as_ref().is_none_or(Timeline::is_completed)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clip::{AnimationChannel, ChannelTarget, ChannelValues};
    use crate::skeleton::{Joint, JointId};
    use abrash_core::math::Mat4;
    use std::f32::consts::FRAC_PI_2;

    const EPSILON: f32 = 1e-3;

    /// Advance a `SkeletonAnimator` by `total` seconds in small increments
    /// to respect `MAX_DELTA_SECS` (0.1s) clamping in `AnimationClock`.
    fn tick_secs(animator: &mut SkeletonAnimator, total: f32) -> Pose {
        let step = 0.05;
        let steps = (total / step) as u32;
        let remainder = total - (steps as f32 * step);
        animator.tick(0.0);
        for _ in 0..steps {
            animator.tick(step);
        }
        if remainder > f32::EPSILON {
            animator.tick(remainder);
        }
        animator.current_pose().clone()
    }

    fn make_single_bone_skeleton() -> Skeleton {
        Skeleton::new(vec![Joint {
            name: "root".to_string(),
            parent: None,
            inverse_bind_matrix: Mat4::identity(),
            bind_transform: abrash_core::transform::Transform::identity(),
        }])
    }

    fn make_two_bone_skeleton() -> Skeleton {
        Skeleton::new(vec![
            Joint {
                name: "root".to_string(),
                parent: None,
                inverse_bind_matrix: Mat4::identity(),
                bind_transform: abrash_core::transform::Transform::identity(),
            },
            Joint {
                name: "child".to_string(),
                parent: Some(JointId(0)),
                inverse_bind_matrix: Mat4::identity(),
                bind_transform: abrash_core::transform::Transform::from_position(Vec3::new(
                    1.0, 0.0, 0.0,
                )),
            },
        ])
    }

    #[test]
    fn single_bone_translation_interpolates_at_midpoint() {
        let skeleton = make_single_bone_skeleton();
        let clip = AnimationClip {
            name: "slide".to_string(),
            duration: 2.0,
            channels: vec![AnimationChannel {
                joint: JointId(0),
                target: ChannelTarget::Translation,
                timestamps: vec![0.0, 2.0],
                values: ChannelValues::Translation(vec![Vec3::ZERO, Vec3::new(10.0, 0.0, 0.0)]),
            }],
        };

        let mut anim = SkeletonAnimator::new(skeleton, &clip, PlaybackMode::Once);

        // Tick to midpoint (1 second into 2-second animation)
        let pose = tick_secs(&mut anim, 1.0);

        // Should be approximately (5, 0, 0)
        let pos = pose.local_transforms[0].position;
        assert!((pos.x - 5.0).abs() < 0.5, "x: expected ~5.0, got {}", pos.x);
        assert!((pos.y).abs() < EPSILON, "y: expected ~0.0, got {}", pos.y);
        assert!((pos.z).abs() < EPSILON, "z: expected ~0.0, got {}", pos.z);
    }

    #[test]
    fn multi_bone_completes_after_full_duration() {
        let skeleton = make_two_bone_skeleton();
        let clip = AnimationClip {
            name: "multi".to_string(),
            duration: 1.0,
            channels: vec![
                AnimationChannel {
                    joint: JointId(0),
                    target: ChannelTarget::Translation,
                    timestamps: vec![0.0, 1.0],
                    values: ChannelValues::Translation(vec![Vec3::ZERO, Vec3::new(5.0, 0.0, 0.0)]),
                },
                AnimationChannel {
                    joint: JointId(1),
                    target: ChannelTarget::Rotation,
                    timestamps: vec![0.0, 1.0],
                    values: ChannelValues::Rotation(vec![
                        Quat::identity(),
                        Quat::from_axis_angle(Vec3::new(0.0, 1.0, 0.0), FRAC_PI_2),
                    ]),
                },
            ],
        };

        let mut anim = SkeletonAnimator::new(skeleton, &clip, PlaybackMode::Once);
        assert!(!anim.is_completed());

        // Tick through full duration
        let _pose = tick_secs(&mut anim, 1.0);
        assert!(
            anim.is_completed(),
            "Should be completed after full duration"
        );
    }

    #[test]
    fn looping_playback_never_completes() {
        let skeleton = make_single_bone_skeleton();
        let clip = AnimationClip {
            name: "loop".to_string(),
            duration: 1.0,
            channels: vec![AnimationChannel {
                joint: JointId(0),
                target: ChannelTarget::Translation,
                timestamps: vec![0.0, 1.0],
                values: ChannelValues::Translation(vec![Vec3::ZERO, Vec3::ONE]),
            }],
        };

        let mut anim = SkeletonAnimator::new(skeleton, &clip, PlaybackMode::Loop);

        // Tick well past the duration
        let _pose = tick_secs(&mut anim, 3.0);
        assert!(
            !anim.is_completed(),
            "Looping animation should never complete"
        );
    }

    #[test]
    fn bones_without_channels_keep_bind_pose() {
        let skeleton = make_two_bone_skeleton();

        // Only animate joint 0 — joint 1 should keep bind pose
        let clip = AnimationClip {
            name: "partial".to_string(),
            duration: 1.0,
            channels: vec![AnimationChannel {
                joint: JointId(0),
                target: ChannelTarget::Translation,
                timestamps: vec![0.0, 1.0],
                values: ChannelValues::Translation(vec![Vec3::ZERO, Vec3::new(10.0, 0.0, 0.0)]),
            }],
        };

        let mut anim = SkeletonAnimator::new(skeleton, &clip, PlaybackMode::Once);
        let pose = tick_secs(&mut anim, 0.5);

        // Joint 1 should keep its bind pose position of (1, 0, 0)
        let joint1_pos = pose.local_transforms[1].position;
        assert!(
            (joint1_pos.x - 1.0).abs() < EPSILON,
            "joint1 x: expected 1.0, got {}",
            joint1_pos.x
        );
        assert!(
            (joint1_pos.y).abs() < EPSILON,
            "joint1 y: expected 0.0, got {}",
            joint1_pos.y
        );
        assert!(
            (joint1_pos.z).abs() < EPSILON,
            "joint1 z: expected 0.0, got {}",
            joint1_pos.z
        );

        // Rotation and scale should be identity/unit
        let joint1_rot = pose.local_transforms[1].rotation;
        assert!(
            (joint1_rot.w - 1.0).abs() < EPSILON,
            "joint1 rotation should be identity, got {:?}",
            joint1_rot
        );

        let joint1_scale = pose.local_transforms[1].scale;
        assert!(
            (joint1_scale.x - 1.0).abs() < EPSILON
                && (joint1_scale.y - 1.0).abs() < EPSILON
                && (joint1_scale.z - 1.0).abs() < EPSILON,
            "joint1 scale should be (1,1,1), got {:?}",
            joint1_scale
        );
    }

    #[test]
    fn out_of_range_joint_index_skipped() {
        let skeleton = make_single_bone_skeleton(); // Only 1 joint (index 0)

        let clip = AnimationClip {
            name: "bad_joint".to_string(),
            duration: 1.0,
            channels: vec![AnimationChannel {
                joint: JointId(99), // Out of range
                target: ChannelTarget::Translation,
                timestamps: vec![0.0, 1.0],
                values: ChannelValues::Translation(vec![Vec3::ZERO, Vec3::ONE]),
            }],
        };

        // Should not panic — out-of-range joints are silently skipped
        let mut anim = SkeletonAnimator::new(skeleton, &clip, PlaybackMode::Once);
        anim.tick(0.0);

        // The single bone should have identity transform (bind pose)
        let pos = anim.current_pose().local_transforms[0].position;
        assert!((pos.x).abs() < EPSILON);
        assert!((pos.y).abs() < EPSILON);
        assert!((pos.z).abs() < EPSILON);
    }

    #[test]
    fn skeleton_accessor() {
        let skeleton = make_two_bone_skeleton();
        let clip = AnimationClip {
            name: "test".to_string(),
            duration: 1.0,
            channels: vec![],
        };

        let anim = SkeletonAnimator::new(skeleton, &clip, PlaybackMode::Once);
        assert_eq!(anim.skeleton().joint_count(), 2);
    }

    #[test]
    fn no_channels_immediately_completed() {
        let skeleton = make_single_bone_skeleton();
        let clip = AnimationClip {
            name: "empty".to_string(),
            duration: 1.0,
            channels: vec![],
        };

        let anim = SkeletonAnimator::new(skeleton, &clip, PlaybackMode::Once);
        // No timelines => all "none" => is_completed returns true
        assert!(anim.is_completed());
    }

    #[test]
    fn tick_reaches_final_values() {
        let skeleton = make_single_bone_skeleton();
        let target_pos = Vec3::new(10.0, 20.0, 30.0);
        let clip = AnimationClip {
            name: "reach_end".to_string(),
            duration: 1.0,
            channels: vec![AnimationChannel {
                joint: JointId(0),
                target: ChannelTarget::Translation,
                timestamps: vec![0.0, 1.0],
                values: ChannelValues::Translation(vec![Vec3::ZERO, target_pos]),
            }],
        };

        let mut anim = SkeletonAnimator::new(skeleton, &clip, PlaybackMode::Once);
        let pose = tick_secs(&mut anim, 1.0);

        let pos = pose.local_transforms[0].position;
        assert!(
            (pos.x - target_pos.x).abs() < EPSILON,
            "x: expected {}, got {}",
            target_pos.x,
            pos.x
        );
        assert!(
            (pos.y - target_pos.y).abs() < EPSILON,
            "y: expected {}, got {}",
            target_pos.y,
            pos.y
        );
        assert!(
            (pos.z - target_pos.z).abs() < EPSILON,
            "z: expected {}, got {}",
            target_pos.z,
            pos.z
        );
        assert!(anim.is_completed());
    }
}
