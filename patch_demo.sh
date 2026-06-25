sed -i 's/0xFF000011/0xFF00_0011/g' examples/lightning_demo.rs
sed -i 's/0xFFAAAAFF/0xFFAA_AAFF/g' examples/lightning_demo.rs
sed -i 's/0xFFFFFFFF/0xFFFF_FFFF/g' examples/lightning_demo.rs
sed -i 's/use abrash::platform::{AppRunner, DemoApp};/#\[cfg(feature = "backend-winit")\]\nuse abrash::platform::{AppRunner, DemoApp};/g' examples/lightning_demo.rs
