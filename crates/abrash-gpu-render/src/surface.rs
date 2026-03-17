//! Windowed display surface (feature-gated behind `windowed`).

/// Windowed display surface backed by a wgpu swapchain.
pub struct GpuSurface {
    pub(crate) surface: wgpu::Surface<'static>,
    pub(crate) config: wgpu::SurfaceConfiguration,
    pub(crate) depth_texture: wgpu::Texture,
    pub(crate) depth_view: wgpu::TextureView,
    pub(crate) width: u32,
    pub(crate) height: u32,
}

impl GpuSurface {
    /// Build a configured surface from already-created wgpu primitives.
    ///
    /// # Errors
    ///
    /// Returns an error if the surface capabilities are missing formats or the size is zero.
    pub(crate) fn from_parts(
        surface: wgpu::Surface<'static>,
        adapter: &wgpu::Adapter,
        device: &wgpu::Device,
        width: u32,
        height: u32,
    ) -> Result<Self, String> {
        if width == 0 || height == 0 {
            return Err("window size must be non-zero".to_string());
        }

        let capabilities = surface.get_capabilities(adapter);
        let format = capabilities
            .formats
            .iter()
            .copied()
            .find(wgpu::TextureFormat::is_srgb)
            .or_else(|| capabilities.formats.first().copied())
            .ok_or_else(|| "surface reported no supported formats".to_string())?;
        let present_mode = capabilities
            .present_modes
            .iter()
            .copied()
            .find(|mode| *mode == wgpu::PresentMode::Fifo)
            .or_else(|| capabilities.present_modes.first().copied())
            .ok_or_else(|| "surface reported no supported present modes".to_string())?;
        let alpha_mode = capabilities
            .alpha_modes
            .first()
            .copied()
            .ok_or_else(|| "surface reported no supported alpha modes".to_string())?;

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            width,
            height,
            present_mode,
            desired_maximum_frame_latency: 2,
            alpha_mode,
            view_formats: vec![],
        };
        surface.configure(device, &config);

        let (depth_texture, depth_view) = Self::create_depth_resources(device, width, height);

        Ok(Self {
            surface,
            config,
            depth_texture,
            depth_view,
            width,
            height,
        })
    }

    /// Resize the swapchain and depth buffer.
    pub fn resize(&mut self, device: &wgpu::Device, width: u32, height: u32) {
        if width == 0 || height == 0 {
            return;
        }

        self.width = width;
        self.height = height;
        self.config.width = width;
        self.config.height = height;
        self.surface.configure(device, &self.config);
        let (depth_texture, depth_view) = Self::create_depth_resources(device, width, height);
        self.depth_texture = depth_texture;
        self.depth_view = depth_view;
    }

    /// Surface format used for presentation.
    #[must_use]
    pub fn format(&self) -> wgpu::TextureFormat {
        self.config.format
    }

    fn create_depth_resources(
        device: &wgpu::Device,
        width: u32,
        height: u32,
    ) -> (wgpu::Texture, wgpu::TextureView) {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("GpuSurface Depth"),
            size: wgpu::Extent3d {
                width: width.max(1),
                height: height.max(1),
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth24Plus,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        (texture, view)
    }
}
