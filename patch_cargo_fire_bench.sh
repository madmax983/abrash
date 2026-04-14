cat << 'INNER_EOF' >> Cargo.toml

[[bench]]
name = "fire_bench"
harness = false
required-features = ["nova"]
INNER_EOF
cargo bench --bench fire_bench --features nova
