/// Swap bytes of an integer for big-endian MIDI format.
///
/// Port of `swap_int()` from `ChordNova/src/include/utility/midi_encoding.h`.
///
/// # Arguments
/// * `value` – The integer to byte-swap.
/// * `len`   – Number of bytes (1–4).
pub const fn swap_int(value: i32, len: u8) -> i32 {
    match len {
        1 => value,
        2 => ((value & 0x00FF) << 8) | ((value & 0xFF00u32 as i32) >> 8),
        3 => ((value & 0x0000FF) << 16)
            | (value & 0x00FF00)
            | ((value & 0xFF0000u32 as i32) >> 16),
        4 => {
            let u = value as u32;
            (((u & 0x000000FF) << 24)
                | ((u & 0x0000FF00) << 8)
                | ((u & 0x00FF0000) >> 8)
                | ((u & 0xFF000000) >> 24)) as i32
        }
        _ => value,
    }
}

/// Encode a non-negative integer as MIDI Variable-Length Quantity (VLQ).
///
/// Each byte uses 7 data bits; bit 7 is set on all bytes except the last.
/// Returns 1–4 bytes, MSB first.
///
/// Port of `to_vlq()` from `ChordNova/src/include/utility/midi_encoding.h`.
pub fn to_vlq(value: u32) -> Vec<u8> {
    if value == 0 {
        return vec![0];
    }

    let mut bytes: Vec<u8> = Vec::new();
    let mut v = value;
    while v > 0 {
        bytes.push((v & 0x7F) as u8);
        v >>= 7;
    }
    // Set continuation bit on all bytes except the lowest-significance (index 0)
    let len = bytes.len();
    for i in 1..len {
        bytes[i] |= 0x80;
    }
    bytes.reverse();
    bytes
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn swap_int_identity_len1() {
        assert_eq!(swap_int(0x12, 1), 0x12);
        assert_eq!(swap_int(0xFF, 1), 0xFF);
    }

    #[test]
    fn swap_int_len2() {
        assert_eq!(swap_int(0x0102, 2), 0x0201);
        assert_eq!(swap_int(0x00FF, 2), 0xFF00u32 as i32);
    }

    #[test]
    fn swap_int_len4() {
        assert_eq!(swap_int(0x01020304, 4), 0x04030201);
        assert_eq!(swap_int(0x00000001, 4), 0x01000000u32 as i32);
    }

    #[test]
    fn vlq_zero() {
        assert_eq!(to_vlq(0), vec![0x00]);
    }

    #[test]
    fn vlq_single_byte() {
        // Values 0–127 encode as one byte
        assert_eq!(to_vlq(1), vec![0x01]);
        assert_eq!(to_vlq(127), vec![0x7F]);
    }

    #[test]
    fn vlq_two_bytes() {
        // 128 = 0x80 → [0x81, 0x00]
        assert_eq!(to_vlq(128), vec![0x81, 0x00]);
        // 255 = 0xFF → [0x81, 0x7F]
        assert_eq!(to_vlq(255), vec![0x81, 0x7F]);
    }

    #[test]
    fn vlq_large_value() {
        // 0x100000 (1048576) → [0xC0, 0x80, 0x00] (3 bytes)
        assert_eq!(to_vlq(0x100000), vec![0xC0, 0x80, 0x00]);
    }
}
