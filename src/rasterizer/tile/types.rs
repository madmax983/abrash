use super::context::CompactScreenPoint;
use crate::math::ScreenPoint;
use crate::math::Vec3;
use crate::rasterizer::PerspectiveTextureGradients;
use crate::rasterizer::gouraud::GouraudGradients;
use std::mem::MaybeUninit;

/// A triangle that has been clipped, projected, culled, Y-sorted, and had gradients computed.
#[derive(Clone, Copy)]
pub struct PreparedTriangle {
    pub p0: CompactScreenPoint,
    pub p1: CompactScreenPoint,
    pub p2: CompactScreenPoint,
    pub dz_dx: f32,
    pub long_edge_is_left: bool,
    pub color: u32,
    pub aabb_min_x: u16,
    pub aabb_min_y: u16,
    pub aabb_max_x: u16,
    pub aabb_max_y: u16,
    pub min_depth: f32, // Minimum depth across triangle
    pub max_depth: f32, // Maximum depth across triangle
}

/// A Gouraud-shaded triangle prepared for rasterization.
#[derive(Clone, Copy)]
pub struct PreparedGouraudTriangle {
    pub p0: CompactScreenPoint,
    pub p1: CompactScreenPoint,
    pub p2: CompactScreenPoint,
    pub c0: (i32, i32, i32), // Fixed-point color at p0
    pub c1: (i32, i32, i32),
    pub c2: (i32, i32, i32),
    pub gradients: GouraudGradients,
    pub long_edge_is_left: bool,
    pub aabb_min_x: u16,
    pub aabb_min_y: u16,
    pub aabb_max_x: u16,
    pub aabb_max_y: u16,
    pub min_depth: f32,
    pub max_depth: f32,
}

/// A textured triangle prepared for rasterization.
///
/// Optimized to fit in exactly 128 bytes (2 cache lines).
#[derive(Clone, Copy)]
pub struct PreparedTexturedTriangle {
    pub p0: ScreenPoint,
    pub p1: ScreenPoint,
    pub p2: ScreenPoint,
    pub u0: f32,
    pub u1: f32,
    pub u2: f32,
    pub v0: f32,
    pub v1: f32,
    pub v2: f32,
    pub gradients: PerspectiveTextureGradients,
    pub long_edge_is_left: bool,
    pub aabb_min_x: i32,
    pub aabb_min_y: i32,
    pub aabb_max_x: i32,
    pub aabb_max_y: i32,
    pub min_depth: f32,
    pub max_depth: f32,
}

pub struct PreparedGouraudTrianglesList {
    pub tris: [MaybeUninit<PreparedGouraudTriangle>; 8],
    pub count: usize,
}

impl PreparedGouraudTrianglesList {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            tris: unsafe { MaybeUninit::uninit().assume_init() },
            count: 0,
        }
    }

    pub const fn push(&mut self, tri: PreparedGouraudTriangle) {
        if self.count < 8 {
            self.tris[self.count].write(tri);
            self.count += 1;
        }
    }
}

impl IntoIterator for PreparedGouraudTrianglesList {
    type Item = PreparedGouraudTriangle;
    type IntoIter = PreparedGouraudTrianglesIter;

    fn into_iter(self) -> Self::IntoIter {
        PreparedGouraudTrianglesIter {
            list: self,
            index: 0,
        }
    }
}

pub struct PreparedGouraudTrianglesIter {
    list: PreparedGouraudTrianglesList,
    index: usize,
}

impl Iterator for PreparedGouraudTrianglesIter {
    type Item = PreparedGouraudTriangle;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index < self.list.count {
            let item = unsafe { self.list.tris[self.index].assume_init() };
            self.index += 1;
            Some(item)
        } else {
            None
        }
    }
}

pub struct PreparedTrianglesList {
    pub tris: [MaybeUninit<PreparedTriangle>; 8],
    pub count: usize,
}

impl PreparedTrianglesList {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            tris: unsafe { MaybeUninit::uninit().assume_init() },
            count: 0,
        }
    }

    pub const fn push(&mut self, tri: PreparedTriangle) {
        if self.count < 8 {
            self.tris[self.count].write(tri);
            self.count += 1;
        }
    }
}

impl IntoIterator for PreparedTrianglesList {
    type Item = PreparedTriangle;
    type IntoIter = PreparedTrianglesIter;

    fn into_iter(self) -> Self::IntoIter {
        PreparedTrianglesIter {
            list: self,
            index: 0,
        }
    }
}

#[cfg(feature = "parallel")]
impl rayon::iter::IntoParallelIterator for PreparedTrianglesList {
    type Item = PreparedTriangle;
    type Iter = rayon::vec::IntoIter<PreparedTriangle>;

    fn into_par_iter(self) -> Self::Iter {
        let mut vec = Vec::with_capacity(self.count);
        for i in 0..self.count {
            vec.push(unsafe { self.tris[i].assume_init() });
        }
        vec.into_par_iter()
    }
}

#[cfg(feature = "parallel")]
impl rayon::iter::IntoParallelIterator for PreparedTexturedTrianglesList {
    type Item = PreparedTexturedTriangle;
    type Iter = rayon::vec::IntoIter<PreparedTexturedTriangle>;

    fn into_par_iter(self) -> Self::Iter {
        let mut vec = Vec::with_capacity(self.count);
        for i in 0..self.count {
            vec.push(unsafe { self.tris[i].assume_init() });
        }
        vec.into_par_iter()
    }
}

#[cfg(feature = "parallel")]
impl rayon::iter::IntoParallelIterator for PreparedGouraudTrianglesList {
    type Item = PreparedGouraudTriangle;
    type Iter = rayon::vec::IntoIter<PreparedGouraudTriangle>;

    fn into_par_iter(self) -> Self::Iter {
        let mut vec = Vec::with_capacity(self.count);
        for i in 0..self.count {
            vec.push(unsafe { self.tris[i].assume_init() });
        }
        vec.into_par_iter()
    }
}

pub struct PreparedTrianglesIter {
    list: PreparedTrianglesList,
    index: usize,
}

impl Iterator for PreparedTrianglesIter {
    type Item = PreparedTriangle;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index < self.list.count {
            let item = unsafe { self.list.tris[self.index].assume_init() };
            self.index += 1;
            Some(item)
        } else {
            None
        }
    }
}

pub struct PreparedTexturedTrianglesList {
    pub tris: [MaybeUninit<PreparedTexturedTriangle>; 8],
    pub count: usize,
}

impl PreparedTexturedTrianglesList {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            tris: unsafe { MaybeUninit::uninit().assume_init() },
            count: 0,
        }
    }

    pub const fn push(&mut self, tri: PreparedTexturedTriangle) {
        if self.count < 8 {
            self.tris[self.count].write(tri);
            self.count += 1;
        }
    }
}

impl IntoIterator for PreparedTexturedTrianglesList {
    type Item = PreparedTexturedTriangle;
    type IntoIter = PreparedTexturedTrianglesIter;

    fn into_iter(self) -> Self::IntoIter {
        PreparedTexturedTrianglesIter {
            list: self,
            index: 0,
        }
    }
}

pub struct PreparedTexturedTrianglesIter {
    list: PreparedTexturedTrianglesList,
    index: usize,
}

impl Iterator for PreparedTexturedTrianglesIter {
    type Item = PreparedTexturedTriangle;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index < self.list.count {
            let item = unsafe { self.list.tris[self.index].assume_init() };
            self.index += 1;
            Some(item)
        } else {
            None
        }
    }
}

/// Flattened linked-list structure for tile binning.
///
/// Replaces `Vec<Vec<usize>>` to reduce heap allocations and improve cache locality.
pub struct TileBins {
    pub heads: Vec<u32>, // Index into nexts/tris. u32::MAX = None
    pub tails: Vec<u32>, // Index into nexts/tris. u32::MAX = None
    pub nexts: Vec<u32>, // Link to next node
    pub tris: Vec<u32>,  // Triangle index
}

impl TileBins {
    #[must_use]
    pub fn new(num_tiles: usize) -> Self {
        Self {
            heads: vec![u32::MAX; num_tiles],
            tails: vec![u32::MAX; num_tiles],
            nexts: Vec::with_capacity(1024),
            tris: Vec::with_capacity(1024),
        }
    }

    pub fn clear(&mut self) {
        self.heads.fill(u32::MAX);
        self.tails.fill(u32::MAX);
        self.nexts.clear();
        self.tris.clear();
    }

    #[inline]
    pub fn push(&mut self, tile_idx: usize, tri_idx: usize) {
        let node_idx = self.tris.len() as u32;
        self.tris.push(tri_idx as u32);
        self.nexts.push(u32::MAX);

        let head = self.heads[tile_idx];
        if head == u32::MAX {
            self.heads[tile_idx] = node_idx;
        } else {
            let tail = self.tails[tile_idx];
            self.nexts[tail as usize] = node_idx;
        }
        self.tails[tile_idx] = node_idx;
    }

    #[must_use]
    pub fn iter(&self, tile_idx: usize) -> TileBinIter<'_> {
        TileBinIter {
            bins: self,
            curr: self.heads[tile_idx],
        }
    }
}

pub struct TileBinIter<'a> {
    bins: &'a TileBins,
    curr: u32,
}

impl Iterator for TileBinIter<'_> {
    type Item = usize;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        if self.curr == u32::MAX {
            None
        } else {
            let idx = self.curr as usize;
            let tri_idx = self.bins.tris[idx] as usize;
            self.curr = self.bins.nexts[idx];
            Some(tri_idx)
        }
    }
}
