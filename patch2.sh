#!/bin/bash
sed -i 's/pub fn apply_chromatic_aberration(fb: &mut Framebuffer, offset: u32)/pub fn apply_chromatic_aberration(fb: &mut Framebuffer, config: \&ChromaticAberrationConfig)/g' crates/abrash-render/src/post_process/filters.rs
sed -i 's/if offset == 0/if config.offset == 0/g' crates/abrash-render/src/post_process/filters.rs
sed -i 's/let offset = offset as usize;/let offset = config.offset as usize;/g' crates/abrash-render/src/post_process/filters.rs
