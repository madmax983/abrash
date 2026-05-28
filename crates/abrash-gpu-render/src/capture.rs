//! Headless GPU capture target and diagnostic output.

use std::fmt::Write as _;
use std::time::Duration;

/// Compute `bytes_per_row` aligned to wgpu's 256-byte requirement.
///
/// # Panics
///
/// Panics if `width * 4` overflows `u32`.
#[must_use]
pub const fn aligned_bytes_per_row(width: u32) -> u32 {
    let unpadded_bytes_per_row = width
        .checked_mul(4)
        .expect("capture width overflowed RGBA byte count");
    unpadded_bytes_per_row.div_ceil(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT)
        * wgpu::COPY_BYTES_PER_ROW_ALIGNMENT
}

/// Configuration for a capture target.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CaptureConfig {
    pub width: u32,
    pub height: u32,
    pub padded_bytes_per_row: u32,
    pub unpadded_bytes_per_row: u32,
}

impl CaptureConfig {
    /// Create a new capture configuration.
    ///
    /// # Examples
    ///
    /// ```
    /// use abrash_gpu_render::capture::CaptureConfig;
    ///
    /// let config = CaptureConfig::new(320, 240);
    /// assert_eq!(config.width, 320);
    /// assert_eq!(config.height, 240);
    /// ```
    ///
    /// # Panics
    ///
    /// Panics if `width * 4` overflows `u32`.
    #[must_use]
    pub const fn new(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            padded_bytes_per_row: aligned_bytes_per_row(width),
            unpadded_bytes_per_row: width
                .checked_mul(4)
                .expect("capture width overflowed RGBA byte count"),
        }
    }
}

/// Offscreen GPU render target with readback capability.
///
/// # Examples
///
/// ```
/// use abrash_gpu_render::capture::GpuCaptureTarget;
/// use abrash_gpu_render::device::{GpuDevice, GpuDeviceConfig};
///
/// # fn main() {
/// #     if let Ok(device) = GpuDevice::new_headless(&GpuDeviceConfig::headless()) {
/// let target = GpuCaptureTarget::new(device.device(), 320, 240);
/// assert_eq!(target.width(), 320);
/// assert_eq!(target.height(), 240);
/// #     }
/// # }
/// ```
#[allow(dead_code)]
#[derive(Debug)]
pub struct GpuCaptureTarget {
    pub(crate) config: CaptureConfig,
    pub(crate) color_texture: wgpu::Texture,
    pub(crate) color_view: wgpu::TextureView,
    pub(crate) depth_texture: wgpu::Texture,
    pub(crate) depth_view: wgpu::TextureView,
    pub(crate) readback_buffer: wgpu::Buffer,
}

impl GpuCaptureTarget {
    /// Create a new capture target.
    #[must_use]
    pub fn new(device: &wgpu::Device, width: u32, height: u32) -> Self {
        let config = CaptureConfig::new(width, height);

        let color_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Capture Color"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let color_view = color_texture.create_view(&wgpu::TextureViewDescriptor::default());

        let depth_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Capture Depth"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth24Plus,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        let depth_view = depth_texture.create_view(&wgpu::TextureViewDescriptor::default());

        let readback_size = u64::from(config.padded_bytes_per_row) * u64::from(height);
        let readback_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Capture Readback"),
            size: readback_size,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        Self {
            config,
            color_texture,
            color_view,
            depth_texture,
            depth_view,
            readback_buffer,
        }
    }

    /// Width of the capture target.
    #[must_use]
    pub const fn width(&self) -> u32 {
        self.config.width
    }

    /// Height of the capture target.
    #[must_use]
    pub const fn height(&self) -> u32 {
        self.config.height
    }
}

/// Per-batch statistics in a captured frame.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BatchStats {
    /// Batch index in submission order.
    pub index: usize,
    /// Triangle count for this batch.
    pub triangle_count: u32,
    /// Flat color in `0xAARRGGBB` format.
    pub color: u32,
}

/// Frame-level statistics from a GPU capture.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FrameStats {
    /// Resolution width.
    pub width: u32,
    /// Resolution height.
    pub height: u32,
    /// Total batches submitted.
    pub batch_count: usize,
    /// Total triangles across all batches.
    pub total_triangles: u32,
    /// GPU render plus readback time.
    pub render_time: Duration,
    /// Per-batch statistics.
    pub batches: Vec<BatchStats>,
}

/// Result of a headless GPU capture.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GpuDebugCapture {
    /// Frame statistics.
    pub stats: FrameStats,
    /// Raw RGBA pixels in row-major order.
    pub pixels_rgba: Vec<u8>,
    /// Number of non-background pixels.
    pub visible_pixel_count: usize,
}

impl GpuDebugCapture {
    /// Total pixel count.
    #[must_use]
    pub const fn total_pixels(&self) -> usize {
        self.stats.width as usize * self.stats.height as usize
    }

    /// Coverage percentage, in the range `0.0..=100.0`.
    ///
    /// # Examples
    ///
    /// ```
    /// use abrash_gpu_render::capture::{GpuDebugCapture, FrameStats};
    /// use std::time::Duration;
    ///
    /// let capture = GpuDebugCapture {
    ///     stats: FrameStats {
    ///         width: 100,
    ///         height: 100,
    ///         batch_count: 0,
    ///         total_triangles: 0,
    ///         render_time: Duration::ZERO,
    ///         batches: vec![],
    ///     },
    ///     pixels_rgba: vec![],
    ///     visible_pixel_count: 5000,
    /// };
    ///
    /// assert_eq!(capture.coverage_percent(), 50.0);
    /// ```
    #[must_use]
    pub fn coverage_percent(&self) -> f32 {
        let total_pixels = self.total_pixels();
        if total_pixels == 0 {
            return 0.0;
        }

        self.visible_pixel_count as f32 / total_pixels as f32 * 100.0
    }

    /// Compact text dump for LLM/developer consumption.
    #[must_use]
    pub fn to_compact_text(&self) -> String {
        let render_time_ms = self.stats.render_time.as_secs_f64() * 1_000.0;
        let mut output = String::new();
        let _ = writeln!(
            output,
            "FRAME {}x{} {} batches {} tris {:.1}ms",
            self.stats.width,
            self.stats.height,
            self.stats.batch_count,
            self.stats.total_triangles,
            render_time_ms,
        );

        for batch in &self.stats.batches {
            let _ = writeln!(
                output,
                "  B{} {}t color=0x{:08X}",
                batch.index, batch.triangle_count, batch.color,
            );
        }

        let _ = writeln!(
            output,
            "COVERAGE {:.1}% ({}/{})",
            self.coverage_percent(),
            self.visible_pixel_count,
            self.total_pixels(),
        );

        output
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_aligned_bytes_per_row() {
        assert_eq!(aligned_bytes_per_row(320), 1280);
        assert_eq!(aligned_bytes_per_row(100), 512);
        assert_eq!(aligned_bytes_per_row(1), 256);
        assert_eq!(aligned_bytes_per_row(1920), 7680);
    }

    #[test]
    #[should_panic(expected = "capture width overflowed RGBA byte count")]
    fn test_aligned_bytes_per_row_overflow() {
        let _ = aligned_bytes_per_row(u32::MAX);
    }

    #[test]
    fn test_capture_target_dimensions() {
        let config = CaptureConfig::new(320, 240);
        assert_eq!(config.width, 320);
        assert_eq!(config.height, 240);
        assert_eq!(config.padded_bytes_per_row, aligned_bytes_per_row(320));
        assert_eq!(config.unpadded_bytes_per_row, 320 * 4);
    }

    #[test]
    fn test_compact_text_format() {
        let capture = GpuDebugCapture {
            stats: FrameStats {
                width: 320,
                height: 240,
                batch_count: 2,
                total_triangles: 24,
                render_time: Duration::from_micros(1_200),
                batches: vec![
                    BatchStats {
                        index: 0,
                        triangle_count: 12,
                        color: 0xFFFF_4444,
                    },
                    BatchStats {
                        index: 1,
                        triangle_count: 12,
                        color: 0xFF44_44FF,
                    },
                ],
            },
            pixels_rgba: vec![0; 320 * 240 * 4],
            visible_pixel_count: 4_248,
        };

        let text = capture.to_compact_text();
        assert!(text.contains("FRAME 320x240 2 batches 24 tris 1.2ms"));
        assert!(text.contains("B0 12t color=0xFFFF4444"));
        assert!(text.contains("B1 12t color=0xFF4444FF"));
        assert!(text.contains("COVERAGE 5.5%"));
        assert!(text.contains("(4248/76800)"));
    }

    #[test]
    fn test_coverage_percent() {
        let capture = GpuDebugCapture {
            stats: FrameStats {
                width: 100,
                height: 100,
                batch_count: 0,
                total_triangles: 0,
                render_time: Duration::ZERO,
                batches: vec![],
            },
            pixels_rgba: vec![0; 100 * 100 * 4],
            visible_pixel_count: 5_000,
        };

        assert!((capture.coverage_percent() - 50.0).abs() < 0.01);
    }

    #[test]
    #[should_panic(expected = "capture width overflowed RGBA byte count")]
    fn test_capture_target_dimensions_overflow() {
        let _ = CaptureConfig::new(u32::MAX, 240);
    }
}
