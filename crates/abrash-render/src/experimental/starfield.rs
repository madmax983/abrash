//! Starfield Effect
//!
//! A retro post-processing effect that renders a 3D flying starfield.

#![cfg(feature = "nova")]

use abrash_core::framebuffer::Framebuffer;
use abrash_core::math::Vec3;
use abrash_core::utils::XorShift32;

/// A classic 3D starfield simulator.
pub struct Starfield {
    /// Positions of the stars in 3D space.
    pub stars: Vec<Vec3>,
    /// The forward movement speed through the starfield.
    pub speed: f32,
    /// Max depth for stars before respawning.
    pub max_depth: f32,
    prng: XorShift32,
}

impl Starfield {
    /// Creates a new starfield with a specific number of stars.
    #[must_use]
    pub fn new(num_stars: usize, speed: f32, max_depth: f32) -> Self {
        let mut prng = XorShift32::new(12345);
        let mut stars = Vec::with_capacity(num_stars);

        for _ in 0..num_stars {
            // Random positions: x and y between -1.0 and 1.0, z between 0.1 and max_depth
            let x = (prng.next_f32() * 2.0) - 1.0;
            let y = (prng.next_f32() * 2.0) - 1.0;
            let z = 0.1 + (prng.next_f32() * (max_depth - 0.1));
            stars.push(Vec3::new(x, y, z));
        }

        Self {
            stars,
            speed,
            max_depth,
            prng,
        }
    }

    /// Updates the star positions.
    pub fn update(&mut self, delta_time: f32) {
        let movement = self.speed * delta_time;

        for star in &mut self.stars {
            star.z -= movement;

            // If a star moves past the camera (z <= 0.0), recycle it
            if star.z <= 0.0 {
                star.x = (self.prng.next_f32() * 2.0) - 1.0;
                star.y = (self.prng.next_f32() * 2.0) - 1.0;
                star.z = self.max_depth;
            }
        }
    }

    /// Renders the starfield onto a framebuffer.
    pub fn render(&self, fb: &mut Framebuffer) {
        let width = fb.width() as f32;
        let height = fb.height() as f32;
        let half_w = width * 0.5;
        let half_h = height * 0.5;
        let fov_scale = half_w; // Simple perspective multiplier

        for star in &self.stars {
            if star.z <= 0.0 {
                continue;
            }

            // Simple perspective projection
            let sx = (star.x / star.z) * fov_scale + half_w;
            let sy = (star.y / star.z) * fov_scale + half_h;

            if sx >= 0.0 && sx < width && sy >= 0.0 && sy < height {
                // Dim the star based on depth
                let depth_ratio = (star.z / self.max_depth).clamp(0.0, 1.0);
                // Inverse so closer is brighter
                let intensity = 1.0 - depth_ratio;

                // Using non-linear falloff for a better look
                let luma = (intensity * intensity * 255.0) as u32;
                if luma > 0 {
                    let color = 0xFF00_0000 | (luma << 16) | (luma << 8) | luma;
                    fb.set_pixel(sx as i32, sy as i32, color);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_starfield_initialization() {
        let starfield = Starfield::new(100, 10.0, 100.0);
        assert_eq!(starfield.stars.len(), 100);
        assert!((starfield.speed - 10.0).abs() < 1e-6);
        assert!((starfield.max_depth - 100.0).abs() < 1e-6);
    }

    #[test]
    fn test_starfield_update() {
        let mut starfield = Starfield::new(10, 10.0, 100.0);
        starfield.stars = vec![Vec3::new(0.0, 0.0, 50.0)];
        starfield.update(1.0);
        assert!((starfield.stars[0].z - 40.0).abs() < 1e-6);
    }

    #[test]
    fn test_starfield_render() {
        let mut fb = Framebuffer::new(100, 100).unwrap();
        fb.clear(0xFF00_0000); // Black

        let mut starfield = Starfield::new(10, 10.0, 100.0);
        // Put a star straight ahead, fairly close
        starfield.stars = vec![Vec3::new(0.0, 0.0, 5.0)];
        starfield.render(&mut fb);

        // Center pixel should not be black
        assert_ne!(fb.get_pixel(50, 50).unwrap(), 0xFF00_0000);
    }
}
