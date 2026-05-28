//! Animation clip data — channels of keyframes per joint.

use abrash_core::math::Vec3;
use abrash_core::quat::Quat;

use crate::skeleton::JointId;

/// Typed keyframe values matching the target property.
#[derive(Debug, Clone)]
pub enum ChannelValues {
    /// A sequence of 3D positions (translations).
    Translation(Vec<Vec3>),
    /// A sequence of 3D rotations, represented as Quaternions.
    Rotation(Vec<Quat>),
    /// A sequence of 3D scales.
    Scale(Vec<Vec3>),
}

impl ChannelValues {
    /// Number of values stored.
    #[must_use]
    pub const fn len(&self) -> usize {
        match self {
            Self::Translation(v) | Self::Scale(v) => v.len(),
            Self::Rotation(v) => v.len(),
        }
    }

    /// Whether the values are empty.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// A single animation channel: keyframes for one property of one joint.
#[derive(Debug, Clone)]
pub struct AnimationChannel {
    /// The ID of the joint this channel animates.
    pub joint: JointId,
    /// Timestamps in seconds, sorted ascending.
    pub timestamps: Vec<f32>,
    /// Values at each timestamp (same length as timestamps).
    pub values: ChannelValues,
}

impl AnimationChannel {
    /// Validate this channel's data integrity.
    ///
    /// Checks:
    /// - At least 2 keyframes
    /// - Timestamps length matches values length
    /// - Timestamps are sorted ascending
    ///
    /// # Errors
    ///
    /// Returns a descriptive error string if validation fails.
    pub fn validate(&self) -> Result<(), String> {
        let ts_len = self.timestamps.len();
        let val_len = self.values.len();

        if ts_len < 2 {
            return Err(format!(
                "Channel requires at least 2 keyframes, got {ts_len}"
            ));
        }

        if ts_len != val_len {
            return Err(format!(
                "Timestamps length ({ts_len}) != values length ({val_len})"
            ));
        }

        for i in 1..ts_len {
            if self.timestamps[i] < self.timestamps[i - 1] {
                return Err(format!(
                    "Timestamps not sorted: t[{}]={} < t[{}]={}",
                    i,
                    self.timestamps[i],
                    i - 1,
                    self.timestamps[i - 1]
                ));
            }
        }

        Ok(())
    }
}

/// A complete animation clip containing multiple channels.
///
/// # Examples
///
/// ```
/// use abrash_skeletal::clip::{AnimationClip, AnimationChannel, ChannelValues};
/// use abrash_skeletal::skeleton::JointId;
/// use abrash_core::math::Vec3;
///
/// let clip = AnimationClip {
///     name: "WalkCycle".to_string(),
///     duration: 1.0,
///     channels: vec![
///         AnimationChannel {
///             joint: JointId(0),
///
///             timestamps: vec![0.0, 0.5, 1.0],
///             values: ChannelValues::Translation(vec![
///                 Vec3::new(0.0, 0.0, 0.0),
///                 Vec3::new(0.0, 1.0, 0.0),
///                 Vec3::new(0.0, 0.0, 0.0),
///             ]),
///         }
///     ],
/// };
///
/// assert_eq!(clip.duration, 1.0);
/// assert_eq!(clip.channels.len(), 1);
/// ```
#[derive(Debug, Clone)]
pub struct AnimationClip {
    /// The name of the animation clip.
    pub name: String,
    /// The total duration of the clip in seconds.
    pub duration: f32,
    /// The individual animation channels (e.g., position/rotation tracks for each joint).
    pub channels: Vec<AnimationChannel>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_valid_translation_channel() -> AnimationChannel {
        AnimationChannel {
            joint: JointId(0),
            timestamps: vec![0.0, 1.0, 2.0],
            values: ChannelValues::Translation(vec![
                Vec3::ZERO,
                Vec3::new(10.0, 0.0, 0.0),
                Vec3::new(10.0, 10.0, 0.0),
            ]),
        }
    }

    fn make_valid_rotation_channel() -> AnimationChannel {
        AnimationChannel {
            joint: JointId(1),
            timestamps: vec![0.0, 1.0],
            values: ChannelValues::Rotation(vec![
                Quat::identity(),
                Quat::from_axis_angle(Vec3::new(0.0, 1.0, 0.0), std::f32::consts::FRAC_PI_2),
            ]),
        }
    }

    #[test]
    fn valid_translation_channel_passes() {
        let channel = make_valid_translation_channel();
        assert!(channel.validate().is_ok());
    }

    #[test]
    fn valid_rotation_channel_passes() {
        let channel = make_valid_rotation_channel();
        assert!(channel.validate().is_ok());
    }

    #[test]
    fn valid_scale_channel_passes() {
        let channel = AnimationChannel {
            joint: JointId(0),
            timestamps: vec![0.0, 0.5],
            values: ChannelValues::Scale(vec![Vec3::ONE, Vec3::new(2.0, 2.0, 2.0)]),
        };
        assert!(channel.validate().is_ok());
    }

    #[test]
    fn unsorted_timestamps_fail() {
        let channel = AnimationChannel {
            joint: JointId(0),
            timestamps: vec![0.0, 2.0, 1.0], // out of order
            values: ChannelValues::Translation(vec![Vec3::ZERO, Vec3::ONE, Vec3::ONE]),
        };
        let err = channel.validate().unwrap_err();
        assert!(err.contains("not sorted"), "unexpected error: {err}");
    }

    #[test]
    fn length_mismatch_fails() {
        let channel = AnimationChannel {
            joint: JointId(0),
            timestamps: vec![0.0, 1.0, 2.0],
            values: ChannelValues::Translation(vec![Vec3::ZERO, Vec3::ONE]), // only 2 values
        };
        let err = channel.validate().unwrap_err();
        assert!(err.contains("length"), "unexpected error: {err}");
    }

    #[test]
    fn fewer_than_two_keyframes_fails() {
        let channel = AnimationChannel {
            joint: JointId(0),
            timestamps: vec![0.0],
            values: ChannelValues::Translation(vec![Vec3::ZERO]),
        };
        let err = channel.validate().unwrap_err();
        assert!(err.contains("at least 2"), "unexpected error: {err}");
    }

    #[test]
    fn empty_channel_fails() {
        let channel = AnimationChannel {
            joint: JointId(0),
            timestamps: vec![],
            values: ChannelValues::Translation(vec![]),
        };
        let err = channel.validate().unwrap_err();
        assert!(err.contains("at least 2"), "unexpected error: {err}");
    }

    #[test]
    fn channel_values_len_matches() {
        let vals = ChannelValues::Translation(vec![Vec3::ZERO, Vec3::ONE, Vec3::ONE]);
        assert_eq!(vals.len(), 3);
        assert!(!vals.is_empty());
    }

    #[test]
    fn clip_stores_metadata() {
        let clip = AnimationClip {
            name: "walk".to_string(),
            duration: 2.0,
            channels: vec![make_valid_translation_channel()],
        };
        assert_eq!(clip.name, "walk");
        assert!((clip.duration - 2.0).abs() < f32::EPSILON);
        assert_eq!(clip.channels.len(), 1);
    }
}
