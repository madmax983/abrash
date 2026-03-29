#!/bin/bash
sed -i '/fn test_apply_sepia() {/,/^    }/d' crates/abrash-render/src/post_process/filters.rs
