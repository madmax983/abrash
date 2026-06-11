//! GPU device, adapter, and queue factory.

#[cfg(feature = "windowed")]
use crate::surface::GpuSurface;
#[cfg(feature = "windowed")]
use std::sync::Arc;
#[cfg(feature = "windowed")]
use winit::window::Window;

/// Configuration for GPU device creation.
#[derive(Debug, Clone)]
pub struct GpuDeviceConfig {
    /// GPU power preference.
    pub power_preference: wgpu::PowerPreference,
    /// Force a software fallback adapter.
    pub force_fallback: bool,
}

impl Default for GpuDeviceConfig {
    fn default() -> Self {
        Self {
            power_preference: wgpu::PowerPreference::HighPerformance,
            force_fallback: false,
        }
    }
}

impl GpuDeviceConfig {
    /// Configuration for headless rendering.
    #[must_use]
    pub const fn headless() -> Self {
        Self {
            power_preference: wgpu::PowerPreference::HighPerformance,
            force_fallback: false,
        }
    }
}

/// Owns the wgpu instance, adapter, device, and queue used by the renderer.
pub struct GpuDevice {
    instance: wgpu::Instance,
    adapter: wgpu::Adapter,
    device: wgpu::Device,
    queue: wgpu::Queue,
}

impl GpuDevice {
    /// Create a headless device without a presentation surface.
    ///
    /// # Errors
    ///
    /// Returns an error if no compatible adapter or device is available.
    pub fn new_headless(config: &GpuDeviceConfig) -> Result<Self, String> {
        Self::new_with_instance(config, wgpu::Instance::default(), None)
    }

    /// Create a device using a caller-provided instance and optional compatible surface.
    ///
    /// # Errors
    ///
    /// Returns an error if no compatible adapter or device is available.
    pub fn new_with_instance(
        config: &GpuDeviceConfig,
        instance: wgpu::Instance,
        compatible_surface: Option<&wgpu::Surface<'_>>,
    ) -> Result<Self, String> {
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: config.power_preference,
            compatible_surface,
            force_fallback_adapter: config.force_fallback,
        }))
        .map_err(|e| format!("No suitable GPU adapter found: {e}"))?;

        // Request RT features when ray-tracing feature is enabled and hardware supports it
        #[allow(unused_mut)]
        let mut features = wgpu::Features::empty();

        let blendable_feature = wgpu::Features::FLOAT32_BLENDABLE;
        if adapter.features().contains(blendable_feature) {
            features |= blendable_feature;
        }

        #[cfg(feature = "ray-tracing")]
        {
            let rt_feature = wgpu::Features::EXPERIMENTAL_RAY_QUERY;
            if adapter.features().contains(rt_feature) {
                features |= rt_feature;
            }
        }

        let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
            label: Some("Abrash GpuDevice"),
            required_features: features,
            required_limits: wgpu::Limits::default(),
            memory_hints: wgpu::MemoryHints::default(),

            experimental_features: wgpu::ExperimentalFeatures::default(),
            trace: wgpu::Trace::Off,
        }))
        .map_err(|error| format!("Failed to create GPU device: {error}"))?;

        Ok(Self {
            instance,
            adapter,
            device,
            queue,
        })
    }

    /// Create a device and configured presentation surface for a window.
    ///
    /// # Errors
    ///
    /// Returns an error if the surface, adapter, or device cannot be created.
    #[cfg(feature = "windowed")]
    pub fn new_windowed(
        window: Arc<Window>,
        config: &GpuDeviceConfig,
    ) -> Result<(Self, GpuSurface), String> {
        let size = window.inner_size();
        let instance = wgpu::Instance::default();
        let surface = instance
            .create_surface(window)
            .map_err(|error| format!("Failed to create surface: {error}"))?;
        let gpu = Self::new_with_instance(config, instance, Some(&surface))?;
        let surface = GpuSurface::from_parts(
            surface,
            gpu.adapter(),
            gpu.device(),
            size.width,
            size.height,
        )?;

        Ok((gpu, surface))
    }

    /// Access the underlying instance.
    #[must_use]
    pub const fn instance(&self) -> &wgpu::Instance {
        &self.instance
    }

    /// Access the underlying adapter.
    #[must_use]
    pub const fn adapter(&self) -> &wgpu::Adapter {
        &self.adapter
    }

    /// Access the underlying device.
    #[must_use]
    pub const fn device(&self) -> &wgpu::Device {
        &self.device
    }

    /// Access the underlying queue.
    #[must_use]
    pub const fn queue(&self) -> &wgpu::Queue {
        &self.queue
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gpu_device_config_default() {
        let config = GpuDeviceConfig::default();
        assert_eq!(
            config.power_preference,
            wgpu::PowerPreference::HighPerformance
        );
        assert!(!config.force_fallback);
    }

    #[test]
    fn test_gpu_device_config_headless() {
        let config = GpuDeviceConfig::headless();
        assert_eq!(
            config.power_preference,
            wgpu::PowerPreference::HighPerformance
        );
        assert!(!config.force_fallback);
    }
}
