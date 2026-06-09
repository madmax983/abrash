1.  **Refactor `src/platform/tui.rs`**
    *   Add types/traits mirroring `winit.rs` (`WindowHostConfig`, `WindowContext`, `WindowApp`, `HostError`, `FrameClock`).
    *   Implement `run_windowed` in `tui.rs` to run the app using Ratatui/Crossterm instead of an explicit `while window.is_open()` loop.
    *   Implement `SoftwarePresenter` in `tui.rs` to act as a no-op or pass-through that tells the `TuiWindow` to blit the framebuffer. (We could also change `TuiWindow` to implement `SoftwarePresenter`). Wait, `run_windowed` takes a `WindowApp`. The `WindowApp` creates a `SoftwarePresenter`. The `SoftwarePresenter` needs some window context.
2.  **Refactor `src/platform/mod.rs`**
    *   Conditionally export `tui::*` or `winit::*` based on the backend feature.
    *   Re-export `WindowApp`, `WindowContext`, `WindowHostConfig`, `HostError`, `SoftwarePresenter`, `run_windowed`.
3.  **Update `examples/cube_3d.rs` (and other examples)**
    *   Remove `use abrash_render::platform::tui::TuiWindow;`.
    *   Ensure they just use `abrash::platform::{run_windowed, WindowApp, ...}` and it works transparently.
4.  **Complete pre commit steps**
    *   Complete pre commit steps to ensure proper testing, verification, review, and reflection are done.
5.  **Submit the change**
    *   Branch: `atlas-tui-winit-split`
    *   Title: `🗺️ Atlas: [Winit/TUI Backend Split]`
    *   Message: Includes Tangle, Blueprint, Stability, Verification.
