#!/bin/bash
cargo test -p abrash --features parallel,nova
cargo test -p abrash-core --features parallel,nova
cargo test -p abrash-render --features parallel,nova
cargo test -p abrash-gpu-render
cargo test -p abrash-anim
cargo test -p abrash-skeletal
cargo test -p embed-demo
