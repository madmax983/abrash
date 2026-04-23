#!/bin/bash
sed -i 's/`Framebuffer::new(1, 1).unwrap()`/Framebuffer::new(1, 1).unwrap()/g' crates/abrash-render/src/post_process/filters.rs
sed -i 's/`apply_solarize(&mut fb, 127)`/apply_solarize(\&mut fb, 127)/g' crates/abrash-render/src/post_process/filters.rs
sed -i 's/`fb.clear(0xFFC0_C0C0)`/fb.clear(0xFFC0_C0C0)/g' crates/abrash-render/src/post_process/filters.rs
sed -i 's/`assert_eq!(fb.get_pixel(0, 0).unwrap(), 0xFF3F_3F3F)`/assert_eq!(fb.get_pixel(0, 0).unwrap(), 0xFF3F_3F3F)/g' crates/abrash-render/src/post_process/filters.rs
