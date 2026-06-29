cargo clippy --all-targets --all-features -- -D warnings > clippy.log 2>&1
cat clippy.log | grep "error:"
