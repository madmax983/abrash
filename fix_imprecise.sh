#!/bin/bash

# Fast multiplication addition wrapper fix
sed -i 's/(p.x \* p.x + p.y \* p.y).sqrt()/p.x.mul_add(p.x, p.y \* p.y).sqrt()/g' crates/abrash-core/src/sdf.rs
sed -i 's/(q.x \* q.x + q.y \* q.y).sqrt()/q.x.mul_add(q.x, q.y \* q.y).sqrt()/g' crates/abrash-core/src/sdf.rs
sed -i 's/(zx \* zx + zy \* zy).sqrt()/zx.mul_add(zx, zy \* zy).sqrt()/g' crates/abrash-core/src/sdf.rs
sed -i 's/(dzx \* dzx + dzy \* dzy).sqrt()/dzx.mul_add(dzx, dzy \* dzy).sqrt()/g' crates/abrash-core/src/sdf.rs
sed -i 's/(dx \* dx + dy \* dy).sqrt()/dx.mul_add(dx, dy \* dy).sqrt()/g' crates/abrash-core/src/sdf.rs
sed -i 's/(p.x \* p.x + p.z \* p.z).sqrt()/p.x.mul_add(p.x, p.z \* p.z).sqrt()/g' crates/abrash-core/src/sdf.rs
sed -i 's/(gx \* gx + gy \* gy).sqrt()/gx.mul_add(gx, gy \* gy).sqrt()/g' crates/abrash-core/src/sdf.rs
