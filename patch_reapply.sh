#!/bin/bash
sed -i '/pub fn apply_chromatic_aberration/i \
/// Configuration for the chromatic aberration filter.\n#[derive(Debug, Clone, PartialEq, Eq)]\npub struct ChromaticAberrationConfig {\n    pub offset: u32,\n}\n' crates/abrash-render/src/post_process/filters.rs
sed -i 's/pub fn apply_chromatic_aberration(fb: &mut Framebuffer, offset: u32)/pub fn apply_chromatic_aberration(fb: \&mut Framebuffer, config: \&ChromaticAberrationConfig)/g' crates/abrash-render/src/post_process/filters.rs
sed -i 's/if offset == 0/if config.offset == 0/g' crates/abrash-render/src/post_process/filters.rs
sed -i 's/let offset = offset as usize;/let offset = config.offset as usize;/g' crates/abrash-render/src/post_process/filters.rs
sed -i 's/apply_chromatic_aberration(&mut fb, 1);/apply_chromatic_aberration(\&mut fb, \&abrash_render::post_process::filters::ChromaticAberrationConfig { offset: 1 });/g' crates/abrash-render/src/post_process/filters.rs
