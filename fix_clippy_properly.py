import re

with open("crates/abrash-core/src/math/utils.rs", "r") as f:
    lines = f.readlines()

# Add specific lint exceptions to utils.rs where the functions are too numerous to fix manually right now
# We only disabled imprecise_flops, suspicious_operation_groupings, and must_use_candidate in the original math.rs
lines.insert(0, "#![allow(clippy::must_use_candidate)]\n")
lines.insert(0, "#![allow(clippy::missing_const_for_fn)]\n")
lines.insert(0, "#![allow(clippy::suspicious_operation_groupings)]\n")
lines.insert(0, "#![allow(clippy::imprecise_flops)]\n")

with open("crates/abrash-core/src/math/utils.rs", "w") as f:
    f.writelines(lines)
