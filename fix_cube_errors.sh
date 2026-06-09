sed -i 's/apply_edge_glow/apply_sobel_edge_detection/g' examples/edge_glow_demo.rs
sed -i 's/EdgeGlowConfig/SobelConfig/g' examples/edge_glow_demo.rs
sed -i 's/SoftwarePresenter::new(ctx)?/SoftwarePresenter::new(ctx.window.clone())?/g' examples/cube_3d.rs
sed -i 's/SoftwarePresenter::new(ctx)?/SoftwarePresenter::new(ctx.window.clone())?/g' examples/god_rays_demo.rs
sed -i 's/SoftwarePresenter::new(ctx)?/SoftwarePresenter::new(ctx.window.clone())?/g' examples/edge_glow_demo.rs
sed -i 's/Mat4::translation(Vec3::new(offset_x, 0.0, 0.0))/Mat4::translation(offset_x, 0.0, 0.0)/g' examples/edge_glow_demo.rs
sed -i 's/COLORS.length/COLORS.len()/g' examples/edge_glow_demo.rs
