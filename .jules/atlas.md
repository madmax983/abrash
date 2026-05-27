## [Unified Experimental Error Handling]
**Tangle:** Inconsistent error handling (mixing `String` and `&'static str`) across `experimental` modules like `lsystem`, `arboretum`, `jelly`, and `steganography`, creating an unpredictable API.
**Blueprint:**
1.  **Introduce Central Error:** Created a unified `Error` enum in `crates/abrash-render/src/experimental/error.rs` to encapsulate all experimental failure modes (e.g., `CapacityExceeded`, `MeshIndexOutOfBounds`).
2.  **Refactor Modules:** Updated experimental modules to return `Result<T, crate::experimental::error::Error>` instead of primitive string errors.
3.  **Result:** Standardized error boundaries within the `experimental` module, improving maintainability and ensuring safe, idiomatic error propagation.
