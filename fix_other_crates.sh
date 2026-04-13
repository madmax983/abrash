#!/bin/bash

# Fix imprecise flops in math.rs
sed -i 's/(self.x \* self.x + self.y \* self.y).sqrt()/self.x.mul_add(self.x, self.y \* self.y).sqrt()/g' crates/abrash-core/src/math.rs
sed -i 's/(dx \* dx + dy \* dy).sqrt()/dx.mul_add(dx, dy \* dy).sqrt()/g' crates/abrash-core/src/math.rs
sed -i 's/(self.x \* self.x + self.y \* self.y + self.z \* self.z).sqrt()/self.x.mul_add(self.x, self.y.mul_add(self.y, self.z \* self.z)).sqrt()/g' crates/abrash-core/src/math.rs

# Fix in abrash-render procedural.rs
sed -i 's/(u \* u + v \* v).sqrt()/u.mul_add(u, v \* v).sqrt()/g' crates/abrash-render/src/procedural.rs

# Fix in abrash-render kaleidoscope.rs
sed -i 's/(dx \* dx + dy \* dy).sqrt()/dx.mul_add(dx, dy \* dy).sqrt()/g' crates/abrash-render/src/experimental/kaleidoscope.rs

# Fix in abrash-raycast bsp.rs
sed -i 's/(dx \* dx + dy \* dy).sqrt()/dx.mul_add(dx, dy \* dy).sqrt()/g' crates/abrash-raycast/src/renderer/bsp.rs

# Fix in abrash-render vision.rs
sed -i 's/center_x \* center_x + center_y \* center_y/center_x.mul_add(center_x, center_y \* center_y)/g' crates/abrash-render/src/experimental/vision.rs
sed -i 's/dx \* dx + dy \* dy/dx.mul_add(dx, dy \* dy)/g' crates/abrash-render/src/experimental/vision.rs

# Remove #[allow(clippy::imprecise_flops)]
find . -type f -name "*.rs" -exec sed -i '/#\[allow(clippy::imprecise_flops)\]/d' {} +
