#!/bin/bash
sed -i 's/0xFFFFFFFF/0xFFFF_FFFF/g' crates/abrash-render/src/experimental/speed_lines.rs
sed -i 's/747796405/747_796_405/g' crates/abrash-render/src/experimental/speed_lines.rs
sed -i 's/283927953/283_927_953/g' crates/abrash-render/src/experimental/speed_lines.rs
sed -i 's/0xFF000000/0xFF00_0000/g' crates/abrash-render/src/experimental/speed_lines.rs
sed -i 's/0xDEADBEEF/0xDEAD_BEEF/g' crates/abrash-render/src/experimental/digital_rain.rs
sed -i 's/1000000/1_000_000/g' crates/abrash-render/src/experimental/frosted_glass.rs
