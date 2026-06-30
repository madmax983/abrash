import re

with open("crates/abrash-core/src/math/quat.rs", "r") as f:
    quat_code = f.read()

# Restore ALL of the Quat code from old_quat.rs that was missing, including `DualQuat`, `from_mat4`, `to_mat4` etc.
with open("old_quat.rs", "r") as f:
    old_quat_code = f.read()

# I will just write a cleaner version that merges them correctly.
# The user wants DualQuat to be moved as well.
# It seems when I moved `quat.rs` to `math/quat.rs` I stripped out too much!
# So I should copy `old_quat.rs` completely over to `math/quat.rs` and apply the necessary fixes.
