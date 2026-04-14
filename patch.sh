#!/bin/bash
sed -i '/pub fn apply_chromatic_aberration/i \
/// Configuration for the chromatic aberration filter.\n#[derive(Debug, Clone, PartialEq)]\npub struct ChromaticAberrationConfig {\n    pub offset: u32,\n}\n' crates/abrash-render/src/post_process/filters.rs
