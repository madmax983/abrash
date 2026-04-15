**[CLI Dashboard Banners]**
**Learning:** Consistent visual hierarchy and dashboard-like outputs significantly improve CLI UX. Standardize CLI outputs using `comfy_table` with `UTF8_ROUND_CORNERS`. Extract logic into a `print_banner` helper function to keep `main()` clean and adhere to the "Z-Pattern" scanning layout.
**Action:** Implemented `print_banner()` across all relevant examples (`obj_viewer.rs`, `mandelbrot_demo.rs`, `gpu_mvp_cube.rs`, `color_splash_demo.rs`, `reaction_diffusion_demo.rs`, `gpu_deferred_showcase.rs`), enforcing standard properties, descriptions, and controls tables.
