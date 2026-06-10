#!/bin/bash

# Create experimental/error.rs
cat << 'ERROR_RS' > crates/abrash-render/src/experimental/error.rs
use std::fmt;

#[derive(Debug, PartialEq, Eq)]
pub enum Error {
    CapacityExceeded(&'static str),
    MeshIndexOutOfBounds(usize, usize),
    InvalidInput(&'static str),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CapacityExceeded(msg) => write!(f, "Capacity exceeded: {msg}"),
            Self::MeshIndexOutOfBounds(idx, max) => write!(f, "Mesh index {idx} out of bounds (max {max})"),
            Self::InvalidInput(msg) => write!(f, "Invalid input: {msg}"),
        }
    }
}

impl std::error::Error for Error {}
ERROR_RS

# Add error module
sed -i '27a\pub mod error;' crates/abrash-render/src/experimental/mod.rs

# L-System
sed -i 's/Result<String, &\x27static str>/Result<String, crate::experimental::error::Error>/g' crates/abrash-render/src/experimental/lsystem.rs
sed -i 's/Err("L-System expansion exceeded maximum capacity limit")/Err(crate::experimental::error::Error::CapacityExceeded("L-System expansion exceeded maximum capacity limit"))/g' crates/abrash-render/src/experimental/lsystem.rs
sed -i 's/Result<Mesh, &\x27static str>/Result<Mesh, crate::experimental::error::Error>/g' crates/abrash-render/src/experimental/lsystem.rs
sed -i 's/Err("L-System stack overflow")/Err(crate::experimental::error::Error::CapacityExceeded("L-System stack overflow"))/g' crates/abrash-render/src/experimental/lsystem.rs
sed -i 's/Err("L-System stack underflow")/Err(crate::experimental::error::Error::CapacityExceeded("L-System stack underflow"))/g' crates/abrash-render/src/experimental/lsystem.rs
sed -i 's/\.map_err(|_| "L-System utf8 decoding error")/.map_err(|_| crate::experimental::error::Error::InvalidInput("L-System utf8 decoding error"))/g' crates/abrash-render/src/experimental/lsystem.rs

# Arboretum
sed -i 's/Result<String, String>/Result<String, crate::experimental::error::Error>/g' crates/abrash-render/src/experimental/arboretum.rs
sed -i 's/Err("Expansion exceeded maximum string capacity"\.to_string())/Err(crate::experimental::error::Error::CapacityExceeded("Expansion exceeded maximum string capacity"))/g' crates/abrash-render/src/experimental/arboretum.rs
sed -i 's/Err("Capacity exceeded during L-System string expansion"\.to_string())/Err(crate::experimental::error::Error::CapacityExceeded("Capacity exceeded during L-System string expansion"))/g' crates/abrash-render/src/experimental/arboretum.rs
sed -i 's/Result<Mesh, String>/Result<Mesh, crate::experimental::error::Error>/g' crates/abrash-render/src/experimental/arboretum.rs
sed -i 's/\.map_err(|e| e\.to_string())/.map_err(|_| crate::experimental::error::Error::InvalidInput("L-System utf8 decoding error"))/g' crates/abrash-render/src/experimental/arboretum.rs
sed -i 's/return Err("L-system exceeded memory limits"\.to_owned());/return Err(crate::experimental::error::Error::CapacityExceeded("L-system exceeded memory limits"));/g' crates/abrash-render/src/experimental/arboretum.rs
sed -i 's/return Err("L-system exceeded maximum stack depth"\.to_string());/return Err(crate::experimental::error::Error::CapacityExceeded("L-system exceeded maximum stack depth"));/g' crates/abrash-render/src/experimental/arboretum.rs

# Steganography
sed -i 's/Result<(), &\x27static str>/Result<(), crate::experimental::error::Error>/g' crates/abrash-render/src/experimental/steganography.rs
sed -i 's/\.ok_or("Message too large")/.ok_or(crate::experimental::error::Error::CapacityExceeded("Message too large"))/g' crates/abrash-render/src/experimental/steganography.rs
sed -i 's/Err("Framebuffer too small to hold the message")/Err(crate::experimental::error::Error::CapacityExceeded("Framebuffer too small to hold the message"))/g' crates/abrash-render/src/experimental/steganography.rs

# Jelly
sed -i 's/Result<(), String>/Result<(), crate::experimental::error::Error>/g' crates/abrash-render/src/experimental/jelly.rs
sed -i 's/Result<Self, String>/Result<Self, crate::experimental::error::Error>/g' crates/abrash-render/src/experimental/jelly.rs
sed -i 's/Err("Mass must be positive"\.to_string())/Err(crate::experimental::error::Error::InvalidInput("Mass must be positive"))/g' crates/abrash-render/src/experimental/jelly.rs

# TUI
sed -i 's/use abrash_render::platform::tui::TuiWindow;/use crate::platform::tui::TuiWindow;/g' src/platform/tui.rs
