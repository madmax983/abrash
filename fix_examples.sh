sed -i 's/TuiWindow::new("God Rays Demo", WIDTH, HEIGHT)?;/TuiWindow::new("God Rays Demo", WIDTH, HEIGHT).unwrap();/g' examples/god_rays_demo.rs
sed -i 's/TuiWindow::new("Edge Glow Demo", width, height)?;/TuiWindow::new("Edge Glow Demo", width, height).unwrap();/g' examples/edge_glow_demo.rs
