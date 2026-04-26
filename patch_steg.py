import re

with open("crates/abrash-render/src/experimental/steganography.rs", "r") as f:
    content = f.read()

# Replace the fake tests
old_tests = """    #[test]
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
    }"""

# Wait, `channel_idx` is calculated as `bit_idx % 3`. It is impossible for it to be 3.
# The tests currently bypass the logic. Sentry journal says:
# "When a guard is "unreachable" via public methods, isolate the specific `match` or state check block into the test body and manually force the illegal value to verify the `unreachable!()` panic executes with the correct message."
# It actually looks like these tests ARE complying with Sentry's journal about isolating the specific match or state check block into the test body.
