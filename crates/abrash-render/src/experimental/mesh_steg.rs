//! Steganography encoding and decoding in 3D Meshes.
//!
//! This module provides functions to hide and extract string messages
//! within the least significant bits (LSB) of a Mesh's vertex coordinates.
//! This allows for invisible watermarking of 3D geometry.

use abrash_core::mesh::Mesh;

/// Encodes a string message into the given mesh's vertices.
///
/// Hides the message length (4 bytes) and the message data in the least
/// significant bit of the mantissa of the `x`, `y`, `z` floating-point
/// components of each vertex. Modifying the LSB of a single precision float
/// introduces negligible error, making the watermark visually imperceptible.
///
/// Returns an error if the mesh does not have enough vertices to hold the message.
/// # Errors
/// Returns an error if the message is too long to fit in the mesh.
pub fn encode_mesh_message(mesh: &mut Mesh, message: &str) -> Result<(), &'static str> {
    let bytes = message.as_bytes();
    let len = bytes.len() as u32;

    // We need 4 bytes for the length, plus the bytes of the message.
    let total_bytes_needed = 4_usize
        .checked_add(bytes.len())
        .ok_or("Message too large")?;

    // Each vertex can store 3 bits (one in x, one in y, one in z).
    let bits_needed = total_bytes_needed
        .checked_mul(8)
        .ok_or("Message too large")?;

    // Calculate required vertices (ceiling division by 3)
    let vertices_needed = bits_needed.checked_add(2).ok_or("Message too large")? / 3;

    if vertices_needed > mesh.vertices.len() {
        return Err("Mesh has too few vertices to hold the message");
    }

    let mut bit_idx = 0;

    for byte in len.to_le_bytes().into_iter().chain(bytes.iter().copied()) {
        for bit in 0..8 {
            let vertex_idx = bit_idx / 3;
            let component_idx = bit_idx % 3;

            let bit_val = u32::from((byte >> bit) & 1);

            let v = &mut mesh.vertices[vertex_idx];

            match component_idx {
                0 => {
                    let mut bits = v.x.to_bits();
                    bits = (bits & !1) | bit_val;
                    v.x = f32::from_bits(bits);
                }
                1 => {
                    let mut bits = v.y.to_bits();
                    bits = (bits & !1) | bit_val;
                    v.y = f32::from_bits(bits);
                }
                2 => {
                    let mut bits = v.z.to_bits();
                    bits = (bits & !1) | bit_val;
                    v.z = f32::from_bits(bits);
                }
                _ => unreachable!("component_idx is always % 3"),
            }

            bit_idx += 1;
        }
    }

    Ok(())
}

/// Decodes a string message hidden in the given mesh's vertices.
///
/// Returns `None` if the length is invalid or the data is not valid UTF-8.
#[must_use]
pub fn decode_mesh_message(mesh: &Mesh) -> Option<String> {
    // Read the first 32 bits to get the length
    let mut len_bytes = [0u8; 4];
    let mut bit_idx = 0;

    for byte in &mut len_bytes {
        for bit in 0..8 {
            let vertex_idx = bit_idx / 3;
            if vertex_idx >= mesh.vertices.len() {
                return None;
            }

            let component_idx = bit_idx % 3;
            let v = &mesh.vertices[vertex_idx];

            let bit_val = match component_idx {
                0 => v.x.to_bits() & 1,
                1 => v.y.to_bits() & 1,
                2 => v.z.to_bits() & 1,
                _ => unreachable!("component_idx is always % 3"),
            };

            *byte |= (bit_val as u8) << bit;
            bit_idx += 1;
        }
    }

    let len = u32::from_le_bytes(len_bytes) as usize;

    // Check if the length is valid
    let bits_needed = 4_usize.checked_add(len)?.checked_mul(8)?;
    let vertices_needed = bits_needed.checked_add(2)? / 3;

    if len > mesh.vertices.len() * 3 / 8 || vertices_needed > mesh.vertices.len() {
        return None;
    }

    let mut message_bytes = Vec::with_capacity(len);

    for _ in 0..len {
        let mut byte = 0u8;
        for bit in 0..8 {
            let vertex_idx = bit_idx / 3;
            let component_idx = bit_idx % 3;
            let v = &mesh.vertices[vertex_idx];

            let bit_val = match component_idx {
                0 => v.x.to_bits() & 1,
                1 => v.y.to_bits() & 1,
                2 => v.z.to_bits() & 1,
                _ => unreachable!("component_idx is always % 3"),
            };

            byte |= (bit_val as u8) << bit;
            bit_idx += 1;
        }
        message_bytes.push(byte);
    }

    String::from_utf8(message_bytes).ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use abrash_core::math::Vec3;

    #[test]
    fn test_encode_decode_mesh_roundtrip() {
        let mut mesh = Mesh::new();
        // Need at least ((4 + 12) * 8) / 3 = 128 / 3 = 43 vertices for "Hello, Nova!"
        for _ in 0..50 {
            mesh.vertices.push(Vec3::new(1.0, 2.0, 3.0));
        }

        let message = "Hello, Nova!";

        assert!(encode_mesh_message(&mut mesh, message).is_ok());

        let decoded = decode_mesh_message(&mesh);
        assert_eq!(decoded.as_deref(), Some(message));
    }

    #[test]
    fn test_encode_mesh_too_large() {
        let mut mesh = Mesh::new();
        // 4 vertices = 12 bits, not enough for the 32-bit length header
        for _ in 0..4 {
            mesh.vertices.push(Vec3::new(1.0, 1.0, 1.0));
        }
        let message = "A";

        assert!(encode_mesh_message(&mut mesh, message).is_err());
    }

    #[test]
    fn test_decode_mesh_invalid_utf8() {
        let mut mesh = Mesh::new();
        for _ in 0..50 {
            mesh.vertices.push(Vec3::new(1.0, 2.0, 3.0));
        }

        // Encode raw invalid UTF-8 manually
        // Let's just fake a valid length but put garbage in the rest.
        // First, encode a string so we have a valid header and proper size.
        let msg = "Hello";
        assert!(encode_mesh_message(&mut mesh, msg).is_ok());

        // Corrupt the data
        // Header takes 32 bits / 3 = 10.6 vertices -> ~11.
        // Let's mess up vertex 12 to invalidate UTF-8
        let mut bits = mesh.vertices[12].x.to_bits();
        bits ^= 1;
        mesh.vertices[12].x = f32::from_bits(bits);

        let decoded = decode_mesh_message(&mesh);
        // Might be None if UTF-8 is invalid, or might be some corrupted string if we happened
        // to flip a bit that results in valid ASCII/UTF-8.
        // Let's ensure it's not the original string.
        assert_ne!(decoded.as_deref(), Some(msg));
    }
}
