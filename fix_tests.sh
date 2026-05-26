sed -i '1i #![cfg(all(feature = "nova", feature = "parallel"))]' crates/abrash-render/tests/havoc_tile_ub_test.rs
sed -i '1i #![cfg(all(feature = "nova", feature = "parallel"))]' crates/abrash-render/tests/havoc_pixel_sort_proptest.rs
sed -i '1i #![cfg(all(feature = "nova", feature = "parallel"))]' crates/abrash-render/tests/havoc_radial_blur_fuzz.rs
