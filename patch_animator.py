import sys

filepath = "crates/abrash-skeletal/src/animator.rs"
with open(filepath, "r") as f:
    content = f.read()

content = content.replace("let evaluable = channel_to_vec3_evaluable(channel);", "let evaluable = channel_to_vec3_evaluable(channel).expect(\"Valid translation or scale channel\");")
content = content.replace("let evaluable = channel_to_quat_evaluable(channel);", "let evaluable = channel_to_quat_evaluable(channel).expect(\"Valid rotation channel\");")

with open(filepath, "w") as f:
    f.write(content)
