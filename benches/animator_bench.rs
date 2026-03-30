use abrash_anim::PlaybackMode;
use abrash_core::math::{Mat4, Vec3};
use abrash_core::transform::Transform;
use abrash_skeletal::animator::SkeletonAnimator;
use abrash_skeletal::clip::{AnimationChannel, AnimationClip, ChannelTarget, ChannelValues};
use abrash_skeletal::skeleton::{Joint, Skeleton};
use criterion::{Criterion, black_box, criterion_group, criterion_main};

fn bench_animator(c: &mut Criterion) {
    let mut joints = Vec::new();
    let num_joints = 100;

    // Create chain
    for i in 0..num_joints {
        joints.push(Joint {
            name: format!("joint_{}", i),
            parent: if i == 0 {
                None
            } else {
                Some(abrash_skeletal::skeleton::JointId(i - 1))
            },
            inverse_bind_matrix: Mat4::identity(),
            bind_transform: Transform::identity(),
        });
    }

    let skeleton = Skeleton::new(joints);

    let mut channels = Vec::new();
    for i in 0..num_joints {
        channels.push(AnimationChannel {
            joint: abrash_skeletal::skeleton::JointId(i),
            target: ChannelTarget::Translation,
            timestamps: vec![0.0, 1.0, 2.0],
            values: ChannelValues::Translation(vec![
                Vec3::new(0.0, 0.0, 0.0),
                Vec3::new(1.0, 1.0, 1.0),
                Vec3::new(2.0, 2.0, 2.0),
            ]),
        });
    }

    let clip = AnimationClip {
        name: "test".to_string(),
        channels,
        duration: 2.0,
    };
    let mut animator = SkeletonAnimator::new(skeleton, &clip, PlaybackMode::Loop);

    c.bench_function("animator_tick", |b| {
        b.iter(|| animator.tick(black_box(0.016)))
    });
}

criterion_group!(benches, bench_animator);
criterion_main!(benches);
