#!/bin/bash
sed -i 's/apply_chromatic_aberration(&mut self.framebuffer, shift);/apply_chromatic_aberration(\&mut self.framebuffer, \&abrash::post_process::ChromaticAberrationConfig { offset: shift });/g' examples/chromatic_aberration_demo.rs
