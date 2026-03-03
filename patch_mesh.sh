sed -i 's/let mut tan1 = Vec::with_capacity(self.vertices.len());/let mut tan1 = vec![Vec3::default(); self.vertices.len()];/' src/mesh.rs
sed -i 's/        tan1.resize(self.vertices.len(), Vec3::default());//' src/mesh.rs
sed -i 's/let mut tan2 = Vec::with_capacity(self.vertices.len());/let mut tan2 = vec![Vec3::default(); self.vertices.len()];/' src/mesh.rs
sed -i 's/        tan2.resize(self.vertices.len(), Vec3::default());//' src/mesh.rs
