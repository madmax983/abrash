use std::ops::{Add, Mul, Sub};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Vec3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl Vec3 {
    pub fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z }
    }

    pub fn dot(self, other: Vec3) -> f32 {
        self.x * other.x + self.y * other.y + self.z * other.z
    }

    pub fn cross(self, other: Vec3) -> Self {
        Self {
            x: self.y * other.z - self.z * other.y,
            y: self.z * other.x - self.x * other.z,
            z: self.x * other.y - self.y * other.x,
        }
    }

    pub fn normalize(self) -> Self {
        let len = (self.x * self.x + self.y * self.y + self.z * self.z).sqrt();
        if len > 0.0 {
            Self {
                x: self.x / len,
                y: self.y / len,
                z: self.z / len,
            }
        } else {
            self
        }
    }
}

impl Sub for Vec3 {
    type Output = Self;
    fn sub(self, other: Self) -> Self {
        Self {
            x: self.x - other.x,
            y: self.y - other.y,
            z: self.z - other.z,
        }
    }
}

impl Add for Vec3 {
    type Output = Self;
    fn add(self, other: Self) -> Self {
        Self {
            x: self.x + other.x,
            y: self.y + other.y,
            z: self.z + other.z,
        }
    }
}

impl Mul<f32> for Vec3 {
    type Output = Self;
    fn mul(self, scalar: f32) -> Self {
        Self {
            x: self.x * scalar,
            y: self.y * scalar,
            z: self.z * scalar,
        }
    }
}

pub struct Framebuffer {
    pub width: usize,
    pub height: usize,
    pub buffer: Vec<u32>,
    pub z_buffer: Vec<f32>,
}

impl Framebuffer {
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            width,
            height,
            buffer: vec![0; width * height],
            z_buffer: vec![f32::INFINITY; width * height],
        }
    }

    pub fn clear(&mut self, color: u32) {
        self.buffer.fill(color);
        self.z_buffer.fill(f32::INFINITY);
    }

    pub fn set_pixel(&mut self, x: usize, y: usize, color: u32) {
        if x < self.width && y < self.height {
            self.buffer[y * self.width + x] = color;
        }
    }

    pub fn get_pixel(&self, x: usize, y: usize) -> u32 {
        if x < self.width && y < self.height {
            self.buffer[y * self.width + x]
        } else {
            0
        }
    }
}

pub fn draw_triangle(
    fb: &mut Framebuffer,
    v0: Vec3,
    v1: Vec3,
    v2: Vec3,
    color: u32,
) {
    // Basic 2D bounding box
    let min_x = v0.x.min(v1.x).min(v2.x).max(0.0) as usize;
    let min_y = v0.y.min(v1.y).min(v2.y).max(0.0) as usize;
    let max_x = v0.x.max(v1.x).max(v2.x).min((fb.width - 1) as f32) as usize;
    let max_y = v0.y.max(v1.y).max(v2.y).min((fb.height - 1) as f32) as usize;

    let edge_function = |a: &Vec3, b: &Vec3, c: &Vec3| -> f32 {
        (c.x - a.x) * (b.y - a.y) - (c.y - a.y) * (b.x - a.x)
    };

    let area = edge_function(&v0, &v1, &v2);
    if area == 0.0 {
        return;
    }

    for y in min_y..=max_y {
        for x in min_x..=max_x {
            let p = Vec3::new(x as f32 + 0.5, y as f32 + 0.5, 0.0);

            let w0 = edge_function(&v1, &v2, &p);
            let w1 = edge_function(&v2, &v0, &p);
            let w2 = edge_function(&v0, &v1, &p);

            if w0 >= 0.0 && w1 >= 0.0 && w2 >= 0.0 {
                // Barycentric coordinates
                let w0 = w0 / area;
                let w1 = w1 / area;
                let w2 = w2 / area;

                let z = w0 * v0.z + w1 * v1.z + w2 * v2.z;

                let idx = y * fb.width + x;
                if z < fb.z_buffer[idx] {
                    fb.z_buffer[idx] = z;
                    fb.buffer[idx] = color;
                }
            }
        }
    }
}
