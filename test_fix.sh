#!/bin/bash
sed -i 's/(self.x \* self.x + self.y \* self.y).sqrt()/self.x.mul_add(self.x, self.y \* self.y).sqrt()/g' crates/abrash-core/src/math.rs
sed -i 's/(dx \* dx + dy \* dy).sqrt()/dx.mul_add(dx, dy \* dy).sqrt()/g' crates/abrash-core/src/math.rs
sed -i 's/(self.x \* self.x + self.y \* self.y + self.z \* self.z).sqrt()/self.x.mul_add(self.x, self.y.mul_add(self.y, self.z \* self.z)).sqrt()/g' crates/abrash-core/src/math.rs

sed -i 's/(p.x \* p.x + p.y \* p.y).sqrt()/p.x.mul_add(p.x, p.y \* p.y).sqrt()/g' crates/abrash-core/src/sdf.rs
sed -i 's/(q.x \* q.x + q.y \* q.y).sqrt()/q.x.mul_add(q.x, q.y \* q.y).sqrt()/g' crates/abrash-core/src/sdf.rs
sed -i 's/(zx \* zx + zy \* zy).sqrt()/zx.mul_add(zx, zy \* zy).sqrt()/g' crates/abrash-core/src/sdf.rs
sed -i 's/(dzx \* dzx + dzy \* dzy).sqrt()/dzx.mul_add(dzx, dzy \* dzy).sqrt()/g' crates/abrash-core/src/sdf.rs
sed -i 's/(dx \* dx + dy \* dy).sqrt()/dx.mul_add(dx, dy \* dy).sqrt()/g' crates/abrash-core/src/sdf.rs
sed -i 's/(p.x \* p.x + p.z \* p.z).sqrt()/p.x.mul_add(p.x, p.z \* p.z).sqrt()/g' crates/abrash-core/src/sdf.rs
sed -i 's/(gx \* gx + gy \* gy).sqrt()/gx.mul_add(gx, gy \* gy).sqrt()/g' crates/abrash-core/src/sdf.rs

sed -i 's/(u \* u + v \* v).sqrt()/u.mul_add(u, v \* v).sqrt()/g' crates/abrash-render/src/procedural.rs
sed -i 's/(dx \* dx + dy \* dy).sqrt()/dx.mul_add(dx, dy \* dy).sqrt()/g' crates/abrash-render/src/experimental/kaleidoscope.rs
sed -i 's/(dx \* dx + dy \* dy).sqrt()/dx.mul_add(dx, dy \* dy).sqrt()/g' crates/abrash-raycast/src/renderer/bsp.rs
sed -i 's/center_x \* center_x + center_y \* center_y/center_x.mul_add(center_x, center_y \* center_y)/g' crates/abrash-render/src/experimental/vision.rs
sed -i 's/dx \* dx + dy \* dy/dx.mul_add(dx, dy \* dy)/g' crates/abrash-render/src/experimental/vision.rs
sed -i 's/dx \* dx + dy \* dy/dx.mul_add(dx, dy \* dy)/g' crates/abrash-render/src/experimental/voronoi.rs
