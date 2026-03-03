import re

with open('src/mesh.rs', 'r') as f:
    content = f.read()

old_fn = '''    #[must_use]
    pub fn compute_face_normals(&self) -> Vec<Vec3> {
        self.indices
            .iter()
            .map(|[i0, i1, i2]| {
                let v0 = self.vertices[*i0];
                let v1 = self.vertices[*i1];
                let v2 = self.vertices[*i2];

                let edge1 = v1 - v0;
                let edge2 = v2 - v0;
                edge1.cross(edge2).normalize()
            })
            .collect()
    }'''

new_fn = '''    #[must_use]
    pub fn compute_face_normals(&self) -> Vec<Vec3> {
        let mut normals = Vec::with_capacity(self.indices.len());
        for &[i0, i1, i2] in &self.indices {
            let v0 = self.vertices[i0];
            let v1 = self.vertices[i1];
            let v2 = self.vertices[i2];

            let edge1 = v1 - v0;
            let edge2 = v2 - v0;
            normals.push(edge1.cross(edge2).normalize());
        }
        normals
    }'''

if old_fn in content:
    content = content.replace(old_fn, new_fn)
    with open('src/mesh.rs', 'w') as f:
        f.write(content)
    print("Replaced successfully!")
else:
    print("Could not find old_fn block!")
