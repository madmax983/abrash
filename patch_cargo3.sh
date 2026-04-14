cat << 'INNER_EOF' > patch_cargo3.diff
--- Cargo.toml
+++ Cargo.toml
@@ -668,7 +668,6 @@ required-features = ["nova"]
 [[example]]
 name = "frosted_glass_demo"
 path = "examples/frosted_glass_demo.rs"
 required-features = ["nova"]
-imprecise_flops = "allow"

 [[example]]
 name = "steganography_demo"
@@ -695,3 +694,8 @@ required-features = ["nova"]
 name = "fire_bench"
 harness = false
 required-features = ["nova"]
+
+[[example]]
+name = "fire_demo"
+path = "examples/fire_demo.rs"
+required-features = ["nova"]
INNER_EOF
patch Cargo.toml < patch_cargo3.diff
