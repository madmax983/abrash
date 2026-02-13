use abrash::framebuffer::Framebuffer;
use abrash::math::{Mat4, Vec2, Vec3};
use abrash::platform::{Window, WindowBackend};
use abrash::rasterizer::fill_triangle_textured;
use abrash::texture::Texture;
use abrash::zbuffer::ZBuffer;
use std::f32::consts::PI;
use std::time::Instant;

const WIDTH: u32 = 800;
const HEIGHT: u32 = 600;

// Simple fixed timestep implementation for example
struct FixedTimestep {
    target_dt: f32,
    accumulator: f32,
    last_time: Instant,
}

impl FixedTimestep {
    fn new(target_fps: u32) -> Self {
        Self {
            target_dt: 1.0 / target_fps as f32,
            accumulator: 0.0,
            last_time: Instant::now(),
        }
    }

    fn update(&mut self) -> u32 {
        let now = Instant::now();
        let frame_time = now.duration_since(self.last_time).as_secs_f32();
        self.last_time = now;

        self.accumulator += frame_time;
        let mut steps = 0;
        while self.accumulator >= self.target_dt {
            self.accumulator -= self.target_dt;
            steps += 1;
        }
        steps
    }

    fn dt(&self) -> f32 {
        self.target_dt
    }
}

// --- Particle System (Moved from experimental) ---

/// A single particle in the system.
#[derive(Clone, Copy, Debug)]
pub struct Particle {
    pub position: Vec3,
    pub velocity: Vec3,
    pub life: f32,
    pub max_life: f32,
    pub size: f32,
    pub color: u32,
}

impl Particle {
    pub fn new(position: Vec3, velocity: Vec3, life: f32, size: f32, color: u32) -> Self {
        Self {
            position,
            velocity,
            life,
            max_life: life,
            size,
            color,
        }
    }
}

/// A particle emitter and manager.
pub struct ParticleSystem {
    pub particles: Vec<Particle>,
    pub position: Vec3,
    pub emission_rate: f32, // Particles per second
    pub gravity: Vec3,
    pub texture: Texture,

    // Emitter properties
    pub start_speed: f32,
    pub start_life: f32,
    pub start_size: f32,
    pub spread: f32,

    // Internal state
    emission_accumulator: f32,
    rng_state: u32,
}

impl ParticleSystem {
    /// Creates a new particle system.
    ///
    /// # Arguments
    /// * `max_particles` - Initial capacity.
    /// * `texture` - The texture to use for particles.
    pub fn new(max_particles: usize, texture: Texture) -> Self {
        Self {
            particles: Vec::with_capacity(max_particles),
            position: Vec3::new(0.0, 0.0, 0.0),
            emission_rate: 10.0,
            gravity: Vec3::new(0.0, -9.8, 0.0),
            texture,
            start_speed: 1.0,
            start_life: 1.0,
            start_size: 0.1,
            spread: 0.5,
            emission_accumulator: 0.0,
            rng_state: 12345,
        }
    }

    /// Simple XorShift RNG
    fn rand_float(&mut self) -> f32 {
        let mut x = self.rng_state;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.rng_state = x;
        (x as f32) / (u32::MAX as f32)
    }

    /// Returns a random float between -1.0 and 1.0
    fn rand_signed(&mut self) -> f32 {
        self.rand_float() * 2.0 - 1.0
    }

    /// Updates the particle system.
    ///
    /// * `dt` - Delta time in seconds.
    pub fn update(&mut self, dt: f32) {
        // Emit new particles
        self.emission_accumulator += dt * self.emission_rate;
        while self.emission_accumulator >= 1.0 {
            self.emit();
            self.emission_accumulator -= 1.0;
        }

        // Update existing particles
        let mut i = 0;
        while i < self.particles.len() {
            let p = &mut self.particles[i];

            p.life -= dt;
            if p.life <= 0.0 {
                // Remove dead particle (swap remove is O(1))
                self.particles.swap_remove(i);
                // Don't increment i, as the swapped element needs to be checked
                continue;
            }

            // Physics
            p.velocity = p.velocity + self.gravity * dt;
            p.position = p.position + p.velocity * dt;

            i += 1;
        }
    }

    fn emit(&mut self) {
        let vel = Vec3::new(
            self.rand_signed() * self.spread,
            1.0 + self.rand_signed() * self.spread, // Generally upwards
            self.rand_signed() * self.spread,
        ).normalize() * self.start_speed;

        let p = Particle::new(
            self.position,
            vel,
            self.start_life,
            self.start_size,
            0xFFFFFFFF,
        );
        self.particles.push(p);
    }

    /// Renders the particles as billboards.
    pub fn render(
        &self,
        fb: &mut Framebuffer,
        zb: &mut ZBuffer,
        view: Mat4,
        proj: Mat4,
    ) {
        // Extract camera Right and Up vectors from View Matrix.
        // The View Matrix transforms World to Camera space.
        // Row 0 is the Right vector (Side)
        // Row 1 is the Up vector
        // (Assuming standard LookAt construction without scaling)
        let right = Vec3::new(view.m[0][0], view.m[1][0], view.m[2][0]);
        let up = Vec3::new(view.m[0][1], view.m[1][1], view.m[2][1]);

        let mvp = proj * view;

        for p in &self.particles {
            let half_size = p.size * 0.5;

            // Billboard corners in World Space
            // v0: Bottom-Left
            let v0_pos = p.position + (right * -half_size) + (up * -half_size);
            // v1: Top-Left
            let v1_pos = p.position + (right * -half_size) + (up * half_size);
            // v2: Top-Right
            let v2_pos = p.position + (right * half_size) + (up * half_size);
            // v3: Bottom-Right
            let v3_pos = p.position + (right * half_size) + (up * -half_size);

            // Transform to Clip Space
            let (c0, w0) = mvp.transform_point(v0_pos);
            let (c1, w1) = mvp.transform_point(v1_pos);
            let (c2, w2) = mvp.transform_point(v2_pos);
            let (c3, w3) = mvp.transform_point(v3_pos);

            // UVs
            let uv0 = Vec2::new(0.0, 1.0); // BL
            let uv1 = Vec2::new(0.0, 0.0); // TL
            let uv2 = Vec2::new(1.0, 0.0); // TR
            let uv3 = Vec2::new(1.0, 1.0); // BR

            // Render 2 Triangles
            // Tri 1: 0-1-2
            fill_triangle_textured(
                fb, zb,
                ((c0, w0), uv0),
                ((c1, w1), uv1),
                ((c2, w2), uv2),
                &self.texture
            );

            // Tri 2: 0-2-3
            fill_triangle_textured(
                fb, zb,
                ((c0, w0), uv0),
                ((c2, w2), uv2),
                ((c3, w3), uv3),
                &self.texture
            );
        }
    }
}

// --- Main ---

fn create_particle_texture() -> Texture {
    let size = 32;
    let mut tex = Texture::new(size, size).unwrap();
    let center = size as f32 / 2.0;
    let max_dist = center;

    for y in 0..size {
        for x in 0..size {
            let dx = x as f32 - center;
            let dy = y as f32 - center;
            let dist = (dx * dx + dy * dy).sqrt();

            if dist > max_dist {
                 tex.set_pixel(x as u32, y as u32, 0x0000_0000); // Fully transparent
            } else {
                let t = dist / max_dist;
                // Center(t=0) -> Alpha=1 (Opaque)
                // Edge(t=1) -> Alpha=254 (Transparent)
                let alpha_val = 1.0 + t * 253.0;
                let alpha_u8 = alpha_val as u8;

                 // Color: Orange Fire
                let r = 255;
                let g = ((1.0 - t) * 200.0) as u8;
                let b = 0;

                let color = ((alpha_u8 as u32) << 24) | ((r as u32) << 16) | ((g as u32) << 8) | b;
                tex.set_pixel(x as u32, y as u32, color);
            }
        }
    }
    tex.generate_mipmaps();
    tex
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut window = Window::new("Nova - Particle System", WIDTH, HEIGHT)?;
    let mut framebuffer = Framebuffer::new(WIDTH, HEIGHT)?;
    let mut zbuffer = ZBuffer::new(WIDTH, HEIGHT)?;
    let mut timestep = FixedTimestep::new(60);

    let texture = create_particle_texture();
    let mut particles = ParticleSystem::new(1000, texture);
    particles.emission_rate = 50.0;
    particles.start_life = 2.0;
    particles.spread = 0.8;
    particles.start_size = 0.5;

    // Camera setup
    let projection = Mat4::perspective(PI / 3.0, WIDTH as f32 / HEIGHT as f32, 0.1, 100.0);

    let mut angle: f32 = 0.0;

    // We need a loop logic that works for the specific window type
    while window.is_open() {
        window.poll_events();

        let steps = timestep.update();
        for _ in 0..steps {
            angle += 0.5 * timestep.dt();
            particles.update(timestep.dt());
        }

        framebuffer.clear(0xFF10_1010); // Background
        zbuffer.clear();

        let eye = Vec3::new(angle.sin() * 5.0, 2.0, angle.cos() * 5.0);
        let target = Vec3::new(0.0, 1.0, 0.0);
        let up = Vec3::new(0.0, 1.0, 0.0);
        let view = Mat4::look_at(eye, target, up);

        particles.render(&mut framebuffer, &mut zbuffer, view, projection);

        window.blit_framebuffer(&framebuffer);
    }

    Ok(())
}
