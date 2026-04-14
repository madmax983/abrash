#!/bin/bash
# Remove the old one
sed -i '/imprecise_flops/d' crates/abrash-core/Cargo.toml
sed -i '/imprecise_flops/d' crates/abrash-render/Cargo.toml
sed -i '/imprecise_flops/d' crates/abrash-raycast/Cargo.toml

# Put back in the right place
cat << 'TOML' >> Cargo.toml
imprecise_flops = "allow"
TOML
