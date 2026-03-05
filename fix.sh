#!/bin/bash
cargo clippy --fix --allow-dirty --allow-no-vcs --all-targets --no-default-features --features "backend-tui nova parallel"
