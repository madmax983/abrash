## [UI Polish]
**Bloat:** Raw panic unwraps in main and demo apps which drop back to terminal unceremoniously, as well as `arboretum_cli` missing `nova` feature and dashboard `main.rs` exits.
**Cut:** Replaced unwraps with `run_windowed_app` mapping errors, added graceful formatted exit pauses (`Press Enter to exit...`).
**Saved:** Ugly `unwrap()` panics on window creation failures across 60+ demos are now clean formatted UI interactions before exiting.
## [UI Polish]
**Bloat:** Raw panic unwraps in main and demo apps which drop back to terminal unceremoniously, as well as `arboretum_cli` missing `nova` feature and dashboard `main.rs` exits.
**Cut:** Replaced unwraps with `run_windowed_app` mapping errors, added graceful formatted exit pauses (`Press Enter to exit...`).
**Saved:** Ugly `unwrap()` panics on window creation failures across 60+ demos are now clean formatted UI interactions before exiting.
