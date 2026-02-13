/*!
 * Defines the layout in bits of a signal so that it can be reused to pack/unpack into bytes
 */

use crate::canlog_reader::CanFrame;

/// One contiguous span of bits within a single byte of the CAN frame data.
///
/// Describes a mapping: "take `num_bits` consecutive bits starting at
/// `bit_offset` in `data[byte_index]`, and place them at `value_shift`
/// in the raw u64 value."
#[derive(Debug, Clone, Copy)]
pub struct BitSpan {
    pub byte_index: usize,
    /// Lowest bit position within the byte (0..=7).
    pub bit_offset: u8,
    /// How many consecutive bits in this span (1..=8).
    pub num_bits: u8,
    /// Where these bits land in the raw u64, LSB-relative.
    /// i.e., the extracted bits are shifted left by this amount.
    pub value_shift: u8,
}

/// Precomputed mapping from a DBC signal's bit positions to frame data bytes.
///
/// Built once from a `can_dbc::Signal` spec via `from_spec()`. The same layout
/// is used by both `extract` (decode) and `pack` (encode), guaranteeing they
/// are inverses by construction.
#[derive(Debug)]
pub struct SignalLayout {
    /// Each segment describes one contiguous run of bits within a single byte.
    /// A 64-bit signal spanning all 8 bytes produces at most 9 segments
    /// (one partial + 8 full, or similar).
    pub segments: Vec<BitSpan>,
    pub signal_size: u64,
}

impl SignalLayout {
    /// Build a layout from a DBC signal specification.
    ///
    /// This is the single source of truth for how DBC start_bit + byte_order
    /// maps to physical byte/bit positions in the CAN frame data array.
    pub fn from_spec(spec: &can_dbc::Signal) -> Self {
        let mut segments = Vec::new();
        let mut byte_index = (spec.start_bit / 8) as usize;
        let mut bit_index = (spec.start_bit % 8) as u8;
        let mut remaining = spec.signal_size;

        match spec.byte_order() {
            can_dbc::ByteOrder::BigEndian => {
                // Big-endian (Motorola): start_bit is the MSB position.
                // Walk downward within each byte, then move to next byte at bit 7.
                // First bits extracted are the MSB of the raw value.
                while remaining > 0 {
                    let num_bits = std::cmp::min(bit_index as u64 + 1, remaining) as u8;
                    let bit_offset = bit_index + 1 - num_bits;
                    remaining -= num_bits as u64;
                    segments.push(BitSpan {
                        byte_index,
                        bit_offset,
                        num_bits,
                        value_shift: remaining as u8,
                    });
                    byte_index += 1;
                    bit_index = 7;
                }
            }
            can_dbc::ByteOrder::LittleEndian => {
                // Little-endian (Intel): start_bit is the LSB position.
                // Walk upward within each byte, then move to next byte at bit 0.
                // First bits extracted are the LSB of the raw value.
                let mut value_shift: u64 = 0;
                while remaining > 0 {
                    let num_bits = std::cmp::min(8 - bit_index as u64, remaining) as u8;
                    segments.push(BitSpan {
                        byte_index,
                        bit_offset: bit_index,
                        num_bits,
                        value_shift: value_shift as u8,
                    });
                    value_shift += num_bits as u64;
                    remaining -= num_bits as u64;
                    byte_index += 1;
                    bit_index = 0;
                }
            }
        }

        Self {
            segments,
            signal_size: spec.signal_size,
        }
    }

    /// Extract the raw unsigned value from the CAN frame data bytes.
    ///
    /// Iterates over the precomputed segments, masking and shifting bits
    /// from each byte into the correct position in the result.
    pub fn extract(&self, data: &[u8; 64]) -> u64 {
        let mut result: u64 = 0;
        for span in &self.segments {
            let mask = ((1u16 << span.num_bits) - 1) as u8;
            let bits = (data[span.byte_index] >> span.bit_offset) & mask;
            result |= (bits as u64) << span.value_shift;
        }
        result
    }

    /// Pack a raw unsigned value into the CAN frame data bytes.
    ///
    /// Iterates over the precomputed segments, slicing bits from the raw value
    /// and writing them into the correct byte positions. Clears target bits
    /// before writing so that multiple signals can be packed into the same frame.
    pub fn pack(&self, data: &mut [u8; 64], raw: u64) {
        for span in &self.segments {
            let mask = ((1u16 << span.num_bits) - 1) as u8;
            let bits = ((raw >> span.value_shift) as u8) & mask;
            data[span.byte_index] &= !(mask << span.bit_offset);
            data[span.byte_index] |= bits << span.bit_offset;
        }
    }

    /// Decode a signal from a CAN frame, returning the physical value.
    ///
    /// Extracts the raw value via the layout, applies sign extension if needed,
    /// then computes: physical = raw * factor + offset.
    pub fn decode(&self, frame: &CanFrame, spec: &can_dbc::Signal) -> f64 {
        let raw = self.extract(&frame.data);
        let final_value = match spec.value_type() {
            can_dbc::ValueType::Signed => {
                let shift_len = 64 - spec.signal_size;
                let sign_extended = ((raw as i64) << shift_len) >> shift_len;
                sign_extended as f64
            }
            can_dbc::ValueType::Unsigned => raw as f64,
        };
        final_value * spec.factor() + spec.offset()
    }
}

#[cfg(test)]
mod tests {
    use std::arch::x86_64;

    use super::*;
    use crate::can_decoder::{self, get_message_spec};
    use crate::can_encoder::encode_message;
    use crate::canlog_reader;

    #[test]
    fn test_extract_motohawk_temperature() {
        // Temperature: start_bit=0, size=12, big-endian, signed, factor=0.01, offset=250
        // Frame: A5B6D90000000000
        // Golden value from cantools: 244.14 degK
        // raw * 0.01 + 250 = 244.14 -> raw = (244.14 - 250)/0.01 = -586
        // -586 as signed 12-bit two's complement: 4096 - 586 = 3510 = 0xDB6
        let line = "(0.0) vcan0 1F0#A5B6D90000000000";
        let frame = canlog_reader::parse_candump_line(line).unwrap();
        let dbc = can_decoder::load_dbc("motohawk.dbc").unwrap();
        let msg = can_decoder::get_message_spec(&dbc, "ExampleMessage").unwrap();

        let signal = can_decoder::get_signal_spec(&msg, "Temperature").unwrap();
        let layout = SignalLayout::from_spec(signal);
        let raw = layout.extract(&frame.data);
        // raw should be 3510 (unsigned representation of -586 in 12 bits)
        assert_eq!(raw, 0xDB6);

        // Full decode should match the golden value
        let decoded = layout.decode(&frame, signal);
        assert_eq!(decoded, 244.14);
    }

    #[test]
    fn test_extract_motohawk_average_radius() {
        // AverageRadius: start_bit=6, size=6, big-endian, unsigned, factor=0.1, offset=0
        // Golden value from cantools: 1.8 m -> raw = 1.8 / 0.1 = 18
        let line = "(0.0) vcan0 1F0#A5B6D90000000000";
        let frame = canlog_reader::parse_candump_line(line).unwrap();
        let dbc = can_decoder::load_dbc("motohawk.dbc").unwrap();
        let msg = can_decoder::get_message_spec(&dbc, "ExampleMessage").unwrap();

        let signal = can_decoder::get_signal_spec(&msg, "AverageRadius").unwrap();
        let layout = SignalLayout::from_spec(signal);
        let raw = layout.extract(&frame.data);
        assert_eq!(raw, 18);

        let decoded = layout.decode(&frame, signal);
        assert!((decoded - 1.8).abs() < 1e-10);
    }

    #[test]
    fn test_extract_motohawk_enable() {
        // Enable: start_bit=7, size=1, big-endian, unsigned, factor=1, offset=0
        // Golden value from cantools: 1 (Enabled)
        let line = "(0.0) vcan0 1F0#A5B6D90000000000";
        let frame = canlog_reader::parse_candump_line(line).unwrap();
        let dbc = can_decoder::load_dbc("motohawk.dbc").unwrap();
        let msg = can_decoder::get_message_spec(&dbc, "ExampleMessage").unwrap();

        let signal = can_decoder::get_signal_spec(&msg, "Enable").unwrap();
        let layout = SignalLayout::from_spec(signal);
        let raw = layout.extract(&frame.data);
        assert_eq!(raw, 1);
    }

    #[test]
    fn test_extract_signed_signals() {
        // Frame: 11223344FF667788
        // Golden values from cantools:
        //   s3big (BE, 3-bit signed): -1
        //   s3    (LE, 3-bit signed): -1
        //   s7    (LE, 7-bit signed): 8
        //   s7big (BE, 7-bit signed): 8
        //   s8big (BE, 8-bit signed): -111
        //   s8    (LE, 8-bit signed): -47
        //   s9    (LE, 9-bit signed): 25
        //   s10big(BE, 10-bit signed): 239
        let line = "(0.0) vcan0 00A#11223344FF667788";
        let frame = canlog_reader::parse_candump_line(line).unwrap();
        let dbc = can_decoder::load_dbc("signed.dbc").unwrap();
        let msg = can_decoder::get_message_spec(&dbc, "Message378910").unwrap();

        let cases: &[(&str, f64)] = &[
            ("s3big", -1.0),
            ("s3", -1.0),
            ("s7", 8.0),
            ("s7big", 8.0),
            ("s8big", -111.0),
            ("s8", -47.0),
            ("s9", 25.0),
            ("s10big", 239.0),
        ];

        for (signal_name, expected) in cases {
            let signal = can_decoder::get_signal_spec(&msg, signal_name).unwrap();
            let layout = SignalLayout::from_spec(signal);
            let decoded = layout.decode(&frame, signal);
            assert_eq!(
                decoded, *expected,
                "signal {signal_name}: expected {expected}, got {decoded}"
            );
        }
    }

    #[test]
    fn test_extract_64bit_signals() {
        let line = "(0.0) vcan0 002#11223344FF667788";
        let frame = canlog_reader::parse_candump_line(line).unwrap();
        let dbc = can_decoder::load_dbc("signed.dbc").unwrap();

        // s64 (LE, 64-bit signed): -8613302515775888879
        let msg = can_decoder::get_message_spec(&dbc, "Message64").unwrap();
        let signal = can_decoder::get_signal_spec(&msg, "s64").unwrap();
        let layout = SignalLayout::from_spec(signal);
        let decoded = layout.decode(&frame, signal);
        assert_eq!(decoded, -8613302515775888879.0);

        // s64big (BE, 64-bit signed): -9223372036854775808
        // uses frame 8000000000000000
        let line_big = "(0.0) vcan0 003#8000000000000000";
        let frame_big = canlog_reader::parse_candump_line(line_big).unwrap();
        let msg_big = can_decoder::get_message_spec(&dbc, "Message64big").unwrap();
        let signal_big = can_decoder::get_signal_spec(&msg_big, "s64big").unwrap();
        let layout_big = SignalLayout::from_spec(signal_big);
        let decoded_big = layout_big.decode(&frame_big, signal_big);
        assert_eq!(decoded_big, -9223372036854775808.0);
    }

    /// Assert that SignalLayout::decode and decode_signal_by_bytes produce
    /// identical results for every signal in `msg` applied to `frame`.
    fn assert_layout_matches_decoder(
        frame: &CanFrame,
        msg: &can_dbc::Message,
    ) {
        for signal in msg.signals() {
            let layout = SignalLayout::from_spec(signal);
            let layout_value = layout.decode(frame, signal);
            let existing_value = can_decoder::decode_signal_by_bytes(frame, signal);
            assert_eq!(
                layout_value, existing_value,
                "mismatch at t={} id=0x{:X} message={} signal={}: layout={}, existing={}",
                frame.timestamp,
                frame.id,
                msg.message_name(),
                signal.name(),
                layout_value,
                existing_value,
            );
        }
    }

    #[test]
    fn test_layout_matches_decode_signal_by_bytes_motohawk() {
        let line = "(0.0) vcan0 1F0#A5B6D90000000000";
        let frame = canlog_reader::parse_candump_line(line).unwrap();
        let dbc = can_decoder::load_dbc("motohawk.dbc").unwrap();
        let msg = can_decoder::get_message_spec(&dbc, "ExampleMessage").unwrap();
        assert_layout_matches_decoder(&frame, msg);
    }

    #[test]
    fn test_layout_matches_decode_signal_by_bytes_signed() {
        let dbc = can_decoder::load_dbc("signed.dbc").unwrap();

        let frames_and_messages: &[(&str, &str)] = &[
            ("(0.0) vcan0 00A#11223344FF667788", "Message378910"),
            ("(0.0) vcan0 002#11223344FF667788", "Message64"),
            ("(0.0) vcan0 003#8000000000000000", "Message64big"),
            ("(0.0) vcan0 000#11223344FF667788", "Message32"),
            ("(0.0) vcan0 005#11223344FF667788", "Message32big"),
            ("(0.0) vcan0 001#11223344FF667788", "Message33"),
            ("(0.0) vcan0 004#11223344FF667788", "Message33big"),
            ("(0.0) vcan0 006#11223344FF667788", "Message63"),
            ("(0.0) vcan0 007#11223344FF667788", "Message63big"),
        ];

        for (line, message_name) in frames_and_messages {
            let frame = canlog_reader::parse_candump_line(line).unwrap();
            let msg = match can_decoder::get_message_spec(&dbc, message_name) {
                Some(m) => m,
                None => continue,
            };
            assert_layout_matches_decoder(&frame, msg);
        }
    }

    /// Compare SignalLayout::decode against decode_signal_by_bytes for every
    /// frame in the Nissan Leaf candump log, across all matching DBC messages.
    /// This is a real-world validation across 147k frames and 18 message types.
    #[test]
    fn test_layout_vs_decode_nissan_leaf_full_log() {
        use std::collections::HashMap;
        use crate::canlog_reader::CanLogParser;
        use std::path::Path;

        let dbc_path = "can_samples/aphryx-canx-nissan-leaf/nissan_leaf_2018.dbc";
        let log_path = "can_samples/aphryx-canx-nissan-leaf/nissan_leaf_candump.log";

        let dbc = can_decoder::load_dbc(dbc_path).unwrap();

        let msg_by_id: HashMap<u32, &can_dbc::Message> = dbc
            .messages()
            .iter()
            .map(|m| (m.message_id().raw(), m))
            .collect();

        let parser = CanLogParser::from_file(Path::new(log_path)).unwrap();
        let mut frames_checked: u64 = 0;

        for frame in parser {
            let msg = match msg_by_id.get(&frame.id) {
                Some(m) => m,
                None => continue,
            };
            assert_layout_matches_decoder(&frame, msg);
            frames_checked += 1;
        }

        assert!(frames_checked > 0, "no frames matched any DBC message");
    }

    // ---------------------------------------------------------------
    // Pack tests
    // ---------------------------------------------------------------

    #[test]
    fn test_pack_motohawk_golden_bytes() {
        // Pack all three motohawk signals into a zeroed frame and verify
        // the resulting bytes match the expected encoding.
        //
        // Temperature: raw=3510 (0xDB6), 12-bit BE, start_bit=0
        //   byte 0 bit 0 = MSB(1)           → 0x01
        //   byte 1 bits 7..0 = 0xB6          → 0xB6
        //   byte 2 bits 7..5 = 0b110          → 0xC0
        //
        // AverageRadius: raw=18, 6-bit BE, start_bit=6
        //   byte 0 bits 6..1 = 18 = 0b010010 → 0x24
        //
        // Enable: raw=1, 1-bit BE, start_bit=7
        //   byte 0 bit 7 = 1                  → 0x80
        //
        // Combined byte 0 = 0x01 | 0x24 | 0x80 = 0xA5
        let dbc = can_decoder::load_dbc("motohawk.dbc").unwrap();
        let msg = can_decoder::get_message_spec(&dbc, "ExampleMessage").unwrap();

        let mut data = [0u8; 64];

        let temp = can_decoder::get_signal_spec(&msg, "Temperature").unwrap();
        SignalLayout::from_spec(temp).pack(&mut data, 0xDB6);

        let radius = can_decoder::get_signal_spec(&msg, "AverageRadius").unwrap();
        SignalLayout::from_spec(radius).pack(&mut data, 18);

        let enable = can_decoder::get_signal_spec(&msg, "Enable").unwrap();
        SignalLayout::from_spec(enable).pack(&mut data, 1);

        assert_eq!(data[0], 0xA5, "byte 0");
        assert_eq!(data[1], 0xB6, "byte 1");
        assert_eq!(data[2], 0xC0, "byte 2"); // only bits 5-7 used
        assert_eq!(data[3], 0x00, "byte 3");
    }

    #[test]
    fn test_pack_signed_signals_golden_bytes() {
        // Pack s64 (LE, 64-bit signed) raw value into zeroed frame.
        //
        // Frame: 11223344FF667788
        // s64: start_bit=0, 64 bits, LE. extract gives all 64 bits as-is in LE order.
        // raw = 0x887766FF44332211
        let line = "(0.0) vcan0 002#11223344FF667788";
        let frame = canlog_reader::parse_candump_line(line).unwrap();
        let dbc = can_decoder::load_dbc("signed.dbc").unwrap();
        let msg = can_decoder::get_message_spec(&dbc, "Message64").unwrap();
        let signal = can_decoder::get_signal_spec(&msg, "s64").unwrap();
        let layout = SignalLayout::from_spec(signal);

        let raw = layout.extract(&frame.data);
        assert_eq!(0x887766FF44332211,raw);

        // Pack into zeroed data and verify we get the original bytes back
        let mut data = [0u8; 64];
        layout.pack(&mut data, raw);
        assert_eq!(&data[..8], &frame.data[..8]);
    }

    #[test]
    fn test_pack_s64big_golden_bytes() {
        // s64big: start_bit=7, 64-bit BE signed
        // Frame: 8000000000000000
        let line = "(0.0) vcan0 003#8000000000000000";
        let frame = canlog_reader::parse_candump_line(line).unwrap();
        let dbc = can_decoder::load_dbc("signed.dbc").unwrap();
        let msg = can_decoder::get_message_spec(&dbc, "Message64big").unwrap();
        let signal = can_decoder::get_signal_spec(&msg, "s64big").unwrap();
        let layout = SignalLayout::from_spec(signal);

        let raw = layout.extract(&frame.data);
        assert_eq!(0x8000000000000000,raw);

        let mut data = [0u8; 64];
        layout.pack(&mut data, raw);
        assert_eq!(&data[..8], &frame.data[..8]);
    }

    

    #[test]
    fn test_pack_clears_existing_bits() {
        // Verify that pack clears the target bits before writing.
        // Start with all-0xFF frame, pack a 0 value, check bits are cleared.
        let dbc = can_decoder::load_dbc("motohawk.dbc").unwrap();
        let msg = can_decoder::get_message_spec(&dbc, "ExampleMessage").unwrap();
        let signal = can_decoder::get_signal_spec(&msg, "Enable").unwrap();
        let layout = SignalLayout::from_spec(signal);

        let mut data = [0xFFu8; 64];
        layout.pack(&mut data, 0);

        // Enable is bit 7 of byte 0. Packing 0 should clear it.
        assert_eq!(data[0], 0x7F); // bit 7 cleared, rest untouched
        assert_eq!(data[1], 0xFF); // other bytes untouched
    }

    // ---------------------------------------------------------------
    // Round-trip tests: extract → pack → extract
    // ---------------------------------------------------------------

    #[test]
    fn test_roundtrip_extract_pack_motohawk() {
        // For each motohawk signal: extract raw from golden frame,
        // pack into zeroed data, extract again → must match.
        let line = "(0.0) vcan0 1F0#A5B6D90000000000";
        let frame = canlog_reader::parse_candump_line(line).unwrap();
        let dbc = can_decoder::load_dbc("motohawk.dbc").unwrap();
        let msg = can_decoder::get_message_spec(&dbc, "ExampleMessage").unwrap();

        for signal in msg.signals() {
            let layout = SignalLayout::from_spec(signal);
            let raw = layout.extract(&frame.data);
            let mut data = [0u8; 64];
            layout.pack(&mut data, raw);
            let raw2 = layout.extract(&data);
            assert_eq!(
                raw, raw2,
                "extract-pack roundtrip failed for signal '{}': {} != {}",
                signal.name(), raw, raw2
            );
        }
    }

    #[test]
    fn test_roundtrip_extract_pack_signed() {
        // Round-trip every signal across all signed.dbc messages.
        let dbc = can_decoder::load_dbc("signed.dbc").unwrap();

        let frames_and_messages: &[(&str, &str)] = &[
            ("(0.0) vcan0 00A#11223344FF667788", "Message378910"),
            ("(0.0) vcan0 002#11223344FF667788", "Message64"),
            ("(0.0) vcan0 003#8000000000000000", "Message64big"),
            ("(0.0) vcan0 000#11223344FF667788", "Message32"),
            ("(0.0) vcan0 005#11223344FF667788", "Message32big"),
            ("(0.0) vcan0 001#11223344FF667788", "Message33"),
            ("(0.0) vcan0 004#11223344FF667788", "Message33big"),
            ("(0.0) vcan0 006#11223344FF667788", "Message63"),
            ("(0.0) vcan0 007#11223344FF667788", "Message63big"),
        ];

        for (line, message_name) in frames_and_messages {
            let frame = canlog_reader::parse_candump_line(line).unwrap();
            let msg = match can_decoder::get_message_spec(&dbc, message_name) {
                Some(m) => m,
                None => continue,
            };
            for signal in msg.signals() {
                let layout = SignalLayout::from_spec(signal);
                let raw = layout.extract(&frame.data);
                let mut data = [0u8; 64];
                layout.pack(&mut data, raw);
                let raw2 = layout.extract(&data);
                assert_eq!(
                    raw, raw2,
                    "extract-pack roundtrip failed for {}.{}: {} != {}",
                    message_name, signal.name(), raw, raw2
                );
            }
        }
    }

    // ---------------------------------------------------------------
    // Round-trip tests: pack → decode_signal_by_bytes
    // ---------------------------------------------------------------

    #[test]
    fn test_pack_then_decode_signal_by_bytes_motohawk() {
        // Pack known raw values, then decode with the existing decoder.
        // Verify the physical value matches.
        //
        // Temperature: raw=3510 → physical = 3510 (as signed 12-bit = -586) * 0.01 + 250 = 244.14
        // AverageRadius: raw=18 → physical = 18 * 0.1 + 0 = 1.8
        // Enable: raw=1 → physical = 1 * 1 + 0 = 1.0
        let dbc = can_decoder::load_dbc("motohawk.dbc").unwrap();
        let msg = can_decoder::get_message_spec(&dbc, "ExampleMessage").unwrap();

        let cases: &[(&str, u64, f64)] = &[
            ("Temperature", 0xDB6, 244.14), //signed 12 bit 0xDB6 = -586
            ("AverageRadius", 18, 1.8),
            ("Enable", 1, 1.0),
        ];

        for (signal_name, raw, expected_physical) in cases {
            let signal = can_decoder::get_signal_spec(&msg, signal_name).unwrap();
            let layout = SignalLayout::from_spec(signal);

            let mut frame: CanFrame = Default::default();
            layout.pack(&mut frame.data, *raw);

            let decoded = can_decoder::decode_signal_by_bytes(&frame, signal);
            assert_eq!(
                decoded, *expected_physical,
                "pack+decode failed for '{}': packed raw={}, decoded={}, expected={}",
                signal_name, raw, decoded, expected_physical
            );
        }
    }

    #[test]
    fn test_pack_then_decode_signal_by_bytes_signed() {
        // For signed.dbc signals: extract raw from golden frame, pack into
        // fresh frame, decode with existing decoder, compare against golden physical value.
        let dbc = can_decoder::load_dbc("signed.dbc").unwrap();

        // (candump line, message name, signal name, expected physical value)
        let cases: &[(&str, &str, &str, f64)] = &[
            ("(0.0) vcan0 00A#11223344FF667788", "Message378910", "s3big", -1.0),
            ("(0.0) vcan0 00A#11223344FF667788", "Message378910", "s3", -1.0),
            ("(0.0) vcan0 00A#11223344FF667788", "Message378910", "s7", 8.0),
            ("(0.0) vcan0 00A#11223344FF667788", "Message378910", "s7big", 8.0),
            ("(0.0) vcan0 00A#11223344FF667788", "Message378910", "s8big", -111.0),
            ("(0.0) vcan0 00A#11223344FF667788", "Message378910", "s8", -47.0),
            ("(0.0) vcan0 00A#11223344FF667788", "Message378910", "s9", 25.0),
            ("(0.0) vcan0 00A#11223344FF667788", "Message378910", "s10big", 239.0),
            ("(0.0) vcan0 002#11223344FF667788", "Message64", "s64", -8613302515775888879.0),
            ("(0.0) vcan0 003#8000000000000000", "Message64big", "s64big", -9223372036854775808.0),
        ];

        for (line, msg_name, signal_name, expected) in cases {
            let src_frame = canlog_reader::parse_candump_line(line).unwrap();
            let msg = can_decoder::get_message_spec(&dbc, msg_name).unwrap();
            let signal = can_decoder::get_signal_spec(&msg, signal_name).unwrap();
            let layout = SignalLayout::from_spec(signal);

            // Extract raw from golden frame, pack into fresh frame
            let raw = layout.extract(&src_frame.data);
            let mut frame: CanFrame = Default::default();
            layout.pack(&mut frame.data, raw);

            // Decode with existing decoder
            let decoded = can_decoder::decode_signal_by_bytes(&frame, signal);
            assert_eq!(
                decoded, *expected,
                "pack+decode failed for {}.{}: raw={}, decoded={}, expected={}",
                msg_name, signal_name, raw, decoded, expected
            );
        }
    }

    #[test]
    fn test_roundtrip_extract_pack_nissan_leaf() {
        // Round-trip every signal in every matching frame from the Nissan Leaf log.
        use std::collections::HashMap;
        use crate::canlog_reader::CanLogParser;
        use std::path::Path;

        let dbc = can_decoder::load_dbc(
            "can_samples/aphryx-canx-nissan-leaf/nissan_leaf_2018.dbc",
        ).unwrap();

        let msg_by_id: HashMap<u32, &can_dbc::Message> = dbc
            .messages()
            .iter()
            .map(|m| (m.message_id().raw(), m))
            .collect();

        let parser = CanLogParser::from_file(Path::new(
            "can_samples/aphryx-canx-nissan-leaf/nissan_leaf_candump.log",
        )).unwrap();
        let mut signals_checked: u64 = 0;

        for frame in parser {
            let msg = match msg_by_id.get(&frame.id) {
                Some(m) => m,
                None => continue,
            };
            for signal in msg.signals() {
                let layout = SignalLayout::from_spec(signal);
                let raw = layout.extract(&frame.data);
                let mut data = [0u8; 64];
                layout.pack(&mut data, raw);
                let raw2 = layout.extract(&data);
                assert_eq!(
                    raw, raw2,
                    "extract-pack roundtrip failed at t={} id=0x{:X} {}.{}: {} != {}",
                    frame.timestamp, frame.id,
                    msg.message_name(), signal.name(), raw, raw2
                );
                signals_checked += 1;
            }
        }

        assert!(signals_checked > 0, "no signals were checked");
    }
}
