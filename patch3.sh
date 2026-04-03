#!/bin/bash
sed -i 's/pub fn apply_chromatic_aberration(fb: pub fn apply_chromatic_aberration(fb: &mut Framebuffer, offset: u32)mut Framebuffer, config: &ChromaticAberrationConfig) {/pub fn apply_chromatic_aberration(fb: \&mut Framebuffer, config: \&ChromaticAberrationConfig) {/g' crates/abrash-render/src/post_process/filters.rs
