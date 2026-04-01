#!/bin/bash
sed -i 's/apply_chromatic_aberration(black_box(&mut fb), black_box(5));/apply_chromatic_aberration(black_box(\&mut fb), black_box(\&abrash::post_process::ChromaticAberrationConfig { offset: 5 }));/g' benches/chromatic_aberration_bench.rs
sed -i 's/post_process::apply_chromatic_aberration(black_box(&mut fb), black_box(5));/post_process::apply_chromatic_aberration(black_box(\&mut fb), black_box(\&abrash::post_process::ChromaticAberrationConfig { offset: 5 }));/g' benches/post_process.rs
sed -i 's/apply_chromatic_aberration(&mut fb, 5);/apply_chromatic_aberration(\&mut fb, \&abrash::post_process::ChromaticAberrationConfig { offset: 5 });/g' tests/chromatic_aberration.rs
sed -i 's/apply_chromatic_aberration(&mut fb, 2);/apply_chromatic_aberration(\&mut fb, \&abrash::post_process::ChromaticAberrationConfig { offset: 2 });/g' tests/chromatic_aberration.rs
sed -i 's/apply_chromatic_aberration(&mut fb, 100);/apply_chromatic_aberration(\&mut fb, \&abrash::post_process::ChromaticAberrationConfig { offset: 100 });/g' tests/havoc_chromatic.rs
sed -i 's/apply_chromatic_aberration(&mut fb, 1);/apply_chromatic_aberration(\&mut fb, \&abrash_render::post_process::filters::ChromaticAberrationConfig { offset: 1 });/g' crates/abrash-render/src/post_process/filters.rs
