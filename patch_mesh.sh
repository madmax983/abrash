sed -i -e '/pub fn sphere(/,/for i in 0..=stacks {/c\
    pub fn sphere(radius: f32, stacks: u32, sectors: u32) -> Self {\
        use std::f32::consts::PI;\
\
        let num_vertices = ((stacks + 1) * (sectors + 1)) as usize;\
        let num_indices = (stacks * sectors * 6) as usize;\
\
        let mut vertices = Vec::with_capacity(num_vertices);\
        let mut normals = Vec::with_capacity(num_vertices);\
        let mut uvs = Vec::with_capacity(num_vertices);\
        let mut indices = Vec::with_capacity(num_indices);\
\
        for i in 0..=stacks {' crates/abrash-core/src/mesh.rs

sed -i -e '/pub fn plane(/,/for z in 0..=divs {/c\
    pub fn plane(size: f32, subdivisions: u32) -> Self {\
        let half = size / 2.0;\
        let divs = subdivisions.max(1);\
        let step = size / divs as f32;\
\
        let num_vertices = ((divs + 1) * (divs + 1)) as usize;\
        let num_indices = (divs * divs * 6) as usize;\
\
        let mut vertices = Vec::with_capacity(num_vertices);\
        let mut normals = Vec::with_capacity(num_vertices);\
        let mut uvs = Vec::with_capacity(num_vertices);\
        let mut indices = Vec::with_capacity(num_indices);\
\
        for z in 0..=divs {' crates/abrash-core/src/mesh.rs

sed -i -e '/pub fn cylinder(/,/for i in 0..=stacks {/c\
    pub fn cylinder(radius: f32, height: f32, sectors: u32, stacks: u32) -> Self {\
        use std::f32::consts::PI;\
\
        let half_h = height / 2.0;\
        let sectors = sectors.max(3);\
        let stacks = stacks.max(1);\
\
        let body_vertices = (stacks + 1) * (sectors + 1);\
        let cap_vertices = (sectors + 1) * 2;\
        let num_vertices = (body_vertices + cap_vertices) as usize;\
\
        let body_indices = stacks * sectors * 6;\
        let cap_indices = sectors * 6;\
        let num_indices = (body_indices + cap_indices) as usize;\
\
        let mut vertices = Vec::with_capacity(num_vertices);\
        let mut normals = Vec::with_capacity(num_vertices);\
        let mut uvs = Vec::with_capacity(num_vertices);\
        let mut indices = Vec::with_capacity(num_indices);\
\
        for i in 0..=stacks {' crates/abrash-core/src/mesh.rs

sed -i -e '/pub fn torus(/,/for i in 0..=maj {/c\
    pub fn torus(\
        major_radius: f32,\
        minor_radius: f32,\
        major_segments: u32,\
        minor_segments: u32,\
    ) -> Self {\
        use std::f32::consts::PI;\
\
        let maj = major_segments.max(3);\
        let min = minor_segments.max(3);\
\
        let num_vertices = ((maj + 1) * (min + 1)) as usize;\
        let num_indices = (maj * min * 6) as usize;\
\
        let mut vertices = Vec::with_capacity(num_vertices);\
        let mut normals = Vec::with_capacity(num_vertices);\
        let mut uvs = Vec::with_capacity(num_vertices);\
        let mut indices = Vec::with_capacity(num_indices);\
\
        for i in 0..=maj {' crates/abrash-core/src/mesh.rs
