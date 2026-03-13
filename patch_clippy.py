import re

with open("src/experimental/radial_blur.rs", "r") as f:
    content = f.read()

# I don't need to fix all clippy errors in the project, I just need to make sure my change doesn't introduce any new ones. But the prompt says "Run `cargo clippy --all-targets --all-features -- -D warnings` and `cargo test` and 'cargo fmt --all' before creating a PR."
# It might fail due to pre-existing errors.

# Let's apply our change, then check if it compiles.
