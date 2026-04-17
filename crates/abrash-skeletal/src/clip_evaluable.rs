//! Bridge from `AnimationChannel` to abrash-anim `Evaluable`.
//!
//! Converts glTF-style timestamp+value arrays into composable `Sequence` segments.

use abrash_core::math::Vec3;
use abrash_core::quat::Quat;

use abrash_anim::easing::Easing;
use abrash_anim::evaluable::Evaluable;
use abrash_anim::keyframe::Keyframe;
use abrash_anim::sequence::Sequence;

use crate::clip::{AnimationChannel, ChannelValues};

/// Convert a translation or scale channel into a `Sequence<Vec3>`.
///
/// Creates N-1 linear `Keyframe` segments from N timestamp/value pairs.
///
/// # Panics
///
/// Panics if the channel values are not `Translation` or `Scale`.
#[must_use]
pub fn channel_to_vec3_evaluable(channel: &AnimationChannel) -> Evaluable<Vec3> {
    let values = match &channel.values {
        ChannelValues::Translation(v) | ChannelValues::Scale(v) => v,
        ChannelValues::Rotation(_) => panic!("Expected Vec3 channel values (Translation or Scale)"),
    };
    let timestamps = &channel.timestamps;

    let mut segments: Vec<Evaluable<Vec3>> = Vec::with_capacity(timestamps.len() - 1);
    for i in 0..timestamps.len() - 1 {
        let duration = timestamps[i + 1] - timestamps[i];
        segments.push(Evaluable::Keyframe(Keyframe::new(
            values[i],
            values[i + 1],
            Easing::Linear,
            duration,
        )));
    }

    Evaluable::Sequence(Sequence::new(segments))
}

/// Convert a rotation channel into a `Sequence<Quat>`.
///
/// Creates N-1 linear `Keyframe` segments from N timestamp/value pairs.
///
/// # Panics
///
/// Panics if the channel values are not `Rotation`.
#[must_use]
pub fn channel_to_quat_evaluable(channel: &AnimationChannel) -> Evaluable<Quat> {
    let ChannelValues::Rotation(values) = &channel.values else {
        panic!("Expected Quat channel values (Rotation)")
    };
    let timestamps = &channel.timestamps;

    let mut segments: Vec<Evaluable<Quat>> = Vec::with_capacity(timestamps.len() - 1);
    for i in 0..timestamps.len() - 1 {
        let duration = timestamps[i + 1] - timestamps[i];
        segments.push(Evaluable::Keyframe(Keyframe::new(
            values[i],
            values[i + 1],
            Easing::Linear,
            duration,
        )));
    }

    Evaluable::Sequence(Sequence::new(segments))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clip::{AnimationChannel, ChannelTarget, ChannelValues};
    use crate::skeleton::JointId;
    use std::f32::consts::FRAC_PI_2;

    const EPSILON: f32 = 1e-4;

    fn assert_vec3_near(actual: Vec3, expected: Vec3, label: &str) {
        assert!(
            (actual.x - expected.x).abs() < EPSILON
                && (actual.y - expected.y).abs() < EPSILON
                && (actual.z - expected.z).abs() < EPSILON,
            "{label}: expected ({}, {}, {}), got ({}, {}, {})",
            expected.x,
            expected.y,
            expected.z,
            actual.x,
            actual.y,
            actual.z
        );
    }

    #[test]
    fn three_keyframe_translation_channel() {
        // 0s: origin, 1s: (10,0,0), 2s: (10,10,0)
        let channel = AnimationChannel {
            joint: JointId(0),
            target: ChannelTarget::Translation,
            timestamps: vec![0.0, 1.0, 2.0],
            values: ChannelValues::Translation(vec![
                Vec3::ZERO,
                Vec3::new(10.0, 0.0, 0.0),
                Vec3::new(10.0, 10.0, 0.0),
            ]),
        };

        let evaluable = channel_to_vec3_evaluable(&channel);

        // phase 0.0 => start of first segment => origin
        let s0 = evaluable.evaluate(0.0);
        assert_vec3_near(s0.value, Vec3::ZERO, "phase 0.0");

        // phase 0.5 => boundary between segments => (10,0,0)
        // Sequence splits evenly: seg0=[0.0..0.5), seg1=[0.5..1.0]
        // At phase=0.5 we're at the start of seg1, which starts at (10,0,0)
        let s_mid = evaluable.evaluate(0.5);
        assert_vec3_near(s_mid.value, Vec3::new(10.0, 0.0, 0.0), "phase 0.5");

        // phase 1.0 => end of last segment => (10,10,0)
        let s1 = evaluable.evaluate(1.0);
        assert_vec3_near(s1.value, Vec3::new(10.0, 10.0, 0.0), "phase 1.0");
    }

    #[test]
    fn two_keyframe_rotation_channel() {
        // 0s: identity, 1s: 90-degree Y rotation
        let rot90y = Quat::from_axis_angle(Vec3::new(0.0, 1.0, 0.0), FRAC_PI_2);
        let channel = AnimationChannel {
            joint: JointId(0),
            target: ChannelTarget::Rotation,
            timestamps: vec![0.0, 1.0],
            values: ChannelValues::Rotation(vec![Quat::identity(), rot90y]),
        };

        let evaluable = channel_to_quat_evaluable(&channel);

        // At phase 0.5, should be a 45-degree rotation
        let s_mid = evaluable.evaluate(0.5);
        // Rotate (1,0,0) by 45 degrees around Y:
        //   x' = cos(45) ~= 0.7071
        //   z' = -sin(45) ~= -0.7071
        let v = s_mid.value.rotate_vec3(Vec3::new(1.0, 0.0, 0.0));
        let expected_cos = (FRAC_PI_2 / 2.0).cos();
        let expected_sin = (FRAC_PI_2 / 2.0).sin();
        assert!(
            (v.x - expected_cos).abs() < EPSILON,
            "x: expected {expected_cos}, got {}",
            v.x
        );
        assert!(
            (v.z + expected_sin).abs() < EPSILON,
            "z: expected {}, got {}",
            -expected_sin,
            v.z
        );
    }

    #[test]
    fn natural_duration_matches_clip_duration() {
        // Channel from 0s to 3s => natural_duration should be 3.0
        let channel = AnimationChannel {
            joint: JointId(0),
            target: ChannelTarget::Translation,
            timestamps: vec![0.0, 1.0, 3.0],
            values: ChannelValues::Translation(vec![Vec3::ZERO, Vec3::ONE, Vec3::ONE]),
        };

        let evaluable = channel_to_vec3_evaluable(&channel);
        // Total duration = (1.0 - 0.0) + (3.0 - 1.0) = 1.0 + 2.0 = 3.0
        assert!(
            (evaluable.natural_duration() - 3.0).abs() < EPSILON,
            "expected 3.0, got {}",
            evaluable.natural_duration()
        );
    }

    #[test]
    fn scale_channel_converts_to_vec3() {
        let channel = AnimationChannel {
            joint: JointId(0),
            target: ChannelTarget::Scale,
            timestamps: vec![0.0, 1.0],
            values: ChannelValues::Scale(vec![Vec3::ONE, Vec3::new(2.0, 2.0, 2.0)]),
        };

        let evaluable = channel_to_vec3_evaluable(&channel);
        let s_mid = evaluable.evaluate(0.5);
        assert_vec3_near(s_mid.value, Vec3::new(1.5, 1.5, 1.5), "scale midpoint");
    }

    #[test]
    #[should_panic(expected = "Expected Vec3")]
    fn vec3_evaluable_panics_on_rotation_values() {
        let channel = AnimationChannel {
            joint: JointId(0),
            target: ChannelTarget::Rotation,
            timestamps: vec![0.0, 1.0],
            values: ChannelValues::Rotation(vec![Quat::identity(), Quat::identity()]),
        };
        let _ = channel_to_vec3_evaluable(&channel);
    }

    #[test]
    #[should_panic(expected = "Expected Quat")]
    fn quat_evaluable_panics_on_translation_values() {
        let channel = AnimationChannel {
            joint: JointId(0),
            target: ChannelTarget::Translation,
            timestamps: vec![0.0, 1.0],
            values: ChannelValues::Translation(vec![Vec3::ZERO, Vec3::ONE]),
        };
        let _ = channel_to_quat_evaluable(&channel);
    }
}
