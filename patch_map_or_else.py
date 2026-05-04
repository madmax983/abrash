with open("crates/abrash-gpu-render/src/renderer.rs", "r") as f:
    code = f.read()

old_code = """        let owned_taa_view;
        let view: &wgpu::TextureView = refraction_override.map_or_else(
            || {
                if !matches!(
                    self.composition_pass.debug_mode,
                    crate::composition::DebugMode::None
                ) || self.taa_enabled
                {
                    owned_taa_view = self
                        .taa_pass
                        .output_texture
                        .as_ref()
                        .map(|t| t.create_view(&wgpu::TextureViewDescriptor::default()));
                    owned_taa_view.as_ref().unwrap_or(&hdr.color_view)
                } else {
                    &hdr.color_view
                }
            },
            |v| v,
        );"""

new_code = """        let mut owned_taa_view = None;
        let view: &wgpu::TextureView = refraction_override.map_or_else(
            || {
                if !matches!(
                    self.composition_pass.debug_mode,
                    crate::composition::DebugMode::None
                ) || self.taa_enabled
                {
                    owned_taa_view = self
                        .taa_pass
                        .output_texture
                        .as_ref()
                        .map(|t| t.create_view(&wgpu::TextureViewDescriptor::default()));
                    owned_taa_view.as_ref().unwrap_or(&hdr.color_view)
                } else {
                    &hdr.color_view
                }
            },
            |v| v,
        );"""

code = code.replace(old_code, new_code)

with open("crates/abrash-gpu-render/src/renderer.rs", "w") as f:
    f.write(code)
