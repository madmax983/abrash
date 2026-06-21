import re

with open("crates/abrash-render/src/experimental/histogram.rs", "r") as f:
    content = f.read()

content = content.replace("pub fn compute_histogram(fb: &Framebuffer) -> [u32; 256] {", "#[must_use]\npub fn compute_histogram(fb: &Framebuffer) -> [u32; 256] {")
content = content.replace("pub fn compute_cdf(hist: &[u32; 256]) -> [u32; 256] {", "#[must_use]\npub fn compute_cdf(hist: &[u32; 256]) -> [u32; 256] {")
content = content.replace("for &val in cdf.iter() {", "for &val in &cdf {")
content = content.replace("let new_r = mapping[r] as u32;", "let new_r = u32::from(mapping[r]);")
content = content.replace("let new_g = mapping[g] as u32;", "let new_g = u32::from(mapping[g]);")
content = content.replace("let new_b = mapping[b] as u32;", "let new_b = u32::from(mapping[b]);")

with open("crates/abrash-render/src/experimental/histogram.rs", "w") as f:
    f.write(content)
