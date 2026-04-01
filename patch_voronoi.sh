cat << 'INNER_EOF' > /tmp/voronoi_patch.diff
<<<<<<< SEARCH
                // Calculate Minkowski distance.
                // Fast paths for Euclidean (metric == 2) and Manhattan (metric == 1).
                let dist = if (metric - 2.0).abs() < f32::EPSILON {
                    (dx * dx + dy * dy).sqrt()
                } else if (metric - 1.0).abs() < f32::EPSILON {
                    dx + dy
                } else {
                    // General case
                    (dx.powf(metric) + dy.powf(metric)).powf(1.0 / metric)
                };

                if dist < min_dist {
                    second_min_dist = min_dist;
                    min_dist = dist;
                    closest_idx = i;
                } else if dist < second_min_dist {
                    second_min_dist = dist;
                }
=======
                // Calculate Minkowski distance squared where possible to avoid `sqrt` in inner loop
                // Fast paths for Euclidean (metric == 2) and Manhattan (metric == 1).
                if (metric - 2.0).abs() < f32::EPSILON {
                    let dist_sq = dx * dx + dy * dy;
                    if dist_sq < min_dist {
                        second_min_dist = min_dist;
                        min_dist = dist_sq;
                        closest_idx = i;
                    } else if dist_sq < second_min_dist {
                        second_min_dist = dist_sq;
                    }
                } else {
                    let dist = if (metric - 1.0).abs() < f32::EPSILON {
                        dx + dy
                    } else {
                        // General case
                        (dx.powf(metric) + dy.powf(metric)).powf(1.0 / metric)
                    };

                    if dist < min_dist {
                        second_min_dist = min_dist;
                        min_dist = dist;
                        closest_idx = i;
                    } else if dist < second_min_dist {
                        second_min_dist = dist;
                    }
                }
>>>>>>> REPLACE
INNER_EOF
