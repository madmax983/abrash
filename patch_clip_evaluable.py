import sys

filepath = "crates/abrash-skeletal/src/clip_evaluable.rs"
with open(filepath, "r") as f:
    content = f.read()

# Fix doc string panics
content = content.replace("/// # Panics", "/// # Errors")
content = content.replace("Panics if the channel values are not `Translation` or `Scale`.", "Returns an error if the channel values are not `Translation` or `Scale`.")
content = content.replace("Panics if the channel values are not `Rotation`.", "Returns an error if the channel values are not `Rotation`.")


content = content.replace(
    'ChannelValues::Rotation(_) => panic!("Expected Vec3 channel values (Translation or Scale)"),',
    'ChannelValues::Rotation(_) => return Err("Expected Vec3 channel values (Translation or Scale)"),'
)

content = content.replace(
    "pub fn channel_to_vec3_evaluable(channel: &AnimationChannel) -> Evaluable<Vec3> {",
    "pub fn channel_to_vec3_evaluable(channel: &AnimationChannel) -> Result<Evaluable<Vec3>, &'static str> {"
)

content = content.replace(
    "    Evaluable::Sequence(Sequence::new(segments))\n}",
    "    Ok(Evaluable::Sequence(Sequence::new(segments)))\n}",
    1 # Only the first one in channel_to_vec3_evaluable
)

content = content.replace(
    'let ChannelValues::Rotation(values) = &channel.values else {\n        panic!("Expected Quat channel values (Rotation)")\n    };',
    'let ChannelValues::Rotation(values) = &channel.values else {\n        return Err("Expected Quat channel values (Rotation)")\n    };'
)

content = content.replace(
    "pub fn channel_to_quat_evaluable(channel: &AnimationChannel) -> Evaluable<Quat> {",
    "pub fn channel_to_quat_evaluable(channel: &AnimationChannel) -> Result<Evaluable<Quat>, &'static str> {"
)

content = content.replace(
    "    Evaluable::Sequence(Sequence::new(segments))\n}",
    "    Ok(Evaluable::Sequence(Sequence::new(segments)))\n}"
)

# Fix tests
content = content.replace("let evaluable = channel_to_vec3_evaluable(&channel);", "let evaluable = channel_to_vec3_evaluable(&channel).unwrap();")
content = content.replace("let evaluable = channel_to_quat_evaluable(&channel);", "let evaluable = channel_to_quat_evaluable(&channel).unwrap();")

content = content.replace('#[should_panic(expected = "Expected Vec3")]', "")
content = content.replace('#[should_panic(expected = "Expected Quat")]', "")

content = content.replace("let _ = channel_to_vec3_evaluable(&channel);", "assert!(channel_to_vec3_evaluable(&channel).is_err());")
content = content.replace("let _ = channel_to_quat_evaluable(&channel);", "assert!(channel_to_quat_evaluable(&channel).is_err());")

with open(filepath, "w") as f:
    f.write(content)
