sed -i 's/let intensity = (density as f32).ln() \/ log_max_density;/let intensity = (density as f32 + 1.0).ln() \/ ((max_density as f32) + 1.0).ln();/g' crates/abrash-render/src/experimental/strange_attractor.rs
sed -i 's/let log_max_density = (max_density as f32).ln();//g' crates/abrash-render/src/experimental/strange_attractor.rs
