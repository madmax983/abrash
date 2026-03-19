# ADR 008: Root Winit Platform Host

## Status
Accepted

## Context

The root crate currently treats `backend-win32` as the default native desktop backend.
That makes the host seam Windows-first even though the GPU path already uses `winit`,
and it leaves the TUI launcher as the primary interactive path.

We want a thin cross-platform desktop host in the root crate that does not dictate what
an app boots into. Examples and future integrations should supply their own render and
update logic, while `backend-tui` remains available as an explicit opt-in.

## Decision

- Add `backend-winit` to the root crate and make it the default desktop backend.
- Keep `backend-tui` as an explicit alternate mode.
- Put the host API in `src/platform/` in the root crate.
- Keep the host thin and callback-based instead of introducing an app shell.
- Use `softbuffer` for CPU framebuffer presentation on the `winit` window.
- Leave the GPU-default flip for a later phase.

## Consequences

- CPU examples stop depending on the Win32-only `Window` implementation.
- The root binary can default to a native desktop window without hard-coding a scene.
- `backend-win32` remains only where D3D12-specific code still needs it.
