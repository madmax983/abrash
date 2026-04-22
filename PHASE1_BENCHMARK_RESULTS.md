scene_render_100_objects
                        time:   [396.00 µs 397.14 µs 398.44 µs]
                        change: [-1.0946% +0.1469% +1.0479%] (p = 0.84 > 0.05)
                        No change in performance detected.

scene_render_integrated_clear_100_objects
                        time:   [282.65 µs 283.48 µs 284.42 µs]
                        change: [+0.9597% +1.6476% +2.2416%] (p = 0.00 < 0.05)
                        Change within noise threshold.
Found 6 outliers among 100 measurements (6.00%)
  6 (6.00%) high mild

scene_extract_only_100_objects
                        time:   [16.675 µs 16.794 µs 17.011 µs]
                        change: [-24.896% -20.460% -15.688%] (p = 0.00 < 0.05)
                        Performance has improved.
Found 4 outliers among 100 measurements (4.00%)
  2 (2.00%) high mild
  2 (2.00%) high severe

scene_clear_only        time:   [112.73 µs 113.24 µs 113.84 µs]
                        change: [+0.2594% +1.0032% +1.7068%] (p = 0.01 < 0.05)
                        Change within noise threshold.
Found 9 outliers among 100 measurements (9.00%)
  9 (9.00%) high mild

scene_rasterize_only_100_objects
                        time:   [267.84 µs 273.10 µs 279.27 µs]
                        change: [+0.5018% +1.6667% +2.9384%] (p = 0.01 < 0.05)
                        Change within noise threshold.
Found 13 outliers among 100 measurements (13.00%)
  2 (2.00%) low mild
  1 (1.00%) high mild
  10 (10.00%) high severe

scene_rasterize_integrated_clear_100_objects
                        time:   [270.89 µs 281.21 µs 292.91 µs]
                        change: [+1.8459% +3.5405% +6.0779%] (p = 0.00 < 0.05)
                        Performance has regressed.
Found 15 outliers among 100 measurements (15.00%)
  1 (1.00%) low mild
  4 (4.00%) high mild
  10 (10.00%) high severe

submit_mesh_only_20k_tris
                        time:   [265.26 µs 265.71 µs 266.22 µs]
                        change: [-0.1407% +0.1053% +0.3427%] (p = 0.39 > 0.05)
                        No change in performance detected.
Found 2 outliers among 100 measurements (2.00%)
  1 (1.00%) high mild
  1 (1.00%) high severe

1080p_20k_tris_100obj   time:   [245.51 µs 245.91 µs 246.39 µs]
                        change: [-0.6021% -0.0681% +0.3155%] (p = 0.82 > 0.05)
                        No change in performance detected.
Found 6 outliers among 100 measurements (6.00%)
  1 (1.00%) low mild
  4 (4.00%) high mild
  1 (1.00%) high severe

1080p_80k_tris_400obj   time:   [952.55 µs 953.58 µs 954.62 µs]
                        change: [-0.4462% -0.3255% -0.2008%] (p = 0.00 < 0.05)
                        Change within noise threshold.
Found 1 outliers among 100 measurements (1.00%)
  1 (1.00%) high mild

1080p_80k_tris_dense_100obj
                        time:   [1.1427 ms 1.1442 ms 1.1458 ms]
                        change: [-0.1355% +0.0690% +0.2633%] (p = 0.51 > 0.05)
                        No change in performance detected.
Found 2 outliers among 100 measurements (2.00%)
  1 (1.00%) low mild
  1 (1.00%) high mild

4k_20k_tris_100obj      time:   [248.10 µs 248.64 µs 249.25 µs]
                        change: [+0.3112% +0.6051% +0.8972%] (p = 0.00 < 0.05)
                        Change within noise threshold.
Found 6 outliers among 100 measurements (6.00%)
  6 (6.00%) high mild

