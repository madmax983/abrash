cat << 'INNER_EOF' > /tmp/water_patch.diff
<<<<<<< SEARCH
        for (x, pixel) in row.iter_mut().enumerate() {
            let x_f32 = x as f32;
            let dx = x_f32 - center_x_px;
            let distance = (dx * dx + dy_sq).sqrt();

            if distance < max_radius_px {
=======
        let max_radius_px_sq = max_radius_px * max_radius_px;
        for (x, pixel) in row.iter_mut().enumerate() {
            let x_f32 = x as f32;
            let dx = x_f32 - center_x_px;
            let dist_sq = dx * dx + dy_sq;

            if dist_sq < max_radius_px_sq {
                let distance = dist_sq.sqrt();
>>>>>>> REPLACE
INNER_EOF
patch crates/abrash-render/src/experimental/water_ripple.rs < /tmp/water_patch.diff
