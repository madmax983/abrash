#!/bin/bash
sed -i 's/        `apply_solarize(&mut fb, 127)`;/        apply_solarize(\&mut fb, 127);/g' crates/abrash-render/src/post_process/filters.rs
sed -i 's/        let mut fb = `Framebuffer::new(1, 1).unwrap()`;/        let mut fb = Framebuffer::new(1, 1).unwrap();/g' crates/abrash-render/src/post_process/filters.rs
