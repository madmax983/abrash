#!/bin/bash

# Find files with strict float comparison warnings in tests and replace with assert_approx_eq or similar
sed -i 's/assert_eq!(a, b);/assert!((a - b).abs() < 1e-4);/' crates/abrash-core/src/noise.rs
sed -i 's/assert!(c.a == 1.0);/assert!((c.a - 1.0).abs() < 1e-4);/' crates/abrash-core/src/color.rs

# Also address redundant clone
sed -i 's/let mut actual = vectors.clone();/let mut actual = vectors;/' crates/abrash-core/src/transform.rs
sed -i 's/let mut actual = points.clone();/let mut actual = points;/' crates/abrash-core/src/transform.rs
