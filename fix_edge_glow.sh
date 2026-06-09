sed -i 's/TuiWindow::new("Edge Glow Demo", width, height)?/TuiWindow::new("Edge Glow Demo", width, height).unwrap()/g' examples/edge_glow_demo.rs
sed -i 's/use abrash::platform::tui::TuiWindow;/use abrash::platform::tui::TuiWindow;\nuse abrash::platform::{run_windowed, WindowApp, WindowContext, WindowHostConfig, HostError};/g' examples/edge_glow_demo.rs
sed -i 's/TuiWindow::new/abrash::platform::TuiWindow::new/g' examples/edge_glow_demo.rs
