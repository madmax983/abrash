//! The `Renderer` trait — core engine contract.

use crate::mesh::Mesh;
use crate::render_api::frame::Frame;
use crate::render_api::handles::{MaterialHandle, MeshHandle, TextureHandle};
use crate::render_api::material::Material;
use crate::render_api::target::RenderTarget;
use crate::texture::Texture;

/// Errors from renderer operations.
#[derive(Debug)]
pub enum RenderError {
    /// A handle referenced a resource that no longer exists.
    StaleHandle(&'static str),
    /// The mesh data was invalid (e.g., index out of bounds).
    InvalidMesh(String),
    /// The texture data was invalid.
    InvalidTexture(String),
    /// Internal renderer error.
    Internal(String),
}

impl std::fmt::Display for RenderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::StaleHandle(kind) => write!(f, "stale {kind} handle"),
            Self::InvalidMesh(msg) => write!(f, "invalid mesh: {msg}"),
            Self::InvalidTexture(msg) => write!(f, "invalid texture: {msg}"),
            Self::Internal(msg) => write!(f, "renderer error: {msg}"),
        }
    }
}

impl std::error::Error for RenderError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_render_error_display() {
        assert_eq!(
            format!("{}", RenderError::StaleHandle("mesh")),
            "stale mesh handle"
        );
        assert_eq!(
            format!(
                "{}",
                RenderError::InvalidMesh("index out of bounds".to_string())
            ),
            "invalid mesh: index out of bounds"
        );
    }
}
