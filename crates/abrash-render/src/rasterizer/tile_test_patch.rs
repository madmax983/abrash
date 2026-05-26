use std::mem::MaybeUninit;

#[derive(Clone, Copy)]
pub struct PreparedTriangle {
    pub dummy: i32,
}

pub struct PreparedTrianglesList {
    pub tris: [MaybeUninit<PreparedTriangle>; 8],
    count: usize,
}

impl PreparedTrianglesList {
    pub const fn new() -> Self {
        Self {
            tris: [const { MaybeUninit::uninit() }; 8],
            count: 0,
        }
    }

    pub const fn push(&mut self, tri: PreparedTriangle) {
        if self.count < 8 {
            self.tris[self.count] = MaybeUninit::new(tri);
            self.count += 1;
        }
    }
}
