#[cfg(target_os = "windows")]
pub mod win32;

#[cfg(target_os = "windows")]
pub use win32::Window;

#[cfg(not(target_os = "windows"))]
pub mod dummy;

#[cfg(not(target_os = "windows"))]
pub use dummy::Window;
