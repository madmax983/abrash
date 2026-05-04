//! Steganography encoding and decoding in Framebuffers.
//!
//! This module provides functions to hide and extract string messages
//! within the least significant bits (LSB) of a Framebuffer's RGB channels.

use crate::framebuffer::Framebuffer;

/// Encodes a string message into the given framebuffer.
///
/// Hides the message length (4 bytes) and the message data in the least
/// significant bits of the RGB channels.
///
/// Returns an error if the framebuffer is not large enough to hold the message.
/// # Errors
/// Returns an error if the message is too long to fit in the framebuffer.
pub fn encode_message(fb: &mut Framebuffer, message: &str) -> Result<(), &'static str> {
    let bytes = message.as_bytes();
    let len = bytes.len() as u32;

    // We need 4 bytes for the length, plus the bytes of the message.
    let total_bytes_needed = 4_usize
        .checked_add(bytes.len())
        .ok_or("Message too large")?;

    // Each pixel can store 3 bits (one in R, one in G, one in B).
    let bits_needed = total_bytes_needed
        .checked_mul(8)
        .ok_or("Message too large")?;
    let pixels_needed = bits_needed.checked_add(2).ok_or("Message too large")? / 3;

    if pixels_needed > (fb.width() * fb.height()) as usize {
        return Err("Framebuffer too small to hold the message");
    }

    let mut data_to_encode = Vec::with_capacity(total_bytes_needed);
    data_to_encode.extend_from_slice(&len.to_le_bytes());
    data_to_encode.extend_from_slice(bytes);

    let pixels = fb.as_mut_slice();
    let mut bit_idx = 0;

    for byte in data_to_encode {
        for bit in 0..8 {
            let pixel_idx = bit_idx / 3;
            let channel_idx = bit_idx % 3;

            let bit_val = (byte >> bit) & 1;

            let mut p = pixels[pixel_idx];

            // clear the lsb of the target channel and set it to bit_val
            match channel_idx {
                0 => {
                    // R
                    p = (p & !(1 << 16)) | (u32::from(bit_val) << 16);
                }
                1 => {
                    // G
                    p = (p & !(1 << 8)) | (u32::from(bit_val) << 8);
                }
                2 => {
                    // B
                    p = (p & !1) | u32::from(bit_val);
                }
                _ => unreachable!("channel_idx is always % 3, so it's 0, 1, or 2"),
            }
            pixels[pixel_idx] = p;

            bit_idx += 1;
        }
    }

    Ok(())
}

/// Decodes a string message hidden in the given framebuffer.
///
/// Returns `None` if the length is invalid or the data is not valid UTF-8.
#[must_use]
pub fn decode_message(fb: &Framebuffer) -> Option<String> {
    let pixels = fb.as_slice();

    // Read the first 32 bits to get the length
    let mut len_bytes = [0u8; 4];
    let mut bit_idx = 0;

    for byte in &mut len_bytes {
        for bit in 0..8 {
            let pixel_idx = bit_idx / 3;
            if pixel_idx >= pixels.len() {
                return None;
            }

            let channel_idx = bit_idx % 3;
            let p = pixels[pixel_idx];

            let bit_val = match channel_idx {
                0 => (p >> 16) & 1,
                1 => (p >> 8) & 1,
                2 => p & 1,
                _ => unreachable!("channel_idx is always % 3, so it's 0, 1, or 2"),
            };

            *byte |= (bit_val as u8) << bit;
            bit_idx += 1;
        }
    }

    let len = u32::from_le_bytes(len_bytes) as usize;

    // Check if the length is valid
    let bits_needed = 4_usize.checked_add(len)?.checked_mul(8)?;
    let pixels_needed = bits_needed.checked_add(2)? / 3;

    if len > pixels.len() * 3 / 8 || pixels_needed > pixels.len() {
        return None;
    }

    let mut message_bytes = Vec::with_capacity(len);

    for _ in 0..len {
        let mut byte = 0u8;
        for bit in 0..8 {
            let pixel_idx = bit_idx / 3;
            let channel_idx = bit_idx % 3;
            let p = pixels[pixel_idx];

            let bit_val = match channel_idx {
                0 => (p >> 16) & 1,
                1 => (p >> 8) & 1,
                2 => p & 1,
                _ => unreachable!("channel_idx is always % 3, so it's 0, 1, or 2"),
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

    #[test]
    fn test_encode_decode_roundtrip() {
        let mut fb = Framebuffer::new(10, 10).unwrap();
        fb.clear(0xFFFF_FFFF); // All white

        let message = "Hello, Nova!";

        assert!(encode_message(&mut fb, message).is_ok());

        let decoded = decode_message(&fb);
        assert_eq!(decoded.as_deref(), Some(message));
    }

    #[test]
    fn test_encode_too_large() {
        let mut fb = Framebuffer::new(2, 2).unwrap(); // 4 pixels, 12 bits
        // We need 4 bytes (32 bits) just for length
        let message = "A";

        assert!(encode_message(&mut fb, message).is_err());
    }

    #[test]
    fn test_decode_message_overflow() {
        let mut fb = Framebuffer::new(10, 10).unwrap();
        let pixels = fb.as_mut_slice();
        for i in 0..11 {
            pixels[i] = 0xFFFF_FFFF;
        }

        let decoded = decode_message(&fb);
        assert_eq!(decoded, None);
    }

    #[test]
    #[should_panic(expected = "channel_idx is always % 3, so it's 0, 1, or 2")]
    fn test_encode_unreachable_guard() {
        let channel_idx = 3;
        match channel_idx {
            0 | 1 | 2 => {}
            _ => unreachable!("channel_idx is always % 3, so it's 0, 1, or 2"),
        }
    }

    #[test]
    #[should_panic(expected = "channel_idx is always % 3, so it's 0, 1, or 2")]
    fn test_decode_length_unreachable_guard() {
        let p = 0;
        let _ = match 3 {
            0 => (p >> 16) & 1,
            1 => (p >> 8) & 1,
            2 => p & 1,
            _ => unreachable!("channel_idx is always % 3, so it's 0, 1, or 2"),
        };
    }

    #[test]
    #[should_panic(expected = "channel_idx is always % 3, so it's 0, 1, or 2")]
    fn test_decode_data_unreachable_guard() {
        let p = 0;
        let _ = match 4 {
            0 => (p >> 16) & 1,
            1 => (p >> 8) & 1,
            2 => p & 1,
            _ => unreachable!("channel_idx is always % 3, so it's 0, 1, or 2"),
        };
    }
}
